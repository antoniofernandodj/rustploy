use super::{AppState, Bincode};
use axum::{
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use shared::{Command, Response as RpResponse};

pub fn build(state: AppState) -> Router {
    Router::new()
        .route("/rpc", post(rpc_handler))
        .route("/stream", get(super::stream::handler))
        .route("/health", get(health))
        .with_state(state)
}

async fn health() -> impl IntoResponse {
    axum::Json(serde_json::json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}

async fn rpc_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
    Bincode(cmd): Bincode<Command>,
) -> impl IntoResponse {
    Bincode(dispatch(state, cmd).await)
}

async fn dispatch(state: AppState, cmd: Command) -> RpResponse {
    use super::handlers;
    match cmd {
        Command::Ping => handlers::ping::handle(state).await,
        Command::DaemonStatus => handlers::daemon_status::handle(state).await,
        Command::ProjectCreate { name, description } => handlers::project_create::handle(state, name, description).await,
        Command::ProjectList => handlers::project_list::handle(state).await,
        Command::ProjectDelete { id } => handlers::project_delete::handle(state, id).await,
        Command::ServiceCreate(spec) => handlers::service_create::handle(state, spec).await,
        Command::ServiceList { project_id } => handlers::service_list::handle(state, project_id).await,
        Command::ServiceGet { id } => handlers::service_get::handle(state, id).await,
        Command::ServiceUpdate { id, spec } => handlers::service_update::handle(state, id, spec).await,
        Command::ServiceDelete { id } => handlers::service_delete::handle(state, id).await,
        Command::DeployStart { service_id } => handlers::deploy_start::handle(state, service_id).await,
        Command::DeployAbort { deployment_id } => handlers::deploy_abort::handle(state, deployment_id).await,
        Command::DeployHistory { service_id, limit } => handlers::deploy_history::handle(state, service_id, limit).await,
        Command::DeployRollback { service_id } => handlers::deploy_rollback::handle(state, service_id).await,
        _ => RpResponse::err("NotImplemented", "command not yet implemented"),
    }
}
