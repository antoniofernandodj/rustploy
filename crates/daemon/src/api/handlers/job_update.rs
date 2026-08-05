use crate::api::AppState;
use shared::{EnvComment, EnvVar, JobGitSource, Recurrence, Response as RpResponse};

#[allow(clippy::too_many_arguments)]
pub async fn handle(
    state: AppState,
    id: String,
    name: String,
    compose: String,
    git_source: Option<JobGitSource>,
    main_service: String,
    env_vars: Vec<EnvVar>,
    env_comments: Vec<EnvComment>,
    enabled: bool,
    recurrence: Option<Recurrence>,
) -> RpResponse {
    if let Some(git) = &git_source {
        if git.url.trim().is_empty() || git.branch.trim().is_empty() {
            return RpResponse::err("InvalidInput", "URL e branch do repositório são obrigatórios");
        }
    }

    match crate::db::job::update(
        &state.db,
        &id,
        &name,
        &compose,
        git_source.as_ref(),
        &main_service,
        &env_vars,
        &env_comments,
        enabled,
        recurrence,
    )
    .await
    {
        Ok(Some(job)) => RpResponse::Job(job),
        Ok(None) => RpResponse::err("NotFound", "job not found"),
        Err(e) => RpResponse::err("DatabaseError", super::humanize_db_error(&e, "job")),
    }
}
