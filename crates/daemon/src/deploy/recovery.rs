use crate::{
    db::Db,
    docker,
    docker::{DockerClient, containers},
    event_bus::EventBus,
    ingress::{IngressController, TlsManager},
    secrets::SecretsManager,
};
use shared::{DeployState, Service, ServiceStatus, compose_project_name};
use std::{path::PathBuf, sync::Arc};
use tracing::{info, warn};

pub async fn recover(
    db: Arc<Db>,
    docker: Arc<DockerClient>,
    ingress: Arc<IngressController>,
    bus: Arc<EventBus>,
    secrets: Arc<SecretsManager>,
    tls: Arc<TlsManager>,
    db_path: PathBuf,
    drain_secs: u64,
) {
    let pending = match crate::db::deployments::get_non_terminal(&db).await {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "failed to query non-terminal deployments");
            return;
        }
    };

    // Restore ingress routes for running services and re-provision missing TLS certs
    restore_routes(&db, &docker, &ingress, &tls).await;

    if pending.is_empty() {
        info!("no deployments to recover");
        return;
    }

    info!(count = pending.len(), "recovering deployments");

    for dep in pending {
        let svc = match crate::db::services::get(&db, &dep.service_id).await {
            Ok(Some(s)) => s,
            _ => {
                warn!(
                    deployment_id = dep.id,
                    "service not found during recovery, marking failed"
                );
                let _ = crate::db::deployments::transition(
                    &db,
                    &dep.id,
                    &dep.state,
                    DeployState::Failed,
                    Some("service not found during recovery".into()),
                )
                .await;
                continue;
            }
        };

        match &dep.state {
            // Pre-swap states: safe to abort
            DeployState::Pending
            | DeployState::ResolvingDeps
            | DeployState::PullingImage
            | DeployState::CloningRepo
            | DeployState::BuildingImage
            | DeployState::ComposingUp
            | DeployState::Staging
            | DeployState::HealthcheckPolling => {
                info!(
                    deployment_id = dep.id,
                    state = dep.state.label(),
                    "aborting pre-swap deployment"
                );
                // Remove staging container if it exists
                let staging_name =
                    containers::staging_name(&svc.spec.name, docker::networks::id_short(&dep.id));
                if let Ok(Some(id)) = containers::find_by_name(&docker.inner, &staging_name).await {
                    let _ = containers::remove(&docker.inner, &id).await;
                }
                let _ = crate::db::deployments::transition(
                    &db,
                    &dep.id,
                    &dep.state,
                    DeployState::Failed,
                    Some("daemon restarted during deploy".into()),
                )
                .await;
                let _ = crate::db::services::update_status(
                    &db,
                    &svc.id,
                    &ServiceStatus::Error("deploy interrupted by restart".into()),
                    None,
                )
                .await;
            }

            // Swap in progress: inspect actual containers to decide
            DeployState::SwappingIn | DeployState::Draining => {
                info!(
                    deployment_id = dep.id,
                    state = dep.state.label(),
                    "resuming swap-in-progress deployment"
                );
                let executor = Arc::new(crate::deploy::executor::DeployExecutor {
                    db: db.clone(),
                    docker: docker.clone(),
                    ingress: ingress.clone(),
                    bus: bus.clone(),
                    secrets: secrets.clone(),
                    tls: tls.clone(),
                    db_path: db_path.clone(),
                    drain_secs,
                });
                let dep_id = dep.id.clone();
                tokio::spawn(async move { executor.run(dep_id).await });
            }

            // Promoting: complete the rename
            DeployState::Promoting => {
                info!(deployment_id = dep.id, "completing promotion");
                let executor = Arc::new(crate::deploy::executor::DeployExecutor {
                    db: db.clone(),
                    docker: docker.clone(),
                    ingress: ingress.clone(),
                    bus: bus.clone(),
                    secrets: secrets.clone(),
                    tls: tls.clone(),
                    db_path: db_path.clone(),
                    drain_secs,
                });
                let dep_id = dep.id.clone();
                tokio::spawn(async move { executor.run(dep_id).await });
            }

            // Rolling back: complete the rollback
            DeployState::RollingBack => {
                info!(deployment_id = dep.id, "completing rollback");
                let executor = Arc::new(crate::deploy::executor::DeployExecutor {
                    db: db.clone(),
                    docker: docker.clone(),
                    ingress: ingress.clone(),
                    bus: bus.clone(),
                    secrets: secrets.clone(),
                    tls: tls.clone(),
                    db_path: db_path.clone(),
                    drain_secs,
                });
                let dep_id = dep.id.clone();
                tokio::spawn(async move { executor.run(dep_id).await });
            }

            DeployState::Live
            | DeployState::Stopped
            | DeployState::Failed
            | DeployState::Pruning => {}
        }
    }
}

/// Reconciles every service's DB status against actual Docker container state.
/// Marks stopped containers as Stopped, running containers as Running, and
/// keeps ingress routes in sync. Safe to call at any time; non-destructive.
pub async fn reconcile(
    db: &Db,
    docker: &DockerClient,
    ingress: &IngressController,
    tls: &Arc<TlsManager>,
) {
    let services = match crate::db::services::list_all(db).await {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "reconcile: falha ao listar serviços");
            return;
        }
    };

    for svc in services {
        let replicas = svc.spec.replicas.max(1);
        let net = format!(
            "rp_net_{}",
            docker::networks::id_short(&svc.spec.project_id)
        );

        let mut ips: Vec<String> = Vec::new();

        for i in 0..replicas {
            let live_name = containers::replica_live_name(&svc.spec.name, i);
            if let Ok(Some(cid)) = containers::find_by_name(&docker.inner, &live_name).await {
                if let Ok(ip) =
                    containers::get_container_ip(&docker.inner, &cid, &net).await
                {
                    ips.push(ip);
                }
            }
        }

        if ips.is_empty() {
            let compose_prefix = format!("{}-", compose_project_name(&svc.id, &svc.spec.name));
            if let Ok(Some(cid)) =
                containers::find_by_prefix(&docker.inner, &compose_prefix).await
            {
                if let Ok(ip) =
                    containers::get_container_ip(&docker.inner, &cid, &net).await
                {
                    ips.push(ip);
                }
            }
        }

        let is_running = !ips.is_empty();
        let was_running = matches!(svc.status, ServiceStatus::Running | ServiceStatus::Degraded);

        match (is_running, was_running) {
            (true, false) => {
                info!(service = svc.spec.name, "reconcile: container encontrado, marcando Running");
                let _ = crate::db::services::update_status(
                    db,
                    &svc.id,
                    &ServiceStatus::Running,
                    None,
                )
                .await;
                reconcile_routes(&svc, ips, ingress, tls).await;
            }
            (false, true) => {
                info!(service = svc.spec.name, "reconcile: container ausente, marcando Stopped");
                let _ = crate::db::services::update_status(
                    db,
                    &svc.id,
                    &ServiceStatus::Stopped,
                    None,
                )
                .await;
                ingress.remove_domains(&svc.spec);
                if let Some(host_port) = svc.spec.host_port {
                    ingress.remove_port_route(host_port);
                }
            }
            (true, true) => {
                // Container e DB ambos Running: garante que as rotas estão registradas.
                reconcile_routes(&svc, ips, ingress, tls).await;
            }
            (false, false) => {}
        }
    }
}

async fn reconcile_routes(
    svc: &Service,
    ips: Vec<String>,
    ingress: &IngressController,
    tls: &Arc<TlsManager>,
) {
    ingress.register_domains(&svc.spec, &ips, &svc.id);
    for route in svc.spec.domain_routes().into_iter().filter(|r| r.tls) {
        let tls = tls.clone();
        let domain = route.domain.clone();
        tokio::spawn(async move {
            if let Err(e) = tls.ensure_cert(&domain).await {
                warn!(domain = %domain, error = %e, "reconcile: falha ao garantir cert TLS");
            }
        });
    }
    if let Some(host_port) = svc.spec.host_port {
        let backends: Vec<String> =
            ips.iter().map(|ip| format!("{ip}:{}", svc.spec.port)).collect();
        ingress.upsert_port_route(host_port, backends);
    }
}

async fn restore_routes(db: &Db, docker: &DockerClient, ingress: &IngressController, tls: &Arc<TlsManager>) {
    let services = match crate::db::services::get_running(db).await {
        Ok(v) => v,
        Err(e) => {
            warn!(error = %e, "failed to restore routes");
            return;
        }
    };

    info!(
        count = services.len(),
        "restoring ingress routes for running services"
    );

    for svc in services {
        let replicas = svc.spec.replicas.max(1);
        let net = format!(
            "rp_net_{}",
            docker::networks::id_short(&svc.spec.project_id)
        );

        // Coleta IPs de todas as réplicas live (Git/Registry)
        let mut ips: Vec<String> = Vec::new();
        for i in 0..replicas {
            let live_name = containers::replica_live_name(&svc.spec.name, i);
            if let Ok(Some(cid)) = containers::find_by_name(&docker.inner, &live_name).await {
                if let Ok(ip) =
                    containers::get_container_ip(&docker.inner, &cid, &net).await
                {
                    ips.push(ip);
                }
            }
        }

        // Fallback para Compose: encontra via prefixo do projeto.
        // O nome interno do serviço no compose file pode diferir do nome rustploy.
        if ips.is_empty() {
            let compose_prefix = format!("{}-", compose_project_name(&svc.id, &svc.spec.name));
            if let Ok(Some(cid)) =
                containers::find_by_prefix(&docker.inner, &compose_prefix).await
            {
                if let Ok(ip) =
                    containers::get_container_ip(&docker.inner, &cid, &net).await
                {
                    ips.push(ip);
                }
            }
        }

        if ips.is_empty() {
            warn!(service = svc.spec.name, "no live containers found, skipping route restore");
            continue;
        }

        if !svc.spec.domain_routes().is_empty() {
            ingress.register_domains(&svc.spec, &ips, &svc.id);
            info!(service = svc.spec.name, ?ips, "routes restored");

            for route in svc.spec.domain_routes().into_iter().filter(|r| r.tls) {
                info!(service = svc.spec.name, domain = %route.domain, "TLS: disparando ensure_cert no restart para serviço running");
                let tls = tls.clone();
                let domain = route.domain.clone();
                tokio::spawn(async move {
                    if let Err(e) = tls.ensure_cert(&domain).await {
                        warn!(domain = %domain, error = %e, "TLS: falha ao re-provisionar certificado no restart");
                    }
                });
            }
        }

        if let Some(host_port) = svc.spec.host_port {
            let backends: Vec<String> =
                ips.iter().map(|ip| format!("{ip}:{}", svc.spec.port)).collect();
            ingress.upsert_port_route(host_port, backends.clone());
            info!(service = svc.spec.name, host_port, ?backends, "port routes restored");
        }
    }
}
