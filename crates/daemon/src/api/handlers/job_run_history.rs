use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState, job_id: String, limit: usize) -> RpResponse {
    match crate::db::job_run::list_for_job(&state.db, &job_id, limit).await {
        Ok(runs) => RpResponse::JobRuns(runs),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
