use crate::{api::AppState, docker::containers};
use chrono::Utc;
use shared::{DeployState, Event, Response as RpResponse, ServiceStatus};

pub async fn handle(state: AppState, service_id: String) -> RpResponse {
    let svc = match crate::db::services::get(&state.db, &service_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return RpResponse::err("NotFound", "serviço não encontrado"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    let container_id = match &svc.live_container_id {
        Some(id) => id.clone(),
        None => return RpResponse::err("NotRunning", "serviço não possui container ativo"),
    };

    if let Err(e) = containers::stop_graceful(&state.docker.inner, &container_id, 10).await {
        return RpResponse::err("DockerError", format!("falha ao parar: {e}"));
    }

    if let Err(e) = containers::start(&state.docker.inner, &container_id).await {
        return RpResponse::err("DockerError", format!("falha ao iniciar: {e}"));
    }

    if let Err(e) = crate::db::services::update_status(
        &state.db,
        &service_id,
        &ServiceStatus::Running,
        Some(&container_id),
    )
    .await
    {
        return RpResponse::err("DatabaseError", e.to_string());
    }

    // Se o deployment estava Stopped, volta para Live.
    if let Ok(history) =
        crate::db::deployments::list_for_service(&state.db, &service_id, 1).await
    {
        if let Some(dep) = history.into_iter().find(|d| d.state == DeployState::Stopped) {
            let _ = crate::db::deployments::transition(
                &state.db,
                &dep.id,
                &DeployState::Stopped,
                DeployState::Live,
                None,
            )
            .await;
            state.bus.publish(Event::DeployStateChanged {
                deployment_id: dep.id,
                service_id: service_id.clone(),
                state: DeployState::Live,
                timestamp: Utc::now(),
                message: None,
            });
        }
    }

    state.bus.publish(Event::ServiceStatusChanged {
        service_id,
        status: ServiceStatus::Running,
    });

    RpResponse::Ok
}
