use super::{extract_id, Db};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::{DeployState, Deployment, StateTransition};
use surrealdb::sql::Datetime as SdbDatetime;
use tracing::info;
use ulid::Ulid;

#[derive(Debug, Serialize, Deserialize)]
struct DeploymentRecord {
    id: Option<surrealdb::sql::Thing>,
    service_id: String,
    image: String,
    state: String,
    states_log: Vec<Value>,
    started_at: SdbDatetime,
    finished_at: Option<SdbDatetime>,
}

impl DeploymentRecord {
    fn into_deployment(self) -> Deployment {
        let state = parse_state(&self.state);
        let states_log = self
            .states_log
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        Deployment {
            id: self.id.as_ref().map(extract_id).unwrap_or_default(),
            service_id: self.service_id,
            image: self.image,
            state,
            states_log,
            started_at: self.started_at.0,
            finished_at: self.finished_at.map(|dt| dt.0),
        }
    }
}

fn parse_state(s: &str) -> DeployState {
    match s {
        "Pending" => DeployState::Pending,
        "ResolvingDeps" => DeployState::ResolvingDeps,
        "PullingImage" => DeployState::PullingImage,
        "CloningRepo" => DeployState::CloningRepo,
        "BuildingImage" => DeployState::BuildingImage,
        "Staging" => DeployState::Staging,
        "HealthcheckPolling" => DeployState::HealthcheckPolling,
        "SwappingIn" => DeployState::SwappingIn,
        "Draining" => DeployState::Draining,
        "Promoting" => DeployState::Promoting,
        "Live" => DeployState::Live,
        "Stopped" => DeployState::Stopped,
        "RollingBack" => DeployState::RollingBack,
        "Failed" => DeployState::Failed,
        "Pruning" => DeployState::Pruning,
        _ => DeployState::Failed,
    }
}

pub async fn create(db: &Db, service_id: &str, image: &str) -> Result<Deployment> {
    let id = Ulid::new().to_string();
    info!(id = %id, service_id = %service_id, image = %image, "db::deployments::create: criando deployment");
    let record = DeploymentRecord {
        id: None,
        service_id: service_id.to_string(),
        image: image.to_string(),
        state: "Pending".into(),
        states_log: vec![],
        started_at: SdbDatetime::from(Utc::now()),
        finished_at: None,
    };
    let created: Option<DeploymentRecord> =
        db.create(("deployment", &id)).content(record).await?;
    let dep = created.unwrap().into_deployment();
    info!(deployment_id = %dep.id, service_id = %service_id, "db::deployments::create: deployment salvo");
    Ok(dep)
}

pub async fn get(db: &Db, id: &str) -> Result<Option<Deployment>> {
    let record: Option<DeploymentRecord> = db.select(("deployment", id)).await?;
    Ok(record.map(|r| r.into_deployment()))
}

pub async fn list_for_service(db: &Db, service_id: &str, limit: usize) -> Result<Vec<Deployment>> {
    let mut result = db
        .query("SELECT * FROM deployment WHERE service_id = $sid ORDER BY started_at DESC LIMIT $lim")
        .bind(("sid", service_id.to_string()))
        .bind(("lim", limit as i64))
        .await?;
    let records: Vec<DeploymentRecord> = result.take(0)?;
    Ok(records.into_iter().map(|r| r.into_deployment()).collect())
}

pub async fn latest_for_service(db: &Db, service_id: &str) -> Result<Option<Deployment>> {
    let mut result = db
        .query("SELECT * FROM deployment WHERE service_id = $sid ORDER BY started_at DESC LIMIT 1")
        .bind(("sid", service_id.to_string()))
        .await?;
    let mut records: Vec<DeploymentRecord> = result.take(0)?;
    Ok(records.pop().map(|r| r.into_deployment()))
}

pub async fn transition(
    db: &Db,
    id: &str,
    from: &DeployState,
    to: DeployState,
    message: Option<String>,
) -> Result<Deployment> {
    info!(
        deployment_id = %id,
        from = from.label(),
        to = to.label(),
        terminal = to.is_terminal(),
        "db::deployments::transition: gravando transição"
    );
    let transition = StateTransition {
        from: from.clone(),
        to: to.clone(),
        at: Utc::now(),
        message,
    };
    let transition_json = serde_json::to_value(&transition)?;

    let finished_at: Option<SdbDatetime> = if to.is_terminal() {
        Some(SdbDatetime::from(Utc::now()))
    } else {
        None
    };

    // IMPORTANTE: usar type::thing() em vez de WHERE id = $id
    // porque $id é uma String mas o campo id é um Thing — a comparação falharia.
    let mut result = db
        .query(
            "UPDATE type::thing('deployment', $id) SET
                state = $state,
                states_log += $transition,
                finished_at = $finished_at
             RETURN AFTER",
        )
        .bind(("id", id.to_string()))
        .bind(("state", to.label()))
        .bind(("transition", transition_json))
        .bind(("finished_at", finished_at))
        .await?;

    let records: Vec<DeploymentRecord> = result.take(0)?;
    records
        .into_iter()
        .next()
        .map(|r| r.into_deployment())
        .ok_or_else(|| anyhow::anyhow!("deployment not found: {id}"))
}

pub async fn list_recent(db: &Db, limit: usize) -> Result<Vec<Deployment>> {
    let mut result = db
        .query("SELECT * FROM deployment ORDER BY started_at DESC LIMIT $lim")
        .bind(("lim", limit as i64))
        .await?;
    let records: Vec<DeploymentRecord> = result.take(0)?;
    Ok(records.into_iter().map(|r| r.into_deployment()).collect())
}

pub async fn get_non_terminal(db: &Db) -> Result<Vec<Deployment>> {
    let terminal: Vec<String> = vec!["Live".into(), "Stopped".into(), "Failed".into(), "Pruning".into()];
    let mut result = db
        .query("SELECT * FROM deployment WHERE state NOT IN $terminal")
        .bind(("terminal", terminal))
        .await?;
    let records: Vec<DeploymentRecord> = result.take(0)?;
    Ok(records.into_iter().map(|r| r.into_deployment()).collect())
}
