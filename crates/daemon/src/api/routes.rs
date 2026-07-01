use super::AppState;
use shared::{Command, Response as RpResponse};
use tracing::info;

pub async fn dispatch(state: AppState, cmd: Command) -> RpResponse {
    use super::handlers;

    let cmd_name = match &cmd {
        Command::Ping => "Ping",
        Command::DaemonStatus => "DaemonStatus",
        Command::DeployEngineStatus => "DeployEngineStatus",
        Command::ProjectCreate { .. } => "ProjectCreate",
        Command::ProjectList => "ProjectList",
        Command::ProjectDelete { .. } => "ProjectDelete",
        Command::ProjectUpdate { .. } => "ProjectUpdate",
        Command::ProjectEnvSet { .. } => "ProjectEnvSet",
        Command::ServiceCreate(_) => "ServiceCreate",
        Command::ServiceList { .. } => "ServiceList",
        Command::ServiceGet { .. } => "ServiceGet",
        Command::ServiceUpdate { .. } => "ServiceUpdate",
        Command::ServiceDelete { .. } => "ServiceDelete",
        Command::DeployStart { .. } => "DeployStart",
        Command::DeployAbort { .. } => "DeployAbort",
        Command::DeployDelete { .. } => "DeployDelete",
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
        Command::SecretSet { .. } => "SecretSet",
        Command::SecretDelete { .. } => "SecretDelete",
        Command::SecretList { .. } => "SecretList",
        Command::ManifestApply { .. } => "ManifestApply",
        Command::ManifestExport { .. } => "ManifestExport",
        Command::PruneContainers => "PruneContainers",
        Command::PruneVolumes => "PruneVolumes",
        Command::PruneImages => "PruneImages",
        Command::PruneBuildCache => "PruneBuildCache",
        Command::PruneNetworks => "PruneNetworks",
        Command::DockerImages => "DockerImages",
        Command::DockerVolumes => "DockerVolumes",
        Command::DockerNetworks => "DockerNetworks",
        Command::StopAllManaged => "StopAllManaged",
        Command::EnvBackupList => "EnvBackupList",
        Command::EnvBackupRestore { .. } => "EnvBackupRestore",
        Command::GitProviderList => "GitProviderList",
        Command::GitProviderCreate { .. } => "GitProviderCreate",
        Command::GitProviderDelete { .. } => "GitProviderDelete",
        Command::GitOAuthStart { .. } => "GitOAuthStart",
        Command::GitRepoList { .. } => "GitRepoList",
        Command::GitBranchList { .. } => "GitBranchList",
        _ => "Unknown",
    };
    info!(
        command = cmd_name, "→ Request"
    );

    let resp = match cmd {
        Command::Ping => handlers::ping::handle(state).await,
        Command::DaemonStatus => handlers::daemon_status::handle(state).await,
        Command::DeployEngineStatus => handlers::deploy_engine_status::handle(state).await,
        Command::PruneContainers => handlers::docker_prune::prune_containers(state).await,
        Command::PruneVolumes => handlers::docker_prune::prune_volumes(state).await,
        Command::PruneImages => handlers::docker_prune::prune_images(state).await,
        Command::PruneBuildCache => handlers::docker_prune::prune_build_cache(state).await,
        Command::PruneNetworks => handlers::docker_prune::prune_networks(state).await,
        Command::DockerImages => handlers::docker_inventory::list_images(state).await,
        Command::DockerVolumes => handlers::docker_inventory::list_volumes(state).await,
        Command::DockerNetworks => handlers::docker_inventory::list_networks(state).await,
        Command::StopAllManaged => handlers::docker_inventory::stop_all_managed(state).await,
        Command::EnvBackupList => handlers::env_backup::list(state).await,
        Command::EnvBackupRestore { snapshot } => handlers::env_backup::restore(state, snapshot).await,
        Command::ProjectCreate { name, description } => {
            handlers::project_create::handle(state, name, description).await
        }
        Command::ProjectList => handlers::project_list::handle(state).await,
        Command::ProjectDelete { id } => handlers::project_delete::handle(state, id).await,
        Command::ProjectUpdate {
            id,
            name,
            description,
        } => handlers::project_update::handle(state, id, name, description).await,
        Command::ProjectEnvSet {
            project_id,
            env_vars,
        } => handlers::project_env_set::handle(state, project_id, env_vars).await,
        Command::ServiceCreate(spec) => handlers::service_create::handle(state, spec).await,
        Command::ServiceList { project_id } => {
            handlers::service_list::handle(state, project_id).await
        }
        Command::ServiceGet { id } => handlers::service_get::handle(state, id).await,
        Command::ServiceUpdate { id, spec } => {
            handlers::service_update::handle(state, id, spec).await
        }
        Command::ServiceDelete { id } => handlers::service_delete::handle(state, id).await,
        Command::DeployStart { service_id } => {
            handlers::deploy_start::handle(state, service_id).await
        }
        Command::DeployAbort { deployment_id } => {
            handlers::deploy_abort::handle(state, deployment_id).await
        }
        Command::DeployDelete { deployment_id } => {
            handlers::deploy_delete::handle(state, deployment_id).await
        }
        Command::DeployHistory { service_id, limit } => {
            handlers::deploy_history::handle(state, service_id, limit).await
        }
        Command::DeployRollback { service_id } => {
            handlers::deploy_rollback::handle(state, service_id).await
        }
        Command::ServiceStop { service_id } => {
            handlers::service_stop::handle(state, service_id).await
        }
        Command::ServiceReload { service_id } => {
            handlers::service_reload::handle(state, service_id).await
        }
        Command::LogsGet { service_id, tail } => {
            handlers::logs_get::handle(state, service_id, tail).await
        }
        Command::RecentDeployments { limit } => {
            handlers::recent_deployments::handle(state, limit).await
        }
        Command::GetBuildLogs { deployment_id } => {
            handlers::get_build_logs::handle(state, deployment_id).await
        }
        Command::GetWebhookUrl { service_id } => {
            handlers::get_webhook_url::handle(state, service_id).await
        }
        Command::RegenerateWebhookToken { service_id } => {
            handlers::regenerate_webhook_token::handle(state, service_id).await
        }
        Command::GetDaemonSettings => handlers::get_daemon_settings::handle(state).await,
        Command::SetDaemonSettings { webhook_base_url, acme_email } => {
            handlers::set_daemon_settings::handle(state, webhook_base_url, acme_email).await
        }
        Command::SecretSet {
            project_id,
            name,
            value,
        } => handlers::secret_set::handle(state, project_id, name, value).await,
        Command::SecretDelete { project_id, name } => {
            handlers::secret_delete::handle(state, project_id, name).await
        }
        Command::SecretList { project_id } => {
            handlers::secret_list::handle(state, project_id).await
        }
        Command::ManifestApply {
            manifests,
            prune,
            deploy,
        } => handlers::manifest_apply::handle(state, manifests, prune, deploy).await,
        Command::ManifestExport { project_id } => {
            handlers::manifest_export::handle(state, project_id).await
        }
        Command::GitProviderList => handlers::git_provider_list::handle(state).await,
        Command::GitProviderCreate {
            kind,
            name,
            base_url,
            auth_mode,
            oauth_client_id,
            oauth_client_secret,
            pat,
        } => {
            handlers::git_provider_create::handle(
                state,
                kind,
                name,
                base_url,
                auth_mode,
                oauth_client_id,
                oauth_client_secret,
                pat,
            )
            .await
        }
        Command::GitProviderDelete { id } => handlers::git_provider_delete::handle(state, id).await,
        Command::GitOAuthStart { provider_id } => {
            handlers::git_oauth_start::handle(state, provider_id).await
        }
        Command::GitRepoList { provider_id } => {
            handlers::git_repo_list::handle(state, provider_id).await
        }
        Command::GitBranchList {
            provider_id,
            repo_full_name,
        } => handlers::git_branch_list::handle(state, provider_id, repo_full_name).await,
        _ => RpResponse::err("NotImplemented", "command not yet implemented"),
    };

    let ok = !matches!(resp, RpResponse::Err { .. });
    info!(
        ok,
        command = cmd_name,
        "← Response"
    );
    resp
}
