use super::{AppState, Bincode};
use axum::{
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use shared::{Command, Response as RpResponse};
use tracing::info;

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

    let cmd_name = match &cmd {
        Command::Ping => "Ping",
        Command::DaemonStatus => "DaemonStatus",
        Command::ProjectCreate { .. } => "ProjectCreate",
        Command::ProjectList => "ProjectList",
        Command::ProjectDelete { .. } => "ProjectDelete",
        Command::ProjectEnvSet { .. } => "ProjectEnvSet",
        Command::ServiceCreate(_) => "ServiceCreate",
        Command::ServiceList { .. } => "ServiceList",
        Command::ServiceGet { .. } => "ServiceGet",
        Command::ServiceUpdate { .. } => "ServiceUpdate",
        Command::ServiceDelete { .. } => "ServiceDelete",
        Command::DeployStart { .. } => "DeployStart",
        Command::DeployAbort { .. } => "DeployAbort",
        Command::DeployHistory { .. } => "DeployHistory",
        Command::DeployRollback { .. } => "DeployRollback",
        _ => "Unknown",
    };
    info!(command = cmd_name, "→ RPC recebido");

    let resp = match cmd {
        Command::Ping => handlers::ping::handle(state).await,
        Command::DaemonStatus => handlers::daemon_status::handle(state).await,
        Command::ProjectCreate { name, description } => handlers::project_create::handle(state, name, description).await,
        Command::ProjectList => handlers::project_list::handle(state).await,
        Command::ProjectDelete { id } => handlers::project_delete::handle(state, id).await,
        Command::ProjectEnvSet { project_id, env_vars } => handlers::project_env_set::handle(state, project_id, env_vars).await,
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
    };

    let ok = !matches!(resp, RpResponse::Err { .. });
    info!(command = cmd_name, ok, "← RPC respondido");
    resp
}
