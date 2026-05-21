use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState, name: String, description: Option<String>) -> RpResponse {
    match crate::db::projects::create(&state.db, name, description).await {
        Ok(p) => RpResponse::Project(p),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
