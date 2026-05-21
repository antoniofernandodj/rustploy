use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState, id: String) -> RpResponse {
    if let Ok(Some(svc)) = crate::db::services::get(&state.db, &id).await {
        state.ingress.remove_route(&svc.spec.domain);
    }
    match crate::db::services::delete(&state.db, &id).await {
        Ok(true) => RpResponse::Ok,
        Ok(false) => RpResponse::err("NotFound", "service not found"),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
