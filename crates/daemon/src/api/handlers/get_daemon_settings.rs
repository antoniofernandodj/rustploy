use crate::{api::AppState, db::daemon_settings};
use shared::Response as RpResponse;

pub async fn handle(state: AppState) -> RpResponse {
    let webhook_base_url = daemon_settings::get(&state.db, daemon_settings::KEY_WEBHOOK_BASE_URL)
        .await
        .ok()
        .flatten();

    RpResponse::DaemonSettings { webhook_base_url }
}
