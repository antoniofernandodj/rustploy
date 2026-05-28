use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState, deployment_id: String) -> RpResponse {
    match crate::db::build_logs::get_for_deployment(&state.db, &deployment_id).await {
        Ok(lines) => RpResponse::BuildLogs(lines),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
