use crate::{api::AppState, db::daemon_settings};
use shared::Response as RpResponse;
use tracing::{error, info};

pub async fn handle(
    state: AppState,
    webhook_base_url: Option<String>,
    acme_email: Option<String>,
) -> RpResponse {
    if let Err(e) = save_optional(
        &state,
        daemon_settings::KEY_WEBHOOK_BASE_URL,
        webhook_base_url,
    )
    .await
    {
        return e;
    }

    if let Err(e) = save_optional(&state, daemon_settings::KEY_ACME_EMAIL, acme_email).await {
        return e;
    }

    info!("daemon settings saved");
    RpResponse::Ok
}

async fn save_optional(
    state: &AppState,
    key: &str,
    value: Option<String>,
) -> Result<(), RpResponse> {
    match value {
        Some(v) if !v.trim().is_empty() => {
            daemon_settings::set(&state.db, key, v.trim()).await.map_err(|e| {
                error!(error = %e, key, "failed to save daemon setting");
                RpResponse::err("DatabaseError", e.to_string())
            })
        }
        _ => daemon_settings::delete(&state.db, key).await.map_err(|e| {
            error!(error = %e, key, "failed to delete daemon setting");
            RpResponse::err("DatabaseError", e.to_string())
        }),
    }
}
