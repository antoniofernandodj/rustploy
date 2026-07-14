use crate::api::AppState;
use shared::{Recurrence, Response as RpResponse};

pub async fn handle(
    state: AppState,
    project_id: String,
    trigger_service_id: String,
    name: String,
    compose: String,
    main_service: String,
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

    match crate::db::job::create(
        &state.db,
        &project_id,
        trigger_service_id.as_deref(),
        &name,
        &compose,
        &main_service,
        recurrence,
    )
    .await
    {
        Ok(job) => RpResponse::Job(job),
        Err(e) => RpResponse::err("DatabaseError", super::humanize_db_error(&e, "job")),
    }
}
