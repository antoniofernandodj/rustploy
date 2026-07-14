use crate::{api::AppState, db::webhook_tokens};
use shared::Response as RpResponse;

pub async fn handle(state: AppState, service_id: String) -> RpResponse {
    let token = match webhook_tokens::get(&state.db, &service_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return RpResponse::WebhookUrl(None),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    RpResponse::WebhookUrl(Some(build_url(&state, &service_id, &token)))
}

/// `{public_base_url}/webhook/{service_id}/{token}` — servido pelo listener da
/// API (mesma porta), então a base é a da própria API (`AppState::public_base_url`).
pub fn build_url(state: &AppState, service_id: &str, token: &str) -> String {
    format!(
        "{}/webhook/{}/{}",
        state.public_base_url(),
        service_id,
        token
    )
}
