use crate::{api::AppState, db::services, deploy::executor::DeployExecutor};
use shared::{Event, Response as RpResponse, ServiceSource, ServiceStatus};
use std::sync::Arc;
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
            ServiceSource::Compose(c) => format!("compose:{}", c.content),
        },
        port = svc.spec.port,
        "deploy_start: serviço encontrado"
    );

    if matches!(svc.status, ServiceStatus::Deploying) {
        warn!(service_id = %service_id, "deploy_start: deploy já em andamento, rejeitando");
        return RpResponse::err("ServiceAlreadyDeploying", "deploy already in progress");
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

    info!(service_id = %service_id, "deploy_start: atualizando status para Deploying");
    let _ =
        crate::db::services::update_status(&state.db, &service_id, &ServiceStatus::Deploying, None)
            .await;

    state.bus.publish(Event::ServiceStatusChanged {
        service_id: service_id.clone(),
        status: ServiceStatus::Deploying,
    });
    info!(service_id = %service_id, "deploy_start: evento ServiceStatusChanged(Deploying) publicado");

    // Auto-cria webhook token na primeira vez para serviços Application (não Compose)
    if !matches!(svc.spec.source, ServiceSource::Compose(_)) {
        if let Ok(None) = crate::db::webhook_tokens::get(&state.db, &service_id).await {
            let token = crate::db::webhook_tokens::generate_token();
            let _ = crate::db::webhook_tokens::upsert(&state.db, &service_id, &token).await;
        }
    }

    let executor = Arc::new(DeployExecutor {
        db: state.db.clone(),
        docker: state.docker.clone(),
        ingress: state.ingress.clone(),
        bus: state.bus.clone(),
        secrets: state.secrets.clone(),
        tls: state.tls.clone(),
        db_path: state.db_path.clone(),
        drain_secs: state.drain_secs,
    });
    let dep_id = dep.id.clone();
    info!(deployment_id = %dep_id, "deploy_start: spawning DeployExecutor");
    tokio::spawn(async move { executor.run(dep_id).await });

    RpResponse::Deployment(dep)
}
