use crate::api::AppState;
use shared::{Response as RpResponse, ServiceSpec};

pub async fn handle(state: AppState, spec: ServiceSpec) -> RpResponse {
    match crate::db::services::create(&state.db, spec).await {
        Ok(s) => RpResponse::Service(s),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
