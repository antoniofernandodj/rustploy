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
    let id = Ulid::new().to_string();
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

pub async fn update_status(
    db: &Db,
    id: &str,
    status: &ServiceStatus,
    container_id: Option<&str>,
) -> Result<()> {
    info!(service_id = %id, status = %status, container_id = ?container_id, "db::services:
:update_status");
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
