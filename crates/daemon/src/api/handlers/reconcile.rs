/// Reconciles deployments marked as "Live" in the DB against the actual Docker
/// container state, correcting stale entries.
///
/// Rules:
///   1. For each service, only the most recent deployment can be Live.
///      All older Live deployments for the same service → Pruning.
///   2. For the most recent Live deployment per service, verify its container
///      is actually running. If not → Stopped.
use crate::{api::AppState, docker};
use chrono::Utc;
use shared::{DeployState, Deployment, Event, ServiceStatus};
use std::collections::HashSet;
use tracing::warn;

pub async fn fix_stale_live(state: &AppState, deployments: Vec<Deployment>) -> Vec<Deployment> {
    // Collect the ID of the most-recent Live deployment per service.
    // The list is already ordered by started_at DESC, so the first Live entry
    // per service_id is the candidate; subsequent ones are stale.
    let mut seen_live: HashSet<String> = HashSet::new();
    let mut out = Vec::with_capacity(deployments.len());

    for mut dep in deployments {
        if dep.state != DeployState::Live {
            out.push(dep);
            continue;
        }

        // Deploy em andamento? Não toca em NADA desse serviço. O executor é a
        // fonte da verdade enquanto roda: ele mesmo promove o novo Live, poda o
        // antigo e escreve o status final (Running/Error).
        //
        // Sem esse guard o reconcile corrompia o estado durante todo deploy: ele
        // roda a cada snapshot (2s, via recent_deployments) e também no
        // deploy_history da aba Deployments, via o `is_container_running` abaixo
        // — que retorna false enquanto o container Live antigo está parado ou o
        // serviço ainda não subiu. Resultado: o Live anterior virava Stopped e o
        // status do serviço era reescrito para Stopped POR CIMA do Deploying que
        // o deploy_start tinha acabado de gravar. Na GUI: card "Stopped" com um
        // deployment "Building" logo abaixo.
        if service_is_deploying(state, &dep.service_id).await {
            out.push(dep);
            continue;
        }

        if seen_live.contains(&dep.service_id) {
            // A newer Live deployment was already accepted for this service.
            // This one is stale → Pruning.
            warn!(
                deployment_id = %dep.id,
                service_id = %dep.service_id,
                "reconcile: deployment Live duplicado para o mesmo serviço — corrigindo para Pruning"
            );
            transition_deployment(
                state,
                &mut dep,
                DeployState::Pruning,
                "superseded by newer deployment",
            )
            .await;
            out.push(dep);
            continue;
        }

        // First Live for this service: verify the container is actually running.
        if !is_container_running(state, &dep.service_id).await {
            warn!(
                deployment_id = %dep.id,
                service_id = %dep.service_id,
                "reconcile: deployment Live mas container não está rodando — corrigindo para Stopped"
            );
            transition_deployment(
                state,
                &mut dep,
                DeployState::Stopped,
                "container not running at read time",
            )
            .await;
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
        } else {
            seen_live.insert(dep.service_id.clone());
        }

        out.push(dep);
    }

    out
}

/// True enquanto um deploy do serviço está em curso ou esperando na fila
/// (`ServiceStatus::Deploying`/`Queued`). `Queued` é escrito por `deploy_start`
/// ao enfileirar; `Deploying` pelo worker ao pegar o deploy — ambos limpos no
/// desfecho. Reconciliar por cima disso zeraria o status (o container ainda não
/// existe), então pulamos.
async fn service_is_deploying(state: &AppState, service_id: &str) -> bool {
    matches!(
        crate::db::services::get(&state.db, service_id).await,
        Ok(Some(svc)) if matches!(svc.status, ServiceStatus::Deploying | ServiceStatus::Queued)
    )
}

async fn transition_deployment(state: &AppState, dep: &mut Deployment, to: DeployState, msg: &str) {
    let _ = crate::db::deployments::transition(
        &state.db,
        &dep.id,
        &dep.state,
        to.clone(),
        Some(msg.into()),
    )
    .await;
    state.bus.publish(Event::DeployStateChanged {
        deployment_id: dep.id.clone(),
        service_id: dep.service_id.clone(),
        state: to.clone(),
        timestamp: Utc::now(),
        message: Some(msg.into()),
    });
    dep.state = to;
}

// Um deploy recém-promovido pode ter seu container ainda propagando o estado
// "running" no Docker Engine no instante exato em que este reconcile roda
// (ex.: GUI busca a lista de deployments quase junto com o evento DeployStateChanged
// de Live). Sem essa margem, esse read prematuro derruba um deploy que na
// prática subiu certo — visto em produção com um serviço Compose.
const RUNNING_CHECK_RETRIES: u32 = 3;
const RUNNING_CHECK_DELAY: std::time::Duration = std::time::Duration::from_millis(300);

async fn is_container_running(state: &AppState, service_id: &str) -> bool {
    let svc = match crate::db::services::get(&state.db, service_id).await {
        Ok(Some(s)) => s,
        _ => return false,
    };

    if svc.status == ServiceStatus::Stopped {
        return false;
    }

    let Some(container_id) = svc.live_container_id else {
        return false;
    };

    for attempt in 0..RUNNING_CHECK_RETRIES {
        let running = match docker::containers::inspect(&state.docker.inner, &container_id).await {
            Ok(info) => info.state.as_ref().and_then(|s| s.running).unwrap_or(false),
            Err(_) => false,
        };
        if running {
            return true;
        }
        if attempt + 1 < RUNNING_CHECK_RETRIES {
            tokio::time::sleep(RUNNING_CHECK_DELAY).await;
        }
    }
    false
}
