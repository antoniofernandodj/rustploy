//! Servidor de banco compartilhado: conecta o container dele às redes dos
//! projetos autorizados, sob o alias global `rp-shared-<id8>-<safe>`.
//!
//! O servidor é que entra na rede do consumidor (e não o contrário), só com o
//! alias global — o alias curto do Compose (`rp_postgres`) não vaza, e os
//! consumidores não passam a se enxergar. Toda recriação do container desfaz a
//! conexão; por isso [`sync_server`] roda após cada `compose up` do servidor e
//! a cada volta do reconcile (30 s, e no boot). Ver
//! `docs/plano-banco-compartilhado.md` §2.3.

use crate::{
    db::Db,
    docker::{DockerClient, containers, networks},
};
use anyhow::{Result, anyhow};
use bollard::{Docker, container::InspectContainerOptions};
use shared::{Service, ServiceSource};
use tracing::{info, warn};

/// Alias global do servidor (`shared::shared_alias` sobre a stack gravada).
pub fn alias_of(svc: &Service) -> String {
    shared::shared_alias(&svc.compose_project_name())
}

/// Container do servidor: o mesmo que o deploy elege como alvo da stack.
pub async fn server_container(docker: &Docker, svc: &Service) -> Option<String> {
    let ServiceSource::Compose(c) = &svc.spec.source else {
        return None;
    };
    let host = shared::connection::compose_host(
        &c.content,
        c.ingress_service.as_deref(),
        svc.spec.db_kind.as_deref(),
    );
    containers::find_compose_ingress_container(
        docker,
        &svc.compose_project_name(),
        host.as_deref(),
        &[svc.spec.port],
    )
    .await
    .ok()
    .flatten()
}

/// Outro container (≠ `me`) que já responde por `alias` em `network`.
async fn alias_holder(docker: &Docker, network: &str, alias: &str, me: &str) -> Option<String> {
    let net = docker.inspect_network::<String>(network, None).await.ok()?;
    for (cid, _) in net.containers.unwrap_or_default() {
        if cid == me {
            continue;
        }
        let Ok(info) = docker
            .inspect_container(&cid, None::<InspectContainerOptions>)
            .await
        else {
            continue;
        };
        let has = info
            .network_settings
            .and_then(|n| n.networks)
            .and_then(|n| n.get(network).cloned())
            .and_then(|e| e.aliases)
            .is_some_and(|a| a.iter().any(|x| x == alias));
        if has {
            return Some(cid);
        }
    }
    None
}

/// Conecta `container_id` à rede com `alias` — e só com ele, sem o alias curto
/// do Compose. Diferente de `containers::attach_network_alias` (que presume o
/// container já na rede), aqui ele normalmente ainda não está. Idempotente: se
/// o alias já está lá, não mexe (não derruba conexões à toa).
async fn connect_with_alias(
    docker: &Docker,
    container_id: &str,
    network: &str,
    alias: &str,
) -> Result<()> {
    use bollard::{
        models::EndpointSettings,
        network::{ConnectNetworkOptions, DisconnectNetworkOptions},
    };
    let info = docker
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await?;
    let endpoint = info
        .network_settings
        .and_then(|n| n.networks)
        .and_then(|n| n.get(network).cloned());
    if let Some(e) = endpoint {
        if e.aliases.is_some_and(|a| a.iter().any(|x| x == alias)) {
            return Ok(());
        }
        // Conectado sem o alias: o Docker não altera aliases de endpoint existente.
        docker
            .disconnect_network(
                network,
                DisconnectNetworkOptions {
                    container: container_id.to_string(),
                    force: true,
                },
            )
            .await
            .map_err(|e| anyhow!("falha ao desconectar de {network}: {e}"))?;
    }
    docker
        .connect_network(
            network,
            ConnectNetworkOptions {
                container: container_id.to_string(),
                endpoint_config: EndpointSettings {
                    aliases: Some(vec![alias.to_string()]),
                    ..Default::default()
                },
            },
        )
        .await
        .map_err(|e| anyhow!("falha ao conectar a {network} com alias {alias}: {e}"))?;
    Ok(())
}

/// Conecta o servidor à rede de um projeto (cria a rede se preciso).
pub async fn connect_to_project(
    db: &Db,
    docker: &DockerClient,
    svc: &Service,
    container_id: &str,
    project_id: &str,
) -> Result<String> {
    if project_id == svc.spec.project_id {
        return Err(anyhow!(
            "o servidor já está na rede deste projeto; use o nome interno dele"
        ));
    }
    let net = crate::db::projects::network_name(db, project_id).await?;
    networks::ensure_project_network(&docker.inner, &net).await?;
    let alias = alias_of(svc);
    if let Some(other) = alias_holder(&docker.inner, &net, &alias, container_id).await {
        return Err(anyhow!(
            "o alias {alias} já está em uso na rede {net} pelo container {other}"
        ));
    }
    connect_with_alias(&docker.inner, container_id, &net, &alias).await?;
    info!(service = %svc.spec.name, network = %net, alias = %alias, "shared_net: servidor conectado");
    Ok(net)
}

/// Garante a conexão do servidor a **todas** as redes autorizadas. Devolve
/// quantas redes ficaram conectadas; erros por rede são logados, não abortam.
pub async fn sync_server(db: &Db, docker: &DockerClient, svc: &Service) -> usize {
    if svc.spec.shared.is_none() {
        return 0;
    }
    let Some(cid) = server_container(&docker.inner, svc).await else {
        return 0;
    };
    let projects = match crate::db::shared_access::projects_of(db, &svc.id).await {
        Ok(p) => p,
        Err(e) => {
            warn!(service = %svc.spec.name, error = %e, "shared_net: falha ao ler acessos");
            return 0;
        }
    };
    let mut ok = 0;
    for p in projects {
        match connect_to_project(db, docker, svc, &cid, &p).await {
            Ok(_) => ok += 1,
            Err(e) => warn!(service = %svc.spec.name, project = %p, error = %e, "shared_net: não conectou"),
        }
    }
    ok
}

/// Desconecta o servidor da rede de um projeto (revogação de acesso).
pub async fn disconnect_from_project(
    db: &Db,
    docker: &DockerClient,
    svc: &Service,
    project_id: &str,
) -> Result<()> {
    let Some(cid) = server_container(&docker.inner, svc).await else {
        return Ok(());
    };
    let net = crate::db::projects::network_name(db, project_id).await?;
    if let Err(e) = networks::_disconnect_container(&docker.inner, &net, &cid).await {
        // Rede ou conexão já inexistente não é erro de revogação.
        warn!(network = %net, error = %e, "shared_net: desconexão (ignorado)");
    }
    Ok(())
}
