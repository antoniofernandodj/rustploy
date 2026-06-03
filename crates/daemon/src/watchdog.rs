use crate::{api::AppState, db::Db, deploy::executor::DeployExecutor, event_bus::EventBus};
use bollard::Docker;
use shared::{Event, Healthcheck, HealthcheckKind, ServiceSource, ServiceStatus};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::sleep;
use tracing::{debug, info, warn};

const BASE_TICK: Duration = Duration::from_secs(5);
const MAX_RESTART_ATTEMPTS: u32 = 3;
const RESTART_WAIT: Duration = Duration::from_secs(3);

struct ServiceState {
    last_check: Instant,
    consecutive_failures: u32,
    restart_attempts: u32,
}

pub async fn watchdog_loop(state: AppState) {
    let mut states: HashMap<String, ServiceState> = HashMap::new();

    loop {
        sleep(BASE_TICK).await;

        let services = match crate::db::services::get_watchable(&state.db).await {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "watchdog: failed to fetch watchable services");
                continue;
            }
        };

        let running_ids: std::collections::HashSet<&str> =
            services.iter().map(|s| s.id.as_str()).collect();
        states.retain(|id, _| running_ids.contains(id.as_str()));

        let now = Instant::now();

        for svc in &services {
            let Some(container_id) = &svc.live_container_id else {
                continue;
            };

            let hc = &svc.spec.healthcheck;
            let interval = Duration::from_secs(hc.interval_secs as u64);
            let timeout = Duration::from_secs(hc.timeout_secs as u64);

            let svc_state = states
                .entry(svc.id.clone())
                .or_insert_with(|| ServiceState {
                    last_check: now - interval,
                    consecutive_failures: 0,
                    restart_attempts: 0,
                });

            if now.duration_since(svc_state.last_check) < interval {
                continue;
            }
            svc_state.last_check = now;

            // 1. Container está rodando?
            if !container_is_running(&state.docker.inner, container_id).await {
                try_restart(&state, svc, container_id, svc_state).await;
                continue;
            }

            // Container está vivo: zerar contador de restarts
            svc_state.restart_attempts = 0;

            // 2. Healthcheck funcional (HTTP / TCP / DockerNative)
            let ok = run_healthcheck(
                hc,
                container_id,
                &state.docker.inner,
                svc.spec.port,
                timeout,
            )
            .await;

            if ok {
                if svc_state.consecutive_failures > 0 {
                    info!(
                        service = %svc.spec.name,
                        prev_failures = svc_state.consecutive_failures,
                        "watchdog: healthcheck voltou a passar, restaurando Running"
                    );
                    mark_service(
                        &state.db,
                        &state.bus,
                        &svc.id,
                        &ServiceStatus::Running,
                        Some(container_id),
                    )
                    .await;
                } else {
                    debug!(service = %svc.spec.name, "watchdog: healthcheck OK");
                }
                svc_state.consecutive_failures = 0;
            } else {
                svc_state.consecutive_failures += 1;
                let failures = svc_state.consecutive_failures;
                let retries = hc.retries;

                if failures >= retries {
                    warn!(
                        service = %svc.spec.name,
                        failures,
                        retries,
                        "watchdog: healthcheck falhou {failures}x, marcando como Error"
                    );
                    let new_status =
                        ServiceStatus::Error(format!("healthcheck falhou {failures}x"));
                    mark_service(&state.db, &state.bus, &svc.id, &new_status, None).await;
                    states.remove(&svc.id);
                } else if failures == 1 {
                    warn!(service = %svc.spec.name, failures, retries, "watchdog: Degraded");
                    mark_service(
                        &state.db,
                        &state.bus,
                        &svc.id,
                        &ServiceStatus::Degraded,
                        Some(container_id),
                    )
                    .await;
                } else {
                    warn!(service = %svc.spec.name, failures, retries, "watchdog: healthcheck falhando ({failures}/{retries})");
                }
            }
        }
    }
}

async fn try_restart(
    state: &AppState,
    svc: &shared::Service,
    container_id: &str,
    svc_state: &mut ServiceState,
) {
    svc_state.restart_attempts += 1;
    let attempt = svc_state.restart_attempts;

    if attempt > MAX_RESTART_ATTEMPTS {
        warn!(
            service = %svc.spec.name,
            "watchdog: container não subiu após {MAX_RESTART_ATTEMPTS} tentativas → redeploy"
        );
        trigger_redeploy(state, svc).await;
        return;
    }

    info!(
        service = %svc.spec.name,
        container_id = %container_id,
        attempt,
        max = MAX_RESTART_ATTEMPTS,
        "watchdog: container parou, tentando restart ({attempt}/{MAX_RESTART_ATTEMPTS})"
    );

    mark_service(
        &state.db,
        &state.bus,
        &svc.id,
        &ServiceStatus::Degraded,
        Some(container_id),
    )
    .await;

    use bollard::container::StartContainerOptions;
    let start_result = state
        .docker
        .inner
        .start_container(container_id, None::<StartContainerOptions<String>>)
        .await;

    match start_result {
        Err(e) if is_not_found_error(&e) => {
            // Container foi removido (docker rm) — não adianta tentar start, precisa redeployar
            warn!(
                service = %svc.spec.name,
                container_id = %container_id,
                "watchdog: container foi removido, disparando redeploy"
            );
            trigger_redeploy(state, svc).await;
            return;
        }
        Err(e) => {
            warn!(
                service = %svc.spec.name,
                container_id = %container_id,
                error = %e,
                attempt,
                "watchdog: docker start falhou"
            );
            if attempt >= MAX_RESTART_ATTEMPTS {
                trigger_redeploy(state, svc).await;
            }
            return;
        }
        Ok(_) => {}
    }

    // Aguardar o container inicializar
    sleep(RESTART_WAIT).await;

    if container_is_running(&state.docker.inner, container_id).await {
        info!(
            service = %svc.spec.name,
            container_id = %container_id,
            attempt,
            "watchdog: container reiniciado com sucesso"
        );
        mark_service(
            &state.db,
            &state.bus,
            &svc.id,
            &ServiceStatus::Running,
            Some(container_id),
        )
        .await;
        svc_state.consecutive_failures = 0;
    } else {
        warn!(
            service = %svc.spec.name,
            container_id = %container_id,
            attempt,
            "watchdog: container não ficou em pé após restart"
        );
        if attempt >= MAX_RESTART_ATTEMPTS {
            trigger_redeploy(state, svc).await;
        }
    }
}

fn is_not_found_error(e: &bollard::errors::Error) -> bool {
    let msg = e.to_string();
    msg.contains("404") || msg.contains("No such container")
}

async fn trigger_redeploy(state: &AppState, svc: &shared::Service) {
    info!(service = %svc.spec.name, service_id = %svc.id, "watchdog: disparando redeploy automático");

    let image = match &svc.spec.source {
        ServiceSource::Registry { image } => image.clone(),
        ServiceSource::Git(_) => format!("rp_{}", svc.spec.name),
        ServiceSource::Compose(c) => format!("compose:{}", c.content),
    };

    let dep = match crate::db::deployments::create(&state.db, &svc.id, &image).await {
        Ok(d) => d,
        Err(e) => {
            warn!(service = %svc.spec.name, error = %e, "watchdog: falha ao criar deployment para redeploy");
            mark_service(
                &state.db,
                &state.bus,
                &svc.id,
                &ServiceStatus::Error("redeploy falhou: não foi possível criar deployment".into()),
                None,
            )
            .await;
            return;
        }
    };

    let _ = crate::db::services::update_status(&state.db, &svc.id, &ServiceStatus::Deploying, None)
        .await;
    state.bus.publish(Event::ServiceStatusChanged {
        service_id: svc.id.clone(),
        status: ServiceStatus::Deploying,
    });

    let executor = Arc::new(DeployExecutor {
        db: state.db.clone(),
        docker: state.docker.clone(),
        ingress: state.ingress.clone(),
        bus: state.bus.clone(),
        secrets: state.secrets.clone(),
        db_path: state.db_path.clone(),
        drain_secs: state.drain_secs,
    });
    let dep_id = dep.id.clone();
    tokio::spawn(async move { executor.run(dep_id).await });
}

async fn run_healthcheck(
    hc: &Healthcheck,
    container_id: &str,
    docker: &Docker,
    port: u16,
    timeout: Duration,
) -> bool {
    match &hc.kind {
        HealthcheckKind::DockerNative => {
            use bollard::container::InspectContainerOptions;
            match docker
                .inspect_container(container_id, None::<InspectContainerOptions>)
                .await
            {
                Ok(info) => {
                    use bollard::models::HealthStatusEnum;
                    let status = info
                        .state
                        .as_ref()
                        .and_then(|s| s.health.as_ref())
                        .and_then(|h| h.status.as_ref());
                    match status {
                        None => true,
                        Some(s) => *s == HealthStatusEnum::HEALTHY,
                    }
                }
                Err(_) => false,
            }
        }
        HealthcheckKind::Http {
            path,
            expected_status,
        } => match get_container_ip(docker, container_id).await {
            Some(ip) => {
                let url = format!("http://{ip}:{port}{path}");
                crate::health::check_http(&url, *expected_status, timeout).await
            }
            None => false,
        },
        HealthcheckKind::Tcp => match get_container_ip(docker, container_id).await {
            Some(ip) => {
                let addr = format!("{ip}:{port}");
                crate::health::check_tcp(&addr, timeout).await
            }
            None => false,
        },
    }
}

async fn get_container_ip(docker: &Docker, container_id: &str) -> Option<String> {
    use bollard::container::InspectContainerOptions;
    let info = docker
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await
        .ok()?;
    info.network_settings?
        .networks?
        .values()
        .find_map(|net| net.ip_address.clone().filter(|ip| !ip.is_empty()))
}

async fn container_is_running(docker: &Docker, container_id: &str) -> bool {
    use bollard::container::InspectContainerOptions;
    match docker
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await
    {
        Ok(info) => info.state.as_ref().and_then(|s| s.running).unwrap_or(false),
        Err(_) => false,
    }
}

async fn mark_service(
    db: &Arc<Db>,
    bus: &Arc<EventBus>,
    service_id: &str,
    status: &ServiceStatus,
    container_id: Option<&str>,
) {
    match crate::db::services::update_status(db, service_id, status, container_id).await {
        Ok(_) => bus.publish(Event::ServiceStatusChanged {
            service_id: service_id.to_string(),
            status: status.clone(),
        }),
        Err(e) => {
            warn!(error = %e, service_id = %service_id, "watchdog: falha ao atualizar status")
        }
    }
}
