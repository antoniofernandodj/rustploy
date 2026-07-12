use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState, job_run_id: String) -> RpResponse {
    match crate::db::job_log::get_for_run(&state.db, &job_run_id).await {
        Ok(lines) => RpResponse::JobLogs(lines),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
