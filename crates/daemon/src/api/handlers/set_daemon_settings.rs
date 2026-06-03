use crate::{api::AppState, db::daemon_settings};
use shared::Response as RpResponse;
use tracing::error;

pub async fn handle(state: AppState, webhook_base_url: Option<String>) -> RpResponse {
    match webhook_base_url {
        Some(url) if !url.trim().is_empty() => {
            if let Err(e) =
                daemon_settings::set(&state.db, daemon_settings::KEY_WEBHOOK_BASE_URL, url.trim())
                    .await
            {
                error!(error = %e, "failed to save daemon settings");
                return RpResponse::err("DatabaseError", e.to_string());
            }
        }
        _ => {
            if let Err(e) =
                daemon_settings::delete(&state.db, daemon_settings::KEY_WEBHOOK_BASE_URL).await
            {
                error!(error = %e, "failed to delete daemon setting");
                return RpResponse::err("DatabaseError", e.to_string());
            }
        }
    }

    RpResponse::Ok
}
