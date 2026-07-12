use crate::api::AppState;
use shared::{Recurrence, Response as RpResponse};

#[allow(clippy::too_many_arguments)]
pub async fn handle(
    state: AppState,
    id: String,
    name: String,
    compose: String,
    main_service: String,
    enabled: bool,
    recurrence: Option<Recurrence>,
) -> RpResponse {
    match crate::db::job::update(&state.db, &id, &name, &compose, &main_service, enabled, recurrence)
        .await
    {
        Ok(Some(job)) => RpResponse::Job(job),
        Ok(None) => RpResponse::err("NotFound", "job not found"),
        Err(e) => RpResponse::err("DatabaseError", super::humanize_db_error(&e, "job")),
    }
}
