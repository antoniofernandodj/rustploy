use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use shared::{DeployState, Deployment, StateTransition};
use tracing::info;
use ulid::Ulid;

type DeploymentRow = (
    String,                // id
    String,                // service_id
    String,                // image
    String,                // state
    String,                // states_log (JSON)
    DateTime<Utc>,         // started_at
    Option<DateTime<Utc>>, // finished_at
);

fn row_to_deployment(row: DeploymentRow) -> Deployment {
    let (id, service_id, image, state_str, states_log_json, started_at, finished_at) = row;
    let state = parse_state(&state_str);
    let states_log: Vec<StateTransition> =
        serde_json::from_str(&states_log_json).unwrap_or_default();
    Deployment {
        id,
        service_id,
        image,
        state,
        states_log,
        started_at,
        finished_at,
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
        "ComposingUp" => DeployState::ComposingUp,
        _ => DeployState::Failed,
    }
}

const SELECT_COLS: &str = "id, service_id, image, state, states_log, started_at, finished_at";

pub async fn create(db: &Db, service_id: &str, image: &str) -> Result<Deployment> {
    let id = format!("dep_{}", Ulid::new());
    info!(id = %id, service_id = %service_id, image = %image, "db::deployments::create");
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO deployment (id, service_id, image, state, states_log, started_at, finished_at)
         VALUES (?, ?, ?, 'Pending', '[]', ?, NULL)",
    )
    .bind(&id)
    .bind(service_id)
    .bind(image)
    .bind(now)
    .execute(db)
    .await?;
    let dep = Deployment {
        id: id.clone(),
        service_id: service_id.to_string(),
        image: image.to_string(),
        state: DeployState::Pending,
        states_log: vec![],
        started_at: now,
        finished_at: None,
    };
    info!(deployment_id = %dep.id, service_id = %service_id, "db::deployments::create: salvo");
    Ok(dep)
}

pub async fn get(db: &Db, id: &str) -> Result<Option<Deployment>> {
    let row = sqlx::query_as::<_, DeploymentRow>(&format!(
        "SELECT {SELECT_COLS} FROM deployment WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(db)
    .await?;
    Ok(row.map(row_to_deployment))
}

pub async fn list_for_service(db: &Db, service_id: &str, limit: usize) -> Result<Vec<Deployment>> {
    let rows = sqlx::query_as::<_, DeploymentRow>(&format!(
        "SELECT {SELECT_COLS} FROM deployment WHERE service_id = ? ORDER BY started_at DESC LIMIT ?"
    ))
    .bind(service_id)
    .bind(limit as i64)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(row_to_deployment).collect())
}

pub async fn latest_for_service(db: &Db, service_id: &str) -> Result<Option<Deployment>> {
    let row = sqlx::query_as::<_, DeploymentRow>(&format!(
        "SELECT {SELECT_COLS} FROM deployment WHERE service_id = ? ORDER BY started_at DESC LIMIT 1"
    ))
    .bind(service_id)
    .fetch_optional(db)
    .await?;
    Ok(row.map(row_to_deployment))
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
        "db::deployments::transition"
    );
    let transition = StateTransition {
        from: from.clone(),
        to: to.clone(),
        at: Utc::now(),
        message,
    };

    // Fetch current states_log, append the new transition, write back atomically.
    let row = sqlx::query_as::<_, (String,)>("SELECT states_log FROM deployment WHERE id = ?")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("deployment not found: {id}"))?;

    let mut log: Vec<StateTransition> = serde_json::from_str(&row.0).unwrap_or_default();
    log.push(transition);
    let log_json = serde_json::to_string(&log)?;

    let finished_at: Option<DateTime<Utc>> = if to.is_terminal() {
        Some(Utc::now())
    } else {
        None
    };

    sqlx::query("UPDATE deployment SET state = ?, states_log = ?, finished_at = ? WHERE id = ?")
        .bind(to.label())
        .bind(&log_json)
        .bind(finished_at)
        .bind(id)
        .execute(db)
        .await?;

    get(db, id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("deployment not found after update: {id}"))
}

pub async fn list_recent(db: &Db, limit: usize) -> Result<Vec<Deployment>> {
    let rows = sqlx::query_as::<_, DeploymentRow>(&format!(
        "SELECT {SELECT_COLS} FROM deployment ORDER BY started_at DESC LIMIT ?"
    ))
    .bind(limit as i64)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(row_to_deployment).collect())
}

pub async fn get_non_terminal(db: &Db) -> Result<Vec<Deployment>> {
    let rows = sqlx::query_as::<_, DeploymentRow>(&format!(
        "SELECT {SELECT_COLS} FROM deployment
             WHERE state NOT IN ('Live', 'Stopped', 'Failed', 'Pruning')"
    ))
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(row_to_deployment).collect())
}

pub async fn list_terminal_last_24h(db: &Db, limit: usize) -> Result<Vec<Deployment>> {
    let since = chrono::Utc::now() - chrono::Duration::hours(24);
    let rows = sqlx::query_as::<_, DeploymentRow>(&format!(
        "SELECT {SELECT_COLS} FROM deployment
             WHERE state IN ('Live', 'Failed')
             AND started_at > ?
             ORDER BY started_at DESC LIMIT ?"
    ))
    .bind(since)
    .bind(limit as i64)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(row_to_deployment).collect())
}

pub async fn stats_last_24h(db: &Db) -> Result<(u64, u64, u64)> {
    let since = chrono::Utc::now() - chrono::Duration::hours(24);
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM deployment WHERE started_at > ?")
        .bind(since)
        .fetch_one(db)
        .await
        .unwrap_or(0);
    let successful: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployment WHERE started_at > ? AND state = 'Live'",
    )
    .bind(since)
    .fetch_one(db)
    .await
    .unwrap_or(0);
    let failed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM deployment WHERE started_at > ? AND state = 'Failed'",
    )
    .bind(since)
    .fetch_one(db)
    .await
    .unwrap_or(0);
    Ok((total as u64, successful as u64, failed as u64))
}
