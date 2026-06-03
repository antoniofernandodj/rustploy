use crate::{api::AppState, docker::containers};
use chrono::Utc;
use shared::{DeployState, Event, Response as RpResponse, ServiceStatus};

pub async fn handle(state: AppState, service_id: String) -> RpResponse {
    let svc = match crate::db::services::get(&state.db, &service_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return RpResponse::err("NotFound", "serviço não encontrado"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    // Para todas as instâncias do serviço (suporte a replicas).
    let all_ids = match containers::find_all_by_service_id(&state.docker.inner, &service_id).await {
        Ok(ids) => ids,
        Err(e) => return RpResponse::err("DockerError", e.to_string()),
    };

    if all_ids.is_empty() && svc.live_container_id.is_none() {
        return RpResponse::err("NotRunning", "serviço não possui containers ativos");
    }

    // Para cada container encontrado via label; se nenhum, cai no live_container_id.
    let ids_to_stop: Vec<String> = if all_ids.is_empty() {
        svc.live_container_id.clone().into_iter().collect()
    } else {
        all_ids
    };

    for cid in &ids_to_stop {
        if let Err(e) = containers::stop_graceful(&state.docker.inner, cid, 10).await {
            return RpResponse::err("DockerError", e.to_string());
        }
    }

    let primary_id = ids_to_stop.first().map(|s| s.as_str()).or(svc.live_container_id.as_deref());

    // Preserva o container_id no banco para que um Reload futuro possa encontrá-lo.
    if let Err(e) = crate::db::services::update_status(
        &state.db,
        &service_id,
        &ServiceStatus::Stopped,
        primary_id,
    )
    .await
    {
        return RpResponse::err("DatabaseError", e.to_string());
    }

    // Transiciona o deployment Live → Stopped e notifica o cliente.
    if let Ok(history) =
        crate::db::deployments::list_for_service(&state.db, &service_id, 1).await
    {
        if let Some(dep) = history.into_iter().find(|d| d.state == DeployState::Live) {
            let _ = crate::db::deployments::transition(
                &state.db,
                &dep.id,
                &DeployState::Live,
                DeployState::Stopped,
                None,
            )
            .await;
            state.bus.publish(Event::DeployStateChanged {
                deployment_id: dep.id,
                service_id: service_id.clone(),
                state: DeployState::Stopped,
                timestamp: Utc::now(),
                message: None,
            });
        }
    }

    state.bus.publish(Event::ServiceStatusChanged {
        service_id,
        status: ServiceStatus::Stopped,
    });

    RpResponse::Ok
}
