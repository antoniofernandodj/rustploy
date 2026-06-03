use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState, service_id: String) -> RpResponse {
    match crate::db::deployments::list_for_service(&state.db, &service_id, 10).await {
        Ok(history) => {
            let prev = history
                .iter()
                .skip(1)
                .find(|d| d.state == shared::DeployState::Live);
            match prev {
                Some(d) => RpResponse::Deployment(d.clone()),
                None => RpResponse::err("NotFound", "no previous successful deploy"),
            }
        }
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
