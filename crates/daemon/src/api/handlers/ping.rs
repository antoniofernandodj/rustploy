use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState) -> RpResponse {
    RpResponse::Pong { uptime_secs: state.started_at.elapsed().as_secs() }
}
