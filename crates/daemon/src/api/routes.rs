use super::AppState;
use shared::{Command, Response as RpResponse};
use tracing::info;

pub async fn dispatch(state: AppState, cmd: Command) -> RpResponse {
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
        Command::ServiceStop { .. } => "ServiceStop",
        Command::ServiceReload { .. } => "ServiceReload",
        Command::LogsGet { .. } => "LogsGet",
        Command::RecentDeployments { .. } => "RecentDeployments",
        Command::GetBuildLogs { .. } => "GetBuildLogs",
        Command::GetWebhookUrl { .. } => "GetWebhookUrl",
        Command::RegenerateWebhookToken { .. } => "RegenerateWebhookToken",
        Command::GetDaemonSettings => "GetDaemonSettings",
        Command::SetDaemonSettings { .. } => "SetDaemonSettings",
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
        Command::ServiceStop { service_id } => handlers::service_stop::handle(state, service_id).await,
        Command::ServiceReload { service_id } => handlers::service_reload::handle(state, service_id).await,
        Command::LogsGet { service_id, tail } => handlers::logs_get::handle(state, service_id, tail).await,
        Command::RecentDeployments { limit } => handlers::recent_deployments::handle(state, limit).await,
        Command::GetBuildLogs { deployment_id } => handlers::get_build_logs::handle(state, deployment_id).await,
        Command::GetWebhookUrl { service_id } => handlers::get_webhook_url::handle(state, service_id).await,
        Command::RegenerateWebhookToken { service_id } => handlers::regenerate_webhook_token::handle(state, service_id).await,
        Command::GetDaemonSettings => handlers::get_daemon_settings::handle(state).await,
        Command::SetDaemonSettings { webhook_base_url } => handlers::set_daemon_settings::handle(state, webhook_base_url).await,
        _ => RpResponse::err("NotImplemented", "command not yet implemented"),
    };

    let ok = !matches!(resp, RpResponse::Err { .. });
    info!(command = cmd_name, ok, "← RPC respondido");
    resp
}
