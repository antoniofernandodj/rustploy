use crate::api::AppState;
use shared::{Event, Response as RpResponse};

/// Pausa (`true`) ou retoma (`false`) a fila global. Pausada, o worker não puxa
/// o próximo deploy; o que já estiver rodando segue até o fim.
pub async fn handle(state: AppState, paused: bool) -> RpResponse {
    state.deploy_queue.set_paused(paused);
    state.bus.publish(Event::DeployQueueChanged);
    RpResponse::Ok
}
