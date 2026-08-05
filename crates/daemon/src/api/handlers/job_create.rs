use crate::api::AppState;
use shared::{EnvComment, EnvVar, JobGitSource, Recurrence, Response as RpResponse};

#[allow(clippy::too_many_arguments)]
pub async fn handle(
    state: AppState,
    project_id: String,
    trigger_service_id: String,
    name: String,
    compose: String,
    git_source: Option<JobGitSource>,
    main_service: String,
    env_vars: Vec<EnvVar>,
    env_comments: Vec<EnvComment>,
    recurrence: Option<Recurrence>,
) -> RpResponse {
    // String vazia (sentinel no wire, ver db/job.rs) = job autônomo, sem
    // serviço gatilho — nada a validar nesse caso.
    let trigger_service_id = if trigger_service_id.is_empty() {
        None
    } else {
        Some(trigger_service_id)
    };

    if let Some(sid) = &trigger_service_id {
        if crate::db::services::get(&state.db, sid).await.ok().flatten().is_none() {
            return RpResponse::err("NotFound", "serviço gatilho não encontrado");
        }
    }

    if let Some(git) = &git_source {
        if git.url.trim().is_empty() || git.branch.trim().is_empty() {
            return RpResponse::err("InvalidInput", "URL e branch do repositório são obrigatórios");
        }
    }

    match crate::db::job::create(
        &state.db,
        &project_id,
        trigger_service_id.as_deref(),
        &name,
        &compose,
        git_source.as_ref(),
        &main_service,
        &env_vars,
        &env_comments,
        recurrence,
    )
    .await
    {
        Ok(job) => RpResponse::Job(job),
        Err(e) => RpResponse::err("DatabaseError", super::humanize_db_error(&e, "job")),
    }
}
