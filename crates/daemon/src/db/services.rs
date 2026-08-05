use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use shared::{Service, ServiceSpec, ServiceStatus};
use tracing::{debug, info};
use ulid::Ulid;

type ServiceRow = (
    String,         // id
    String,         // name
    String,         // project_id
    String,         // spec (JSON)
    String,         // status
    Option<String>, // live_container_id
    DateTime<Utc>,  // created_at
    DateTime<Utc>,  // updated_at
);

fn row_to_service(row: ServiceRow) -> Result<Service> {
    let (id, _name, _project_id, spec_json, status_str, live_container_id, created_at, updated_at) =
        row;
    let spec: ServiceSpec = serde_json::from_str(&spec_json)
        .map_err(|e| anyhow::anyhow!("falha ao deserializar spec do banco: {}", e))?;
    let status = parse_status(&status_str);
    Ok(Service {
        id,
        spec,
        status,
        live_container_id,
        created_at,
        updated_at,
    })
}

fn parse_status(s: &str) -> ServiceStatus {
    match s {
        "Stopped" => ServiceStatus::Stopped,
        "Stopping" => ServiceStatus::Stopping,
        "Deploying" => ServiceStatus::Deploying,
        "Queued" => ServiceStatus::Queued,
        "Running" => ServiceStatus::Running,
        "Degraded" => ServiceStatus::Degraded,
        s if s.starts_with("Error:") => {
            ServiceStatus::Error(s.trim_start_matches("Error:").trim().to_string())
        }
        _ => ServiceStatus::Stopped,
    }
}

const SELECT_COLS: &str =
    "id, name, project_id, spec, status, live_container_id, created_at, updated_at";

pub async fn create(db: &Db, spec: ServiceSpec) -> Result<Service> {
    // Unicidade: dois serviços não podem ter o mesmo nome (normalizado) no
    // mesmo projeto — o nome vira o container/DNS `rp_<safe_name>` e o compose
    // project name, então colidiriam. Comparamos por `normalize_name` para
    // pegar também nomes diferentes que colapsam no mesmo safe_name
    // (ex.: "my-api" e "my_api").
    let new_safe = spec.safe_name();
    let existing: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM service WHERE project_id = ?")
            .bind(&spec.project_id)
            .fetch_all(db)
            .await?;
    if existing.iter().any(|(n,)| shared::normalize_name(n) == new_safe) {
        anyhow::bail!(
            "já existe um serviço com o nome \"{}\" neste projeto",
            spec.name
        );
    }

    let id = format!("svc_{}", Ulid::new());
    info!(id = %id, name = %spec.name, project_id = %spec.project_id, "db::services:
:create");
    let now = Utc::now();
    let spec_json = serde_json::to_string(&spec)?;
    sqlx::query(
        "INSERT INTO service (id, name, project_id, spec, status, live_container_id, created_at, updated_at)
         VALUES (?, ?, ?, ?, 'Stopped', NULL, ?, ?)",
    )
    .bind(&id)
    .bind(&spec.name)
    .bind(&spec.project_id)
    .bind(&spec_json)
    .bind(now)
    .bind(now)
    .execute(db)
    .await?;
    let svc = Service {
        id: id.clone(),
        spec,
        status: ServiceStatus::Stopped,
        live_container_id: None,
        created_at: now,
        updated_at: now,
    };
    info!(service_id = %svc.id, name = %svc.spec.name, "db::services:
:create: salvo");
    Ok(svc)
}

pub async fn list(db: &Db, project_id: &str) -> Result<Vec<Service>> {
    let rows = sqlx::query_as::<_, ServiceRow>(&format!(
        "SELECT {SELECT_COLS} FROM service WHERE project_id = ? ORDER BY created_at ASC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(row_to_service).collect()
}

pub async fn get(db: &Db, id: &str) -> Result<Option<Service>> {
    let row =
        sqlx::query_as::<_, ServiceRow>(&format!("SELECT {SELECT_COLS} FROM service WHERE id = ?"))
            .bind(id)
            .fetch_optional(db)
            .await?;
    row.map(row_to_service).transpose()
}

pub async fn update_spec(db: &Db, id: &str, spec: ServiceSpec) -> Result<Option<Service>> {
    // Mesma regra de unicidade de `create`, mas ignorando o próprio serviço:
    // um rename não pode colidir (por nome normalizado) com outro serviço do
    // mesmo projeto.
    let new_safe = spec.safe_name();
    let others: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM service WHERE project_id = ? AND id != ?")
            .bind(&spec.project_id)
            .bind(id)
            .fetch_all(db)
            .await?;
    if others.iter().any(|(n,)| shared::normalize_name(n) == new_safe) {
        anyhow::bail!(
            "já existe um serviço com o nome \"{}\" neste projeto",
            spec.name
        );
    }

    let spec_json = serde_json::to_string(&spec)?;
    let now = Utc::now();
    let rows_affected =
        sqlx::query("UPDATE service SET spec = ?, name = ?, updated_at = ? WHERE id = ?")
            .bind(&spec_json)
            .bind(&spec.name)
            .bind(now)
            .bind(id)
            .execute(db)
            .await?
            .rows_affected();
    if rows_affected == 0 {
        return Ok(None);
    }
    get(db, id).await
}

/// Remove o job dado da fila de pré-deploy check (`pre_deploy_job_ids`, e do
/// `pre_deploy_job_id` legado) de qualquer serviço que o referencie —
/// cascade do `job_delete` (o job pode ter sido apagado em Schedules
/// enquanto ainda referenciado como check de um ou mais serviços; sem essa
/// limpeza o deploy falha em `DeployState::PreDeployCheck` com "job não
/// encontrado", e a UI mostra a fila sem esse item porque o seletor só lista
/// jobs que existem hoje, mascarando o ID órfão que continua salvo no
/// spec). Preserva a ordem dos itens restantes. Retorna quantos serviços
/// foram atualizados.
pub async fn clear_pre_deploy_job(db: &Db, job_id: &str) -> Result<u64> {
    let rows = sqlx::query_as::<_, ServiceRow>(&format!("SELECT {SELECT_COLS} FROM service"))
        .fetch_all(db)
        .await?;
    let mut cleared = 0u64;
    for row in rows {
        let svc = row_to_service(row)?;
        let referenced_legacy = svc.spec.pre_deploy_job_id.as_deref() == Some(job_id);
        let referenced_in_queue = svc.spec.pre_deploy_job_ids.iter().any(|j| j == job_id);
        if !referenced_legacy && !referenced_in_queue {
            continue;
        }
        let mut spec = svc.spec;
        if referenced_legacy {
            spec.pre_deploy_job_id = None;
        }
        spec.pre_deploy_job_ids.retain(|j| j != job_id);
        let spec_json = serde_json::to_string(&spec)?;
        sqlx::query("UPDATE service SET spec = ?, updated_at = ? WHERE id = ?")
            .bind(&spec_json)
            .bind(Utc::now())
            .bind(&svc.id)
            .execute(db)
            .await?;
        cleared += 1;
    }
    Ok(cleared)
}

pub async fn update_status(
    db: &Db,
    id: &str,
    status: &ServiceStatus,
    container_id: Option<&str>,
) -> Result<()> {

    info!(
        service_id = %id,
        status = %status,
        container_id = ?container_id.map(
            |container_id| format!("...{}", &container_id[..container_id.len().min(10)])
        ),
        "db::services::update_status"
    );
    let now = Utc::now();
    sqlx::query(
        "UPDATE service SET status = ?, live_container_id = ?, updated_at = ? WHERE id = ?",
    )
    .bind(status.to_string())
    .bind(container_id)
    .bind(now)
    .bind(id)
    .execute(db)
    .await?;
    debug!(service_id = %id, status = %status, "db::services:
:update_status: atualizado");
    Ok(())
}

pub async fn delete(db: &Db, id: &str) -> Result<bool> {
    let rows_affected = sqlx::query("DELETE FROM service WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?
        .rows_affected();
    Ok(rows_affected > 0)
}

pub async fn get_running(db: &Db) -> Result<Vec<Service>> {
    let rows = sqlx::query_as::<_, ServiceRow>(&format!(
        "SELECT {SELECT_COLS} FROM service WHERE status = 'Running'"
    ))
    .fetch_all(db)
    .await?;
    rows.into_iter().map(row_to_service).collect()
}

pub async fn count_by_project(db: &Db, project_id: &str) -> Result<i64> {
    let (count,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM service WHERE project_id = ?")
            .bind(project_id)
            .fetch_one(db)
            .await?;
    Ok(count)
}

pub async fn get_watchable(db: &Db) -> Result<Vec<Service>> {
    let rows = sqlx::query_as::<_, ServiceRow>(&format!(
        "SELECT {SELECT_COLS} FROM service WHERE status IN ('Running', 'Degraded')"
    ))
    .fetch_all(db)
    .await?;
    rows.into_iter().map(row_to_service).collect()
}

pub async fn list_all(db: &Db) -> Result<Vec<Service>> {
    let rows = sqlx::query_as::<_, ServiceRow>(&format!(
        "SELECT {SELECT_COLS} FROM service"
    ))
    .fetch_all(db)
    .await?;
    rows.into_iter().map(row_to_service).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::{Healthcheck, ResourceLimits, ServiceSource};

    fn spec(name: &str, project: &str) -> ServiceSpec {
        ServiceSpec {
            name: name.into(),
            project_id: project.into(),
            source: ServiceSource::Registry { image: "nginx:latest".into() },
            port: 80,
            host_port: None,
            domain: None,
            tls_enabled: false,
            env_vars: vec![],
            env_comments: vec![],
            volumes: vec![],
            healthcheck: Healthcheck::default(),
            replicas: 1,
            resources: ResourceLimits::default(),
            run_command: None,
            run_args: vec![],
            db_kind: None,
            domains: vec![],
            pre_deploy_job_id: None,
            pre_deploy_job_ids: vec![],
        }
    }

    async fn mem_db() -> Db {
        let dir = std::env::temp_dir()
            .join(format!("rustploy_test_{}", Ulid::new()));
        super::super::connect(&dir).await.unwrap()
    }

    #[tokio::test]
    async fn rejeita_nome_duplicado_no_mesmo_projeto() {
        let db = mem_db().await;
        create(&db, spec("api", "proj_a")).await.unwrap();
        // Mesmo nome exato → erro.
        assert!(create(&db, spec("api", "proj_a")).await.is_err());
        // Nome que normaliza para o mesmo safe_name ("my-api" == "my_api") → erro.
        create(&db, spec("my-api", "proj_a")).await.unwrap();
        assert!(create(&db, spec("my_api", "proj_a")).await.is_err());
    }

    #[tokio::test]
    async fn permite_mesmo_nome_em_projetos_diferentes() {
        let db = mem_db().await;
        create(&db, spec("api", "proj_a")).await.unwrap();
        // Projetos têm redes isoladas → sem colisão.
        assert!(create(&db, spec("api", "proj_b")).await.is_ok());
    }

    #[tokio::test]
    async fn rename_nao_pode_colidir_mas_permite_o_proprio_nome() {
        let db = mem_db().await;
        create(&db, spec("api", "proj_a")).await.unwrap();
        let web = create(&db, spec("web", "proj_a")).await.unwrap();

        // Renomear "web" → "api" (já existe) deve falhar.
        assert!(update_spec(&db, &web.id, spec("api", "proj_a")).await.is_err());
        // Renomear para um nome livre funciona.
        assert!(update_spec(&db, &web.id, spec("web2", "proj_a")).await.unwrap().is_some());
        // Regravar o próprio serviço com o mesmo nome (não é colisão) funciona.
        assert!(update_spec(&db, &web.id, spec("web2", "proj_a")).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn clear_pre_deploy_job_limpa_so_servicos_que_referenciam_o_job_legado() {
        let db = mem_db().await;
        let mut with_job = spec("api", "proj_a");
        with_job.pre_deploy_job_id = Some("job_alvo".into());
        let api = create(&db, with_job).await.unwrap();

        let mut with_other_job = spec("web", "proj_a");
        with_other_job.pre_deploy_job_id = Some("job_outro".into());
        let web = create(&db, with_other_job).await.unwrap();

        let cleared = clear_pre_deploy_job(&db, "job_alvo").await.unwrap();
        assert_eq!(cleared, 1);

        assert_eq!(get(&db, &api.id).await.unwrap().unwrap().spec.pre_deploy_job_id, None);
        assert_eq!(
            get(&db, &web.id).await.unwrap().unwrap().spec.pre_deploy_job_id,
            Some("job_outro".into())
        );
    }

    /// Serviço com a fila NOVA (`pre_deploy_job_ids`): o job removido some da
    /// lista, mas os outros ficam — e na MESMA ordem relativa (só o item
    /// removido sai do meio).
    #[tokio::test]
    async fn clear_pre_deploy_job_remove_so_o_item_da_fila_preservando_ordem() {
        let db = mem_db().await;
        let mut with_queue = spec("api", "proj_a");
        with_queue.pre_deploy_job_ids =
            vec!["job_migration".into(), "job_alvo".into(), "job_test".into()];
        let api = create(&db, with_queue).await.unwrap();

        let cleared = clear_pre_deploy_job(&db, "job_alvo").await.unwrap();
        assert_eq!(cleared, 1);

        assert_eq!(
            get(&db, &api.id).await.unwrap().unwrap().spec.pre_deploy_job_ids,
            vec!["job_migration".to_string(), "job_test".to_string()]
        );
    }

    #[tokio::test]
    async fn clear_pre_deploy_job_no_op_quando_ninguem_referencia() {
        let db = mem_db().await;
        let mut with_queue = spec("api", "proj_a");
        with_queue.pre_deploy_job_ids = vec!["job_migration".into()];
        create(&db, with_queue).await.unwrap();

        let cleared = clear_pre_deploy_job(&db, "job_nunca_referenciado").await.unwrap();
        assert_eq!(cleared, 0);
    }
}
