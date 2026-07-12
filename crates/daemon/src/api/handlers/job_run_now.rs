use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState, id: String) -> RpResponse {
    let job = match crate::db::job::get(&state.db, &id).await {
        Ok(Some(j)) => j,
        Ok(None) => return RpResponse::err("NotFound", "job not found"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    match crate::jobs::runner::spawn(&state, job).await {
        Ok(run) => RpResponse::JobRun(run),
        Err(e) => RpResponse::err("JobRunError", e.to_string()),
    }
}
