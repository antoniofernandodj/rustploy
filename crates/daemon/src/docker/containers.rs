use anyhow::{anyhow, Result};
use bollard::{
    container::{
        Config, CreateContainerOptions, InspectContainerOptions, RemoveContainerOptions,
        RenameContainerOptions, StartContainerOptions, StopContainerOptions,
    },
    models::{HostConfig, Mount, MountTypeEnum, RestartPolicy, RestartPolicyNameEnum},
    Docker,
};
use shared::ServiceSpec;
use std::collections::HashMap;
use tracing::info;

pub fn staging_name(service_name: &str, deployment_id_short: &str) -> String {
    format!("rp_{service_name}_staging_{deployment_id_short}")
}

pub fn live_name(service_name: &str) -> String {
    format!("rp_{service_name}")
}

pub async fn create_staging(
    docker: &Docker,
    spec: &ServiceSpec,
    image: &str,
    service_id: &str,
    deployment_id: &str,
    network_id: &str,
    resolved_env: &[(String, String)],
) -> Result<String> {
    let dep_short = &deployment_id[..8.min(deployment_id.len())];
    let name = staging_name(&spec.name, dep_short);

    info!(name, image, "creating staging container");

    let env: Vec<String> = resolved_env
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();

    let mounts: Vec<Mount> = spec
        .volumes
        .iter()
        .map(|v| Mount {
            target: Some(v.container_path.clone()),
            source: Some(v.host_path.clone()),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(v.read_only),
            ..Default::default()
        })
        .collect();

    let mut labels = HashMap::new();
    labels.insert("rustploy.managed".to_string(), "true".to_string());
    labels.insert("rustploy.service_id".to_string(), service_id.to_string());
    labels.insert("rustploy.deployment_id".to_string(), deployment_id.to_string());

    let mem_limit = if spec.resources.mem_limit_bytes > 0 {
        Some(spec.resources.mem_limit_bytes as i64)
    } else {
        None
    };
    let cpu_shares = if spec.resources.cpu_shares > 0 {
        Some(spec.resources.cpu_shares as i64)
    } else {
        None
    };

    let host_config = HostConfig {
        network_mode: Some(network_id.to_string()),
        mounts: Some(mounts),
        memory: mem_limit,
        cpu_shares,
        restart_policy: Some(RestartPolicy {
            name: Some(RestartPolicyNameEnum::NO),
            maximum_retry_count: None,
        }),
        ..Default::default()
    };

    let config = Config {
        image: Some(image.to_string()),
        env: Some(env),
        labels: Some(labels),
        host_config: Some(host_config),
        exposed_ports: Some({
            let mut m = HashMap::new();
            m.insert(format!("{}/tcp", spec.port), HashMap::new());
            m
        }),
        ..Default::default()
    };

    let opts = CreateContainerOptions { name: name.clone(), platform: None };
    let response = docker.create_container(Some(opts), config).await?;
    Ok(response.id)
}

pub async fn start(docker: &Docker, container_id: &str) -> Result<()> {
    docker
        .start_container(container_id, None::<StartContainerOptions<String>>)
        .await?;
    Ok(())
}

pub async fn stop_graceful(docker: &Docker, container_id: &str, timeout: i64) -> Result<()> {
    let opts = StopContainerOptions { t: timeout };
    docker.stop_container(container_id, Some(opts)).await?;
    Ok(())
}

pub async fn rename(docker: &Docker, container_id: &str, new_name: &str) -> Result<()> {
    let opts = RenameContainerOptions { name: new_name.to_string() };
    docker.rename_container(container_id, opts).await?;
    Ok(())
}

pub async fn remove(docker: &Docker, container_id: &str) -> Result<()> {
    let opts = RemoveContainerOptions { force: true, v: true, ..Default::default() };
    docker.remove_container(container_id, Some(opts)).await?;
    Ok(())
}

pub async fn inspect(
    docker: &Docker,
    container_id: &str,
) -> Result<bollard::models::ContainerInspectResponse> {
    let resp = docker
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await?;
    Ok(resp)
}

pub async fn get_container_ip(
    docker: &Docker,
    container_id: &str,
    network_name: &str,
) -> Result<String> {
    let info = inspect(docker, container_id).await?;
    let networks = info
        .network_settings
        .and_then(|s| s.networks)
        .ok_or_else(|| anyhow!("no network settings"))?;

    let endpoint = networks
        .get(network_name)
        .ok_or_else(|| anyhow!("container not on network {network_name}"))?;

    endpoint
        .ip_address
        .clone()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("no IP for container on network {network_name}"))
}

pub async fn find_by_name(docker: &Docker, name: &str) -> Result<Option<String>> {
    use bollard::container::ListContainersOptions;
    let mut filters = HashMap::new();
    filters.insert("name".to_string(), vec![format!("^/{name}$")]);
    let opts = ListContainersOptions {
        all: true,
        filters,
        ..Default::default()
    };
    let containers = docker.list_containers(Some(opts)).await?;
    Ok(containers.into_iter().next().and_then(|c| c.id))
}
