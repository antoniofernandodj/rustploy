use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState) -> RpResponse {
    let services = crate::db::services::get_running(&state.db)
        .await
        .unwrap_or_default();

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM service")
        .fetch_one(&*state.db)
        .await
        .unwrap_or(0);

    RpResponse::DaemonStatus(shared::DaemonStatus {
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_secs: state.started_at.elapsed().as_secs(),
        services_running: services.len(),
        services_total: total as usize,
    })
}
