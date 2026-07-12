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
    if crate::db::services::get(&state.db, &trigger_service_id)
        .await
        .ok()
        .flatten()
        .is_none()
    {
        return RpResponse::err("NotFound", "serviço gatilho não encontrado");
    }

    match crate::db::job::create(
        &state.db,
        &project_id,
        &trigger_service_id,
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
