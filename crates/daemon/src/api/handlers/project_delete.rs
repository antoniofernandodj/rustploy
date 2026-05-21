use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState, id: String) -> RpResponse {
    match crate::db::projects::delete(&state.db, &id).await {
        Ok(true) => RpResponse::Ok,
        Ok(false) => RpResponse::err("NotFound", "project not found"),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
