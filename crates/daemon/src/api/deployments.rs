use super::{AppState, Bincode};
use crate::deploy::executor::DeployExecutor;
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use shared::{Command, DeployState, Response, ServiceStatus};
use std::sync::Arc;

pub async fn start(
    State(state): State<AppState>,
    Bincode(cmd): Bincode<Command>,
) -> impl IntoResponse {
    let Command::DeployStart { service_id } = cmd else {
        return Bincode(Response::err("InvalidCommand", "expected DeployStart")).into_response();
    };

    let svc = match crate::db::services::get(&state.db, &service_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Bincode(Response::err("NotFound", "service not found")).into_response()
        }
        Err(e) => return Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    };

    if matches!(svc.status, ServiceStatus::Deploying) {
        return Bincode(Response::err(
            "ServiceAlreadyDeploying",
            "a deploy is already in progress",
        ))
        .into_response();
    }

    let image = match &svc.spec.source {
        shared::ServiceSource::Registry { image } => image.clone(),
        shared::ServiceSource::Git(_) => format!("rp_{}", svc.spec.name),
    };

    let dep = match crate::db::deployments::create(&state.db, &service_id, &image).await {
        Ok(d) => d,
        Err(e) => return Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    };

    let executor = Arc::new(DeployExecutor {
        db: state.db.clone(),
        docker: state.docker.clone(),
        ingress: state.ingress.clone(),
        bus: state.bus.clone(),
        secrets: state.secrets.clone(),
        db_path: state.db_path.clone(),
        drain_secs: state.drain_secs,
    });

    let dep_id = dep.id.clone();
    tokio::spawn(async move { executor.run(dep_id).await });

    Bincode(Response::Deployment(dep)).into_response()
}

pub async fn abort(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let dep = match crate::db::deployments::get(&state.db, &id).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return Bincode(Response::err("NotFound", "deployment not found")).into_response()
        }
        Err(e) => return Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    };

    if dep.state.is_terminal() {
        return Bincode(Response::err("InvalidState", "deployment already finished")).into_response();
    }

    match crate::db::deployments::transition(
        &state.db,
        &id,
        &dep.state,
        DeployState::RollingBack,
        Some("aborted by user".into()),
    )
    .await
    {
        Ok(d) => Bincode(Response::Deployment(d)).into_response(),
        Err(e) => Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    }
}

pub async fn rollback(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Find the last successful deployment and trigger a new deploy from it
    let history = match crate::db::deployments::list_for_service(&state.db, &id, 10).await {
        Ok(h) => h,
        Err(e) => return Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    };

    let previous = history
        .iter()
        .skip(1)
        .find(|d| d.state == DeployState::Live);

    match previous {
        Some(prev) => Bincode(Response::Deployment(prev.clone())).into_response(),
        None => {
            Bincode(Response::err("NotFound", "no previous successful deployment to roll back to"))
                .into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    limit: Option<usize>,
}

pub async fn history(
    State(state): State<AppState>,
    Path(service_id): Path<String>,
    Query(q): Query<HistoryQuery>,
) -> impl IntoResponse {
    let limit = q.limit.unwrap_or(10).min(100);
    match crate::db::deployments::list_for_service(&state.db, &service_id, limit).await {
        Ok(deps) => Bincode(Response::Deployments(deps)).into_response(),
        Err(e) => Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    }
}
