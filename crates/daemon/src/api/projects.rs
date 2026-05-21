use super::{AppState, Bincode};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use shared::{Command, Response};

pub async fn create(
    State(state): State<AppState>,
    Bincode(cmd): Bincode<Command>,
) -> impl IntoResponse {
    let Command::ProjectCreate { name, description } = cmd else {
        return Bincode(Response::err("InvalidCommand", "expected ProjectCreate")).into_response();
    };

    match crate::db::projects::create(&state.db, name, description).await {
        Ok(project) => Bincode(Response::Project(project)).into_response(),
        Err(e) => Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    }
}

pub async fn list(State(state): State<AppState>) -> impl IntoResponse {
    match crate::db::projects::list(&state.db).await {
        Ok(projects) => Bincode(Response::Projects(projects)).into_response(),
        Err(e) => Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    }
}

pub async fn remove(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match crate::db::projects::delete(&state.db, &id).await {
        Ok(true) => Bincode(Response::Ok).into_response(),
        Ok(false) => Bincode(Response::err("NotFound", "project not found")).into_response(),
        Err(e) => Bincode(Response::err("DatabaseError", e.to_string())).into_response(),
    }
}
