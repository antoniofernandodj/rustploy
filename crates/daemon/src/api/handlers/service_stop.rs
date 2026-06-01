use crate::{api::AppState, docker::containers};
use chrono::Utc;
use shared::{DeployState, Event, Response as RpResponse, ServiceSource, ServiceStatus};

pub async fn handle(state: AppState, service_id: String) -> RpResponse {
    let svc = match crate::db::services::get(&state.db, &service_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return RpResponse::err("NotFound", "serviço não encontrado"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    if let ServiceSource::Compose(compose) = &svc.spec.source {
        return stop_compose(&state, &service_id, &svc.spec.name, &compose.content).await;
    }

    let container_id = match &svc.live_container_id {
        Some(id) => id.clone(),
        None => return RpResponse::err("NotRunning", "serviço não possui container ativo"),
    };

    if let Err(e) = containers::stop_graceful(&state.docker.inner, &container_id, 10).await {
        return RpResponse::err("DockerError", e.to_string());
    }

    finish_stop(&state, &service_id, Some(&container_id)).await
}

async fn stop_compose(
    state: &AppState,
    service_id: &str,
    service_name: &str,
    content: &str,
) -> RpResponse {
    if let Err(e) =
        crate::docker::compose::compose_down(content, &format!("rp_{}", service_name)).await
    {
        return RpResponse::err("DockerError", e.to_string());
    }
    finish_stop(state, service_id, None).await
}

async fn finish_stop(state: &AppState, service_id: &str, container_id: Option<&str>) -> RpResponse {
    if let Err(e) = crate::db::services::update_status(
        &state.db,
        service_id,
        &ServiceStatus::Stopped,
        container_id,
    )
    .await
    {
        return RpResponse::err("DatabaseError", e.to_string());
    }

    if let Ok(history) =
        crate::db::deployments::list_for_service(&state.db, service_id, 1).await
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
                service_id: service_id.to_string(),
                state: DeployState::Stopped,
                timestamp: Utc::now(),
                message: None,
            });
        }
    }

    state.bus.publish(Event::ServiceStatusChanged {
        service_id: service_id.to_string(),
        status: ServiceStatus::Stopped,
    });

    RpResponse::Ok
}
