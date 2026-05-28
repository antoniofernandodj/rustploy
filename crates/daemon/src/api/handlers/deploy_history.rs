use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState, service_id: String, limit: usize) -> RpResponse {
    match crate::db::deployments::list_for_service(&state.db, &service_id, limit).await {
        Ok(deps) => {
            let deps = super::reconcile::fix_stale_live(&state, deps).await;
            RpResponse::Deployments(deps)
        }
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
