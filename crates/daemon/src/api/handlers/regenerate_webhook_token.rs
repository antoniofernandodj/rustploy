use crate::{api::AppState, db::webhook_tokens};
use shared::Response as RpResponse;
use tracing::error;

pub async fn handle(state: AppState, service_id: String) -> RpResponse {
    let token = webhook_tokens::generate_token();

    if let Err(e) = webhook_tokens::upsert(&state.db, &service_id, &token).await {
        error!(service_id = %service_id, error = %e, "failed to upsert webhook token");
        return RpResponse::err("DatabaseError", e.to_string());
    }

    let url = super::get_webhook_url::build_url(&state, &service_id, &token).await;
    RpResponse::WebhookUrl(Some(url))
}
