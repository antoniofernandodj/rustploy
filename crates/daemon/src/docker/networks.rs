use anyhow::Result;
use bollard::{
    network::{ConnectNetworkOptions, CreateNetworkOptions, DisconnectNetworkOptions},
    Docker,
};
use tracing::info;

pub fn project_network_name(project_id_short: &str) -> String {
    format!("rp_net_{project_id_short}")
}

pub async fn ensure_project_network(docker: &Docker, project_id: &str) -> Result<String> {
    let short = &project_id[..8.min(project_id.len())];
    let name = project_network_name(short);

    // Return existing network if it exists
    if let Ok(info) = docker.inspect_network::<String>(&name, None).await {
        return Ok(info.id.unwrap_or(name));
    }

    info!(name, "creating project network");
    let options = CreateNetworkOptions {
        name: name.clone(),
        driver: "bridge".to_string(),
        internal: false,
        attachable: false,
        ..Default::default()
    };
    let resp = docker.create_network(options).await?;
    Ok(resp.id.unwrap_or(name))
}

pub async fn remove_project_network(docker: &Docker, project_id: &str) -> Result<()> {
    let short = &project_id[..8.min(project_id.len())];
    let name = project_network_name(short);
    let _ = docker.remove_network(&name).await;
    Ok(())
}

pub async fn connect_container(
    docker: &Docker,
    network_name: &str,
    container_id: &str,
) -> Result<()> {
    let opts = ConnectNetworkOptions {
        container: container_id.to_string(),
        ..Default::default()
    };
    docker.connect_network(network_name, opts).await?;
    Ok(())
}

pub async fn disconnect_container(
    docker: &Docker,
    network_name: &str,
    container_id: &str,
) -> Result<()> {
    let opts = DisconnectNetworkOptions {
        container: container_id.to_string(),
        force: true,
    };
    docker.disconnect_network(network_name, opts).await?;
    Ok(())
}
