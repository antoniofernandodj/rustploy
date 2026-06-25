use anyhow::Result;
use bollard::{
    Docker,
    network::{ConnectNetworkOptions, CreateNetworkOptions, DisconnectNetworkOptions},
};
use tracing::info;

pub fn project_network_name(project_id_short: &str) -> String {
    format!("rp_net_{project_id_short}")
}

pub async fn ensure_project_network(docker: &Docker, project_id: &str) -> Result<String> {
    let pid = project_id.find('_').map(|i| &project_id[i + 1..]).unwrap_or(project_id);
    let short = &pid[..8.min(pid.len())];
    let name = project_network_name(short);

    if let Ok(info) = docker.inspect_network::<String>(&name, None).await {
        let id = info.id.clone().unwrap_or_else(|| name.clone());
        info!(network = %name, id = %id, "networks::ensure: rede já existe");
        return Ok(id);
    }

    info!(network = %name, driver = "bridge", "networks::ensure: criando nova rede");
    let options = CreateNetworkOptions {
        name: name.clone(),
        driver: "bridge".to_string(),
        internal: false,
        attachable: false,
        ..Default::default()
    };
    let resp = docker.create_network(options).await?;
    let id = resp.id.clone().unwrap_or_else(|| name.clone());
    info!(network = %name, id = %id, "networks::ensure: rede criada");
    Ok(id)
}

pub async fn remove_project_network(docker: &Docker, project_id: &str) -> Result<()> {
    let pid = project_id.find('_').map(|i| &project_id[i + 1..]).unwrap_or(project_id);
    let short = &pid[..8.min(pid.len())];
    let name = project_network_name(short);
    info!(network = %name, "networks::remove: removendo rede do projeto");
    let _ = docker.remove_network(&name).await;
    info!(network = %name, "networks::remove: rede removida");
    Ok(())
}

pub async fn connect_container(
    docker: &Docker,
    network_name: &str,
    container_id: &str,
) -> Result<()> {
    info!(network = %network_name, container_id = %container_id, "networks::connect: conectando container");
    let opts = ConnectNetworkOptions {
        container: container_id.to_string(),
        ..Default::default()
    };
    docker.connect_network(network_name, opts).await?;
    info!(network = %network_name, container_id = %container_id, "networks::connect: container conectado");
    Ok(())
}

pub async fn disconnect_container(
    docker: &Docker,
    network_name: &str,
    container_id: &str,
) -> Result<()> {
    info!(network = %network_name, container_id = %container_id, "networks::disconnect: desconectando container");
    let opts = DisconnectNetworkOptions {
        container: container_id.to_string(),
        force: true,
    };
    docker.disconnect_network(network_name, opts).await?;
    info!(network = %network_name, container_id = %container_id, "networks::disconnect: desconectado");
    Ok(())
}
