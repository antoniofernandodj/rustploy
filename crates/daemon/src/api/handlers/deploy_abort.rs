use crate::api::AppState;
use shared::{Event, Response as RpResponse, ServiceStatus};

pub async fn handle(state: AppState, deployment_id: String) -> RpResponse {
    let dep = match crate::db::deployments::get(&state.db, &deployment_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return RpResponse::err("NotFound", "deployment not found"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };
    if dep.state.is_terminal() {
        return RpResponse::err("InvalidState", "deployment already finished");
    }

    // Estava só esperando na fila (nunca rodou): remove da fila e devolve o
    // serviço ao repouso (Stopped) — não há task de executor para abortar.
    if state.deploy_queue.remove_queued(&deployment_id) {
        let _ = crate::db::services::update_status(
            &state.db,
            &dep.service_id,
            &ServiceStatus::Stopped,
            None,
        )
        .await;
        state.bus.publish(Event::ServiceStatusChanged {
            service_id: dep.service_id.clone(),
            status: ServiceStatus::Stopped,
        });
        state.bus.publish(Event::DeployQueueChanged);
    } else if let Ok(mut map) = state.active_deploys.lock() {
        // Estava rodando: aborta a task do executor imediatamente (interrompe o
        // stream do Docker build).
        if let Some(handle) = map.remove(&deployment_id) {
            handle.abort();
        }
    }

    match crate::db::deployments::transition(
        &state.db,
        &deployment_id,
        &dep.state,
        shared::DeployState::Failed,
        Some("aborted by user".into()),
    )
    .await
    {
        Ok(d) => RpResponse::Deployment(d),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
