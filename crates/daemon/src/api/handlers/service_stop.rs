use crate::{api::AppState, docker::containers};
use chrono::Utc;
use shared::{DeployState, Event, Response as RpResponse, ServiceSource, ServiceStatus};

pub async fn handle(state: AppState, service_id: String) -> RpResponse {
    let svc = match crate::db::services::get(&state.db, &service_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return RpResponse::err("NotFound", "serviço não encontrado"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    // Marca o serviço como Stopping antes de qualquer operação bloqueante.
    if let Err(e) = crate::db::services::update_status(
        &state.db,
        &service_id,
        &ServiceStatus::Stopping,
        svc.live_container_id.as_deref(),
    )
    .await
    {
        return RpResponse::err("DatabaseError", e.to_string());
    }
    state.bus.publish(Event::ServiceStatusChanged {
        service_id: service_id.clone(),
        status: ServiceStatus::Stopping,
    });

    // Compose services are stopped via compose_down.
    if let ServiceSource::Compose(compose) = &svc.spec.source {
        let pid = &svc.spec.project_id;
        let network_name = crate::docker::networks::project_net_for(pid);
        return stop_compose(
            &state,
            &service_id,
            &svc.spec.name,
            &compose.content,
            &network_name,
        )
        .await;
    }

    // Para todas as instâncias do serviço (suporte a replicas).
    let all_ids = match containers::find_all_by_service_id(&state.docker.inner, &service_id).await {
        Ok(ids) => ids,
        Err(e) => return RpResponse::err("DockerError", e.to_string()),
    };

    // Fallback por nome: containers existentes antes da migração de prefixos de ID
    // têm labels antigas (sem prefixo svc_) e não são encontrados via find_all_by_service_id.
    let ids_to_stop: Vec<String> = if !all_ids.is_empty() {
        all_ids
    } else if let Some(cid) = &svc.live_container_id {
        vec![cid.clone()]
    } else {
        // Último recurso: procurar por nome (rp_<service_name>)
        let replicas = svc.spec.replicas.max(1);
        let mut found = Vec::new();
        for i in 0..replicas {
            let name = containers::replica_live_name(&svc.spec.name, i);
            if let Ok(Some(cid)) = containers::find_by_name(&state.docker.inner, &name).await {
                found.push(cid);
            }
        }
        found
    };

    if ids_to_stop.is_empty() {
        return RpResponse::err("NotRunning", "serviço não possui containers ativos");
    }

    for cid in &ids_to_stop {
        if let Err(e) = containers::stop_graceful(&state.docker.inner, cid, 10).await {
            return RpResponse::err("DockerError", e.to_string());
        }
    }

    let primary_id = ids_to_stop.first().map(|s| s.as_str()).or(svc.live_container_id.as_deref());
    finish_stop(&state, &service_id, primary_id).await
}

async fn stop_compose(
    state: &AppState,
    service_id: &str,
    service_name: &str,
    content: &str,
    network_name: &str,
) -> RpResponse {
    if let Err(e) =
        crate::docker::compose::compose_down(content, &crate::docker::compose::compose_project_name(service_id, service_name), network_name)
            .await
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

    if let Ok(history) = crate::db::deployments::list_for_service(&state.db, service_id, 1).await {
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
