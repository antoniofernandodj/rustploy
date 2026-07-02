use super::{AppState, Bincode};
use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use shared::{Command, Response, ServiceSpec};

#[derive(Deserialize)]
pub struct ProjectQuery {
    project: String,
}

pub async fn create(
    State(state): State<AppState>,
    Bincode(cmd): Bincode<Command>,
) -> impl IntoResponse {
    let Command::ServiceCreate(spec) = cmd else {
        return Bincode(Response::err("InvalidCommand", "expected ServiceCreate")).into_response();
    };

    match crate::db::services::create(&state.db, spec).await {
        Ok(service) => Bincode(Response::Service(service)).into_response(),
        Err(e) => Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    }
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ProjectQuery>,
) -> impl IntoResponse {
    match crate::db::services::list(&state.db, &q.project).await {
        Ok(services) => Bincode(Response::Services(services)).into_response(),
        Err(e) => Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    }
}

pub async fn get_one(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match crate::db::services::get(&state.db, &id).await {
        Ok(Some(svc)) => Bincode(Response::Service(svc)).into_response(),
        Ok(None) => Bincode(Response::err("NotFound", "service not found")).into_response(),
        Err(e) => Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    }
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Bincode(cmd): Bincode<Command>,
) -> impl IntoResponse {
    let Command::ServiceUpdate { id: _, spec } = cmd else {
        return Bincode(Response::err("InvalidCommand", "expected ServiceUpdate")).into_response();
    };

    match crate::db::services::update_spec(&state.db, &id, spec).await {
        Ok(Some(svc)) => Bincode(Response::Service(svc)).into_response(),
        Ok(None) => Bincode(Response::err("NotFound", "service not found")).into_response(),
        Err(e) => Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    }
}

pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Remove ingress route if service has one
    if let Ok(Some(svc)) = crate::db::services::get(&state.db, &id).await {
        state.ingress.remove_domains(&svc.spec);
    }

    match crate::db::services::delete(&state.db, &id).await {
        Ok(true) => Bincode(Response::Ok).into_response(),
        Ok(false) => Bincode(Response::err("NotFound", "service not found")).into_response(),
        Err(e) => Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    }
}
