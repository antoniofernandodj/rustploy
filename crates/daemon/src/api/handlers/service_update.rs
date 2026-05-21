use crate::api::AppState;
use shared::{Response as RpResponse, ServiceSpec};

pub async fn handle(state: AppState, id: String, spec: ServiceSpec) -> RpResponse {
    match crate::db::services::update_spec(&state.db, &id, spec).await {
        Ok(Some(s)) => RpResponse::Service(s),
        Ok(None) => RpResponse::err("NotFound", "service not found"),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
