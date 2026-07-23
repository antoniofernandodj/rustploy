use crate::{api::AppState, db::services};
use shared::{Event, Response as RpResponse, ServiceSource, ServiceStatus};
use tracing::{error, info, warn};

pub async fn handle(state: AppState, service_id: String) -> RpResponse {
    info!(service_id = %service_id, "deploy_start: solicitação recebida");

    let svc = match services::get(&state.db, &service_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            warn!(service_id = %service_id, "deploy_start: serviço não encontrado");
            return RpResponse::err("NotFound", "service not found");
        }
        Err(e) => {
            error!(service_id = %service_id, error = %e, "deploy_start: erro ao buscar serviço");
            return RpResponse::err("DatabaseError", e.to_string());
        }
    };

    info!(
        service_id = %service_id,
        name = %svc.spec.name,
        status = %svc.status,
        source = match &svc.spec.source {
            ServiceSource::Registry { image } => format!("registry:{image}"),
            ServiceSource::Git(g) => format!("git:{}", g.url),
            ServiceSource::Archive(a) => format!("archive:{}", a.archive_id),
            ServiceSource::Compose(c) => format!("compose:{}", c.content),
        },
        port = svc.spec.port,
        "deploy_start: serviço encontrado"
    );

    if matches!(svc.status, ServiceStatus::Deploying | ServiceStatus::Queued) {
        warn!(service_id = %service_id, "deploy_start: deploy já em andamento/na fila, rejeitando");
        return RpResponse::err(
            "ServiceAlreadyDeploying",
            "deploy already in progress or queued",
        );
    }

    // Valida que a fonte tem conteúdo configurado
    match &svc.spec.source {
        ServiceSource::Registry { image } if image.is_empty() => {
            warn!(service_id = %service_id, "deploy_start: imagem registry vazia");
            return RpResponse::err(
                "InvalidSource",
                "registry image is empty — configure a imagem antes de fazer deploy",
            );
        }
        ServiceSource::Git(g) if g.url.is_empty() => {
            warn!(service_id = %service_id, "deploy_start: URL git vazia");
            return RpResponse::err(
                "InvalidSource",
                "git URL is empty — configure o repositório antes de fazer deploy",
            );
        }
        ServiceSource::Archive(a) if a.archive_id.is_empty() => {
            warn!(service_id = %service_id, "deploy_start: archive vazio");
            return RpResponse::err(
                "InvalidSource",
                "zip archive is empty — envie um zip antes de fazer deploy",
            );
        }
        ServiceSource::Compose(c) if c.content.is_empty() => {
            warn!(service_id = %service_id, "deploy_start: compose file vazio");
            return RpResponse::err(
                "InvalidSource",
                "compose file is empty — configure o caminho do docker-compose.yml",
            );
        }
        _ => {}
    }

    let image = match &svc.spec.source {
        ServiceSource::Registry { image } => {
            info!(service_id = %service_id, image = %image, "deploy_start: fonte registry, imagem resolvida");
            image.clone()
        }
        ServiceSource::Git(_) => {
            let tag = format!("rp_{}", svc.spec.safe_name());
            info!(service_id = %service_id, tag = %tag, "deploy_start: fonte git, tag de build resolvida");
            tag
        }
        ServiceSource::Archive(_) => {
            let tag = format!("rp_{}", svc.spec.safe_name());
            info!(service_id = %service_id, tag = %tag, "deploy_start: fonte archive, tag de build resolvida");
            tag
        }
        ServiceSource::Compose(c) => {
            let tag = format!("compose:{}", c.content);
            info!(service_id = %service_id, tag = %tag, "deploy_start: fonte compose, referência resolvida");
            tag
        }
    };

    let dep = match crate::db::deployments::create(&state.db, &service_id, &image).await {
        Ok(d) => {
            info!(
                service_id = %service_id,
                deployment_id = %d.id,
                image = %d.image,
                "deploy_start: deployment criado no banco"
            );
            d
        }
        Err(e) => {
            error!(service_id = %service_id, error = %e, "deploy_start: falha ao criar deployment");
            return RpResponse::err("DatabaseError", e.to_string());
        }
    };

    info!(service_id = %service_id, "deploy_start: atualizando status para Queued");
    let _ =
        crate::db::services::update_status(&state.db, &service_id, &ServiceStatus::Queued, None)
            .await;

    state.bus.publish(Event::ServiceStatusChanged {
        service_id: service_id.clone(),
        status: ServiceStatus::Queued,
    });
    info!(service_id = %service_id, "deploy_start: evento ServiceStatusChanged(Queued) publicado");

    // Auto-cria webhook token na primeira vez para serviços Application (não Compose)
    if !matches!(svc.spec.source, ServiceSource::Compose(_)) {
        if let Ok(None) = crate::db::webhook_tokens::get(&state.db, &service_id).await {
            let token = crate::db::webhook_tokens::generate_token();
            let _ = crate::db::webhook_tokens::upsert(&state.db, &service_id, &token).await;
        }
    }

    // Enfileira na fila global (um por vez). O worker
    // (`crate::deploy::queue::run_worker`) puxa serialmente, marca o serviço
    // como `Deploying` e roda o `DeployExecutor`.
    info!(deployment_id = %dep.id, "deploy_start: enfileirando deploy");
    state.deploy_queue.enqueue(dep.id.clone());
    state.bus.publish(Event::DeployQueueChanged);

    RpResponse::Deployment(dep)
}
