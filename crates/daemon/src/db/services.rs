use super::{extract_id, Db};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::{Service, ServiceSpec, ServiceStatus};
use ulid::Ulid;

#[derive(Debug, Serialize, Deserialize)]
struct ServiceRecord {
    id: Option<surrealdb::sql::Thing>,
    name: String,
    project_id: String,
    spec: Value,
    status: String,
    live_container_id: Option<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

impl ServiceRecord {
    fn into_service(self) -> Service {
        let spec: ServiceSpec =
            serde_json::from_value(self.spec).unwrap_or_else(|_| panic!("invalid spec in db"));
        let status = parse_status(&self.status);
        Service {
            id: self.id.as_ref().map(extract_id).unwrap_or_default(),
            spec,
            status,
            live_container_id: self.live_container_id,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

fn parse_status(s: &str) -> ServiceStatus {
    match s {
        "Stopped" => ServiceStatus::Stopped,
        "Deploying" => ServiceStatus::Deploying,
        "Running" => ServiceStatus::Running,
        "Degraded" => ServiceStatus::Degraded,
        s if s.starts_with("Error:") => {
            ServiceStatus::Error(s.trim_start_matches("Error:").trim().to_string())
        }
        _ => ServiceStatus::Stopped,
    }
}

pub async fn create(db: &Db, spec: ServiceSpec) -> Result<Service> {
    let id = Ulid::new().to_string();
    let now = Utc::now();
    let record = ServiceRecord {
        id: None,
        name: spec.name.clone(),
        project_id: spec.project_id.clone(),
        spec: serde_json::to_value(&spec)?,
        status: "Stopped".into(),
        live_container_id: None,
        created_at: now,
        updated_at: now,
    };
    let created: Option<ServiceRecord> = db.create(("service", &id)).content(record).await?;
    Ok(created.unwrap().into_service())
}

pub async fn list(db: &Db, project_id: &str) -> Result<Vec<Service>> {
    let mut result = db
        .query("SELECT * FROM service WHERE project_id = $pid")
        .bind(("pid", project_id.to_string()))
        .await?;
    let records: Vec<ServiceRecord> = result.take(0)?;
    Ok(records.into_iter().map(|r| r.into_service()).collect())
}

pub async fn get(db: &Db, id: &str) -> Result<Option<Service>> {
    let record: Option<ServiceRecord> = db.select(("service", id)).await?;
    Ok(record.map(|r| r.into_service()))
}

pub async fn update_spec(db: &Db, id: &str, spec: ServiceSpec) -> Result<Option<Service>> {
    let updated: Option<ServiceRecord> = db
        .update(("service", id))
        .merge(serde_json::json!({
            "spec": serde_json::to_value(&spec)?,
            "name": spec.name,
            "updated_at": Utc::now(),
        }))
        .await?;
    Ok(updated.map(|r| r.into_service()))
}

pub async fn update_status(
    db: &Db,
    id: &str,
    status: &ServiceStatus,
    container_id: Option<&str>,
) -> Result<()> {
    let status_str = status.to_string();
    let mut patch = serde_json::json!({
        "status": status_str,
        "updated_at": Utc::now(),
    });
    if let Some(cid) = container_id {
        patch["live_container_id"] = serde_json::Value::String(cid.to_string());
    }
    let _: Option<ServiceRecord> = db.update(("service", id)).merge(patch).await?;
    Ok(())
}

pub async fn delete(db: &Db, id: &str) -> Result<bool> {
    let deleted: Option<ServiceRecord> = db.delete(("service", id)).await?;
    Ok(deleted.is_some())
}

pub async fn get_running(db: &Db) -> Result<Vec<Service>> {
    let mut result = db
        .query("SELECT * FROM service WHERE status = 'Running'")
        .await?;
    let records: Vec<ServiceRecord> = result.take(0)?;
    Ok(records.into_iter().map(|r| r.into_service()).collect())
}
