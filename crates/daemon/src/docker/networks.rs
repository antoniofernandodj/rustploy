//! Rede Docker por projeto: nome e criação sob demanda.

use anyhow::Result;
use bollard::{
    Docker,
    network::{ConnectNetworkOptions, CreateNetworkOptions, DisconnectNetworkOptions},
};
use tracing::info;

/// Nome de rede no formato **legado** (`rp_net_` + 8 primeiros chars do ID),
/// derivado a cada uso e sem garantia de unicidade. Só a migração de
/// preenchimento (SQL, em `db::migrate`) e o fallback de
/// `db::projects::network_name` o usam; o nome de um projeto novo é gravado na
/// criação (`shared::new_project_network_name`).
pub fn legacy_project_net_for(project_id: &str) -> String {
    let s = project_id
        .find('_')
        .map(|i| &project_id[i + 1..])
        .unwrap_or(project_id);
    format!("rp_net_{}", &s[..8.min(s.len())])
}

pub fn id_short(id: &str) -> &str {
    let s = id.find('_').map(|i| &id[i + 1..]).unwrap_or(id);

    &s[..8.min(s.len())]
}

/// Garante que a rede `name` existe (cria uma bridge se não) e devolve o id.
/// O nome vem de `db::projects::network_name`.
pub async fn ensure_project_network(docker: &Docker, name: &str) -> Result<String> {
    let name = name.to_string();

    if let Ok(info) = docker.inspect_network::<String>(&name, None).await {
        let id = info.id.clone().unwrap_or_else(|| name.clone());

        info!(
            network = %name,
            id = %id,
            "networks::ensure: rede já existe"
        );
        return Ok(id);
    }

    info!(
        network = %name,
        driver = "bridge",
        "networks::ensure: criando nova rede"
    );
    let options = CreateNetworkOptions {
        name: name.clone(),
        driver: "bridge".to_string(),
        internal: false,
        attachable: false,
        ..Default::default()
    };
    let resp = docker.create_network(options).await?;
    let id = resp.id.clone().unwrap_or_else(|| name.clone());
    info!(
        network = %name,
        id = %id,
        "networks::ensure: rede criada"
    );
    Ok(id)
}

pub async fn _remove_project_network(docker: &Docker, name: &str) -> Result<()> {
    info!(
        network = %name,
        "networks::remove: removendo rede do projeto"
    );
    let _ = docker.remove_network(name).await;
    info!(
        network = %name,
        "networks::remove: rede removida"
    );
    Ok(())
}

pub async fn _connect_container(
    docker: &Docker,
    network_name: &str,
    container_id: &str,
) -> Result<()> {
    info!(
        network = %network_name,
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        "networks::connect: conectando container"
    );
    let opts = ConnectNetworkOptions {
        container: container_id.to_string(),
        ..Default::default()
    };
    docker.connect_network(network_name, opts).await?;
    info!(
        network = %network_name,
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        "networks::connect: container conectado"
    );
    Ok(())
}

pub async fn _disconnect_container(
    docker: &Docker,
    network_name: &str,
    container_id: &str,
) -> Result<()> {
    info!(
        network = %network_name,
        container_id = %container_id,
        "networks::disconnect: desconectando container"
    );
    let opts = DisconnectNetworkOptions {
        container: container_id.to_string(),
        force: true,
    };

    docker.disconnect_network(network_name, opts).await?;

    info!(
        network = %network_name,
        container_id = %container_id,
        "networks::disconnect: desconectado"
    );
    Ok(())
}
