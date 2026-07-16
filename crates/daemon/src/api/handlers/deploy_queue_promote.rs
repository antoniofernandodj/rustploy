use crate::api::AppState;
use shared::{Event, Response as RpResponse};

/// Move um deploy enfileirado para o início da fila ("furar fila"). No-op se o
/// id não estiver na fila (já rodando/terminado).
pub async fn handle(state: AppState, deployment_id: String) -> RpResponse {
    state.deploy_queue.promote(&deployment_id);
    state.bus.publish(Event::DeployQueueChanged);
    RpResponse::Ok
}
