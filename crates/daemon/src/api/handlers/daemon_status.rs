use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState) -> RpResponse {
    let services = crate::db::services::get_running(&state.db)
        .await
        .unwrap_or_default();

    let _total: Vec<_> = state
        .db
        .query("SELECT count() FROM service GROUP ALL")
        .await
        .ok()
        .and_then(|mut r| r.take::<Vec<serde_json::Value>>(0).ok())
        .unwrap_or_default();

    RpResponse::DaemonStatus(shared::DaemonStatus {
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_secs: state.started_at.elapsed().as_secs(),
        services_running: services.len(),
        services_total: 0,
    })
}
