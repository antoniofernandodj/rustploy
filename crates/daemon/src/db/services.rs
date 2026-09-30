//! Tabela `service`: CRUD do ServiceSpec, status e container live.

use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use shared::{Service, ServiceSource, ServiceSpec, ServiceStatus};
use tracing::{debug, info, warn};
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
    Option<String>, // compose_project (nome gravado da stack; só serviços Compose)
);

fn row_to_service(row: ServiceRow) -> Result<Service> {
    let (
        id,
        _name,
        _project_id,
        spec_json,
        status_str,
        live_container_id,
        created_at,
        updated_at,
        compose_project,
    ) = row;
    let spec: ServiceSpec = serde_json::from_str(&spec_json)
        .map_err(|e| anyhow::anyhow!("falha ao deserializar spec do banco: {}", e))?;
    let status = parse_status(&status_str);
    Ok(Service {
        id,
        spec,
        compose_project,
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

const SELECT_COLS: &str = "id, name, project_id, spec, status, live_container_id, created_at, updated_at, compose_project";

/// Nome de stack para um serviço Compose novo: o curto
/// (`rp_<últimos 8 do ID>_<nome>`); se já está em uso — chance de ~1 em 10¹² —,
/// o ID inteiro.
async fn new_compose_project(db: &Db, id: &str, name: &str) -> Result<String> {
    let short = shared::new_compose_project_name(id, name);
    let taken: Option<i64> = sqlx::query_scalar("SELECT 1 FROM service WHERE compose_project = ?")
        .bind(&short)
        .fetch_optional(db)
        .await?;
    Ok(match taken {
        None => short,
        Some(_) => shared::new_compose_project_name_long(id, name),
    })
}

/// Regras de nome/`shared` (prefixo `rp-shared-` reservado etc.) — ver
/// `shared::connection::validate_shared_rules`.
fn check_shared_rules(spec: &ServiceSpec) -> Result<()> {
    shared::connection::validate_shared_rules(
        &spec.name,
        &spec.source,
        spec.db_kind.as_deref(),
        spec.shared.is_some(),
    )
    .map_err(|e| anyhow::anyhow!(e))
}

pub async fn create(db: &Db, spec: ServiceSpec) -> Result<Service> {
    check_shared_rules(&spec)?;
    // Unicidade: dois serviços não podem ter o mesmo nome (normalizado) no
    // mesmo projeto — o nome vira o container/DNS `rp_<safe_name>` e o compose
    // project name, então colidiriam. Comparamos por `normalize_name` para
    // pegar também nomes diferentes que colapsam no mesmo safe_name
    // (ex.: "my-api" e "my_api").
    let new_safe = spec.safe_name();
    let existing: Vec<(String,)> = sqlx::query_as("SELECT name FROM service WHERE project_id = ?")
        .bind(&spec.project_id)
        .fetch_all(db)
        .await?;
    if existing
        .iter()
        .any(|(n,)| shared::normalize_name(n) == new_safe)
    {
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
    let compose_project = match &spec.source {
        ServiceSource::Compose(_) => Some(new_compose_project(db, &id, &spec.name).await?),
        _ => None,
    };
    sqlx::query(
        "INSERT INTO service (id, name, project_id, spec, status, live_container_id, created_at, updated_at, compose_project)
         VALUES (?, ?, ?, ?, 'Stopped', NULL, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&spec.name)
    .bind(&spec.project_id)
    .bind(&spec_json)
    .bind(now)
    .bind(now)
    .bind(&compose_project)
    .execute(db)
    .await?;
    let svc = Service {
        id: id.clone(),
        spec,
        compose_project,
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
    check_shared_rules(&spec)?;
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
    if others
        .iter()
        .any(|(n,)| shared::normalize_name(n) == new_safe)
    {
        anyhow::bail!(
            "já existe um serviço com o nome \"{}\" neste projeto",
            spec.name
        );
    }

    let spec_json = serde_json::to_string(&spec)?;
    let now = Utc::now();
    // Serviço que virou Compose numa edição de fonte ainda não tem stack
    // gravada: grava agora, com o nome de agora. Se já tem, não mexe — é o que
    // faz o rename não trocar a stack.
    let new_stack = match &spec.source {
        ServiceSource::Compose(_) => {
            let has: Option<Option<String>> =
                sqlx::query_scalar("SELECT compose_project FROM service WHERE id = ?")
                    .bind(id)
                    .fetch_optional(db)
                    .await?;
            match has {
                Some(None) => Some(new_compose_project(db, id, &spec.name).await?),
                _ => None,
            }
        }
        _ => None,
    };
    let rows_affected = sqlx::query(
        "UPDATE service SET spec = ?, name = ?, updated_at = ?,
                compose_project = COALESCE(compose_project, ?) WHERE id = ?",
    )
    .bind(&spec_json)
    .bind(&spec.name)
    .bind(now)
    .bind(&new_stack)
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
    // Servidor compartilhado removido: os acessos dele não valem mais.
    super::shared_access::delete_for_server(db, id).await?;
    super::managed_database::delete_for_server(db, id).await?;
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
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM service WHERE project_id = ?")
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
    let rows = sqlx::query_as::<_, ServiceRow>(&format!("SELECT {SELECT_COLS} FROM service"))
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
            source: ServiceSource::Registry {
                image: "nginx:latest".into(),
            },
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
            shared: None,
        }
    }

    async fn mem_db() -> Db {
        let dir = std::env::temp_dir().join(format!("rustploy_test_{}", Ulid::new()));
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
        assert!(
            update_spec(&db, &web.id, spec("api", "proj_a"))
                .await
                .is_err()
        );
        // Renomear para um nome livre funciona.
        assert!(
            update_spec(&db, &web.id, spec("web2", "proj_a"))
                .await
                .unwrap()
                .is_some()
        );
        // Regravar o próprio serviço com o mesmo nome (não é colisão) funciona.
        assert!(
            update_spec(&db, &web.id, spec("web2", "proj_a"))
                .await
                .unwrap()
                .is_some()
        );
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

        assert_eq!(
            get(&db, &api.id)
                .await
                .unwrap()
                .unwrap()
                .spec
                .pre_deploy_job_id,
            None
        );
        assert_eq!(
            get(&db, &web.id)
                .await
                .unwrap()
                .unwrap()
                .spec
                .pre_deploy_job_id,
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
            get(&db, &api.id)
                .await
                .unwrap()
                .unwrap()
                .spec
                .pre_deploy_job_ids,
            vec!["job_migration".to_string(), "job_test".to_string()]
        );
    }

    #[tokio::test]
    async fn clear_pre_deploy_job_no_op_quando_ninguem_referencia() {
        let db = mem_db().await;
        let mut with_queue = spec("api", "proj_a");
        with_queue.pre_deploy_job_ids = vec!["job_migration".into()];
        create(&db, with_queue).await.unwrap();

        let cleared = clear_pre_deploy_job(&db, "job_nunca_referenciado")
            .await
            .unwrap();
        assert_eq!(cleared, 0);
    }
}

/// Chaves de serviço (`services:`) de um compose, para casar com as de uma stack
/// viva. Vazio se o YAML não parseia ou não tem `services`.
fn compose_service_keys(content: &str) -> std::collections::BTreeSet<String> {
    serde_yaml::from_str::<serde_yaml::Value>(content)
        .ok()
        .and_then(|v| v.get("services").and_then(|s| s.as_mapping().cloned()))
        .map(|m| {
            m.keys()
                .filter_map(|k| k.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Grava `compose_project` nos serviços Compose que ainda não têm, com o nome
/// **que a stack já usa** — nenhum container ou volume em produção muda de
/// nome. `live` são as stacks que existem hoje no Docker (nome → chaves de
/// serviço dos containers): cobre o serviço renomeado e ainda não redeployado,
/// cuja stack viva (a que tem os volumes) é a do nome antigo.
///
/// Uma stack só é adotada por um serviço **renomeado** se, além do prefixo do
/// nome (8 primeiros chars do ID — igual para tudo criado no mesmo segundo, como
/// num import de manifesto), as chaves de serviço dos containers cabem nas do
/// compose do serviço, e só uma stack serve. Sem isso um serviço qualquer sem
/// stack levaria a de outro. Sem como decidir, vale a fórmula legada com o nome
/// atual — o mesmo que o daemon já calculava. Depois cria o índice `UNIQUE`.
/// Roda no boot, com o Docker no ar.
pub async fn backfill_compose_projects(
    db: &Db,
    live: &std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
) -> Result<()> {
    let rows = sqlx::query_as::<_, ServiceRow>(&format!("SELECT {SELECT_COLS} FROM service"))
        .fetch_all(db)
        .await?;
    let services: Vec<Service> = rows
        .into_iter()
        .map(row_to_service)
        .collect::<Result<_>>()?;
    let compose: Vec<&Service> = services
        .iter()
        .filter(|s| matches!(s.spec.source, ServiceSource::Compose(_)))
        .collect();

    // Nomes que não podem ser "achados" por outro serviço: os já gravados e a
    // fórmula legada de cada serviço Compose.
    let mut taken: std::collections::HashSet<String> = compose
        .iter()
        .filter_map(|s| s.compose_project.clone())
        .collect();
    let legacy_of = |s: &Service| shared::compose_project_name(&s.id, &s.spec.name);
    let legacy_names: std::collections::HashSet<String> =
        compose.iter().map(|s| legacy_of(s)).collect();

    // Primeiro os serviços cuja stack viva tem exatamente o nome legado (o caso
    // comum), para que essas stacks saiam de `taken`/candidatas dos demais.
    let (exact, rest): (Vec<&&Service>, Vec<&&Service>) = compose
        .iter()
        .filter(|s| s.compose_project.is_none())
        .partition(|s| live.contains_key(&legacy_of(s)));
    for svc in exact {
        let legacy = legacy_of(svc);
        sqlx::query(
            "UPDATE service SET compose_project = ? WHERE id = ? AND compose_project IS NULL",
        )
        .bind(&legacy)
        .bind(&svc.id)
        .execute(db)
        .await?;
        taken.insert(legacy);
    }

    for svc in rest {
        let legacy = legacy_of(svc);
        let ServiceSource::Compose(c) = &svc.spec.source else {
            continue;
        };
        let keys = compose_service_keys(&c.content);
        let id_prefix = {
            let legacy_id = svc.id.strip_prefix("svc_").unwrap_or(&svc.id);
            format!(
                "rp_{}_",
                legacy_id.get(..8).unwrap_or(legacy_id).to_lowercase()
            )
        };
        let candidates: Vec<&String> = live
            .iter()
            .filter(|(p, _)| p.starts_with(&id_prefix))
            .filter(|(p, _)| !taken.contains(*p) && !legacy_names.contains(*p))
            // As chaves de serviço da stack cabem nas do compose deste serviço.
            .filter(|(_, stack_keys)| !stack_keys.is_empty() && stack_keys.is_subset(&keys))
            .map(|(p, _)| p)
            .collect();
        let chosen = if candidates.len() == 1 {
            info!(
                service_id = %svc.id, name = %svc.spec.name, stack = %candidates[0],
                "db::services::backfill: stack viva com nome antigo (serviço renomeado sem redeploy)"
            );
            candidates[0].clone()
        } else {
            if candidates.len() > 1 {
                warn!(
                    service_id = %svc.id, name = %svc.spec.name, ?candidates,
                    "db::services::backfill: mais de uma stack candidata; usando o nome legado"
                );
            }
            legacy
        };
        sqlx::query(
            "UPDATE service SET compose_project = ? WHERE id = ? AND compose_project IS NULL",
        )
        .bind(&chosen)
        .bind(&svc.id)
        .execute(db)
        .await?;
        taken.insert(chosen);
    }

    super::ensure_unique_index(
        db,
        "idx_service_compose_project",
        "service(compose_project)",
        "duas stacks Compose com o mesmo nome; separar exige decidir de quem são os volumes",
    )
    .await;
    Ok(())
}

#[cfg(test)]
mod nome_gravado_tests {
    use super::*;
    use crate::db::projects;

    async fn temp_db() -> Db {
        let dir = std::env::temp_dir().join(format!("rustploy-test-{}", Ulid::new()));
        crate::db::connect(&dir).await.unwrap()
    }

    /// Stacks vivas do Docker: `(nome da stack, chaves de serviço dos containers)`.
    fn live(
        v: &[(&str, &[&str])],
    ) -> std::collections::BTreeMap<String, std::collections::BTreeSet<String>> {
        v.iter()
            .map(|(p, keys)| (p.to_string(), keys.iter().map(|k| k.to_string()).collect()))
            .collect()
    }

    #[tokio::test]
    async fn projeto_novo_grava_rede_com_id_inteiro_e_dois_juntos_nao_colidem() {
        let db = temp_db().await;
        let a = projects::create(&db, "a".into(), None).await.unwrap();
        let b = projects::create(&db, "b".into(), None).await.unwrap();
        let na = projects::network_name(&db, &a.id).await.unwrap();
        let nb = projects::network_name(&db, &b.id).await.unwrap();
        assert_eq!(na, shared::new_project_network_name(&a.id));
        assert!(na.len() > "rp_net_".len() + 20, "ID inteiro: {na}");
        assert_ne!(na, nb);
    }

    #[tokio::test]
    async fn migracao_preenche_rede_com_o_nome_legado() {
        let db = temp_db().await;
        sqlx::query(
            "INSERT INTO project (id, name, env_vars, created_at) VALUES ('prj_01ABCDEFGHJK', 'velho', '[]', ?)",
        )
        .bind(Utc::now())
        .execute(&db)
        .await
        .unwrap();
        // Roda a migração de novo, como num boot seguinte.
        crate::db::connect_existing_for_test(&db).await;
        assert_eq!(
            projects::network_name(&db, "prj_01ABCDEFGHJK")
                .await
                .unwrap(),
            "rp_net_01ABCDEF"
        );
    }

    #[tokio::test]
    async fn rename_de_servico_compose_nao_muda_a_stack() {
        let db = temp_db().await;
        let p = projects::create(&db, "p".into(), None).await.unwrap();
        let spec = shared::wizard::compose_spec("postgres".into(), p.id.clone());
        let svc = create(&db, spec).await.unwrap();
        let stack = svc.compose_project.clone().expect("Compose grava a stack");
        assert_eq!(stack, shared::new_compose_project_name(&svc.id, "postgres"));

        let mut spec = svc.spec.clone();
        spec.name = "banco".into();
        let renomeado = update_spec(&db, &svc.id, spec).await.unwrap().unwrap();
        assert_eq!(renomeado.spec.name, "banco");
        assert_eq!(renomeado.compose_project_name(), stack);
    }

    #[tokio::test]
    async fn servico_que_nao_e_compose_nao_grava_stack() {
        let db = temp_db().await;
        let p = projects::create(&db, "p".into(), None).await.unwrap();
        let spec = shared::wizard::compose_spec("x".into(), p.id.clone());
        let mut spec = spec;
        spec.source = ServiceSource::Registry {
            image: "nginx".into(),
        };
        let svc = create(&db, spec).await.unwrap();
        assert!(svc.compose_project.is_none());
    }

    #[tokio::test]
    async fn backfill_usa_a_stack_viva_de_servico_renomeado_sem_redeploy() {
        let db = temp_db().await;
        let p = projects::create(&db, "p".into(), None).await.unwrap();
        let mut spec = shared::wizard::compose_spec("postgres".into(), p.id.clone());
        spec.source = ServiceSource::Compose(shared::ComposeSource {
            content: "services:\n  app:\n    image: x\n".into(),
            ingress_service: None,
        });
        let svc = create(&db, spec).await.unwrap();
        // Simula banco antigo: stack não gravada; serviço renomeado; a stack
        // viva no Docker é a do nome antigo.
        sqlx::query("UPDATE service SET compose_project = NULL WHERE id = ?")
            .bind(&svc.id)
            .execute(&db)
            .await
            .unwrap();
        let mut spec = svc.spec.clone();
        spec.name = "banco".into();
        update_spec(&db, &svc.id, spec).await.unwrap();
        sqlx::query("UPDATE service SET compose_project = NULL WHERE id = ?")
            .bind(&svc.id)
            .execute(&db)
            .await
            .unwrap();

        let antiga = shared::compose_project_name(&svc.id, "postgres");
        backfill_compose_projects(&db, &live(&[("outra_coisa", &["x"]), (&antiga, &["app"])]))
            .await
            .unwrap();
        let depois = get(&db, &svc.id).await.unwrap().unwrap();
        assert_eq!(depois.compose_project.as_deref(), Some(antiga.as_str()));
    }

    #[tokio::test]
    async fn backfill_sem_stack_no_docker_usa_a_formula_legada_com_o_nome_atual() {
        let db = temp_db().await;
        let p = projects::create(&db, "p".into(), None).await.unwrap();
        let svc = create(&db, shared::wizard::compose_spec("db".into(), p.id.clone()))
            .await
            .unwrap();
        sqlx::query("UPDATE service SET compose_project = NULL WHERE id = ?")
            .bind(&svc.id)
            .execute(&db)
            .await
            .unwrap();
        backfill_compose_projects(&db, &live(&[])).await.unwrap();
        let depois = get(&db, &svc.id).await.unwrap().unwrap();
        assert_eq!(
            depois.compose_project,
            Some(shared::compose_project_name(&svc.id, "db"))
        );
    }

    #[tokio::test]
    async fn backfill_nao_rouba_a_stack_de_outro_servico() {
        let db = temp_db().await;
        let p = projects::create(&db, "p".into(), None).await.unwrap();
        let a = create(&db, shared::wizard::compose_spec("a".into(), p.id.clone()))
            .await
            .unwrap();
        let b = create(&db, shared::wizard::compose_spec("b".into(), p.id.clone()))
            .await
            .unwrap();
        sqlx::query("UPDATE service SET compose_project = NULL")
            .execute(&db)
            .await
            .unwrap();
        // Só a stack de `a` existe no Docker; `b` não pode adotá-la, mesmo se
        // os dois tivessem os mesmos 8 primeiros chars do ID.
        let stack_a = shared::compose_project_name(&a.id, "a");
        backfill_compose_projects(&db, &live(&[(&stack_a, &["app"])]))
            .await
            .unwrap();
        let a2 = get(&db, &a.id).await.unwrap().unwrap();
        let b2 = get(&db, &b.id).await.unwrap().unwrap();
        assert_ne!(a2.compose_project, b2.compose_project);
        assert_eq!(
            b2.compose_project,
            Some(shared::compose_project_name(&b.id, "b"))
        );
    }

    /// Regressão achada no teste com a infra de produção: um import de manifesto
    /// cria todos os serviços no mesmo segundo, então todos têm o mesmo prefixo
    /// de ID. O serviço `db`, que nunca subiu, adotava a stack viva do `broker`
    /// (renomeado para `mq` sem redeploy) por ser o único candidato pelo prefixo.
    #[tokio::test]
    async fn backfill_de_import_no_mesmo_segundo_liga_cada_stack_ao_seu_servico() {
        let db = temp_db().await;
        let p = projects::create(&db, "p".into(), None).await.unwrap();
        let mk = |name: &str, key: &str| {
            let mut spec = shared::wizard::compose_spec(name.into(), p.id.clone());
            spec.source = ServiceSource::Compose(shared::ComposeSource {
                content: format!("services:\n  {key}:\n    image: x\n"),
                ingress_service: None,
            });
            spec
        };
        let db_svc = create(&db, mk("db", "rp_db")).await.unwrap();
        let broker = create(&db, mk("broker", "rp_broker")).await.unwrap();
        // Mesmo prefixo de 8 chars, como num import.
        for (svc, sufixo) in [
            (&db_svc, "AAAAAAAAAAAAAAAAAAAAAAAA"),
            (&broker, "BBBBBBBBBBBBBBBBBBBBBBBB"),
        ] {
            sqlx::query("UPDATE service SET id = ? WHERE id = ?")
                .bind(format!("svc_01M3Q7T1{sufixo}"))
                .bind(&svc.id)
                .execute(&db)
                .await
                .unwrap();
        }
        let broker_id = "svc_01M3Q7T1BBBBBBBBBBBBBBBBBBBBBBBB";
        let db_id = "svc_01M3Q7T1AAAAAAAAAAAAAAAAAAAAAAAA";
        // `broker` foi renomeado para `mq` e a stack viva segue com o nome antigo.
        let mut spec = get(&db, broker_id).await.unwrap().unwrap().spec;
        spec.name = "mq".into();
        update_spec(&db, broker_id, spec).await.unwrap();
        sqlx::query("UPDATE service SET compose_project = NULL")
            .execute(&db)
            .await
            .unwrap();

        backfill_compose_projects(&db, &live(&[("rp_01m3q7t1_broker", &["rp_broker"])]))
            .await
            .unwrap();

        let mq = get(&db, broker_id).await.unwrap().unwrap();
        let db2 = get(&db, db_id).await.unwrap().unwrap();
        assert_eq!(mq.compose_project.as_deref(), Some("rp_01m3q7t1_broker"));
        assert_eq!(db2.compose_project.as_deref(), Some("rp_01m3q7t1_db"));
    }
}
