use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState) -> RpResponse {
    match crate::db::projects::list(&state.db).await {
        Ok(ps) => RpResponse::Projects(ps),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
