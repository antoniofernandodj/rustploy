use crate::api::AppState;
use shared::{Event, Response as RpResponse};

/// Reordena a fila para a ordem dada (ids de deployment). Ids desconhecidos são
/// ignorados; enfileirados omitidos vão ao fim preservando a ordem relativa.
pub async fn handle(state: AppState, order: Vec<String>) -> RpResponse {
    state.deploy_queue.reorder(&order);
    state.bus.publish(Event::DeployQueueChanged);
    RpResponse::Ok
}
