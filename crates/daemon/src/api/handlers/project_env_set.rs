use crate::api::AppState;
use shared::{EnvComment, EnvVar, Response as RpResponse};
use tracing::info;

pub async fn handle(
    state: AppState,
    project_id: String,
    env_vars: Vec<EnvVar>,
    env_comments: Vec<EnvComment>,
) -> RpResponse {
    info!(
        project_id = %project_id,
        count = env_vars.len(),
        comments = env_comments.len(),
        "project_env_set: atualizando env vars do projeto"
    );
    match crate::db::projects::update_env_vars(&state.db, &project_id, env_vars, env_comments).await
    {
        Ok(Some(project)) => {
            info!(project_id = %project.id, "project_env_set: env vars atualizadas");
            RpResponse::Project(project)
        }
        Ok(None) => RpResponse::err("NotFound", "project not found"),
        Err(e) => {
            tracing::error!(error = %e, "project_env_set: falha ao atualizar env vars");
            RpResponse::err("DatabaseError", e.to_string())
        }
    }
}
