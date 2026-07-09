use anyhow::{Result, anyhow};
use bollard::{
    Docker,
    container::{
        Config, CreateContainerOptions, InspectContainerOptions, LogsOptions,
        NetworkingConfig, RemoveContainerOptions, RenameContainerOptions, StartContainerOptions,
        StopContainerOptions,
    },
    models::{EndpointSettings, HostConfig, Mount, MountTypeEnum, RestartPolicy, RestartPolicyNameEnum},
};
use futures::StreamExt;
use shared::ServiceSpec;
use std::collections::HashMap;
use tracing::{debug, info, warn};

pub fn staging_name(service_name: &str, deployment_id_short: &str) -> String {
    replica_staging_name(service_name, deployment_id_short, 0)
}

pub fn _live_name(service_name: &str) -> String {
    replica_live_name(service_name, 0)
}

pub fn replica_staging_name(service_name: &str, dep_short: &str, idx: u32) -> String {
    let safe_name = shared::normalize_name(service_name);
    if idx == 0 {
        format!("rp_{safe_name}_staging_{dep_short}")
    } else {
        format!("rp_{safe_name}_staging_{dep_short}_r{idx}")
    }
}

pub fn replica_live_name(service_name: &str, idx: u32) -> String {
    let safe_name = shared::normalize_name(service_name);
    if idx == 0 {
        format!("rp_{safe_name}")
    } else {
        format!("rp_{safe_name}_r{idx}")
    }
}

pub async fn create_staging(
    docker: &Docker,
    spec: &ServiceSpec,
    image: &str,
    service_id: &str,
    deployment_id: &str,
    network_id: &str,
    resolved_env: &[(String, String)],
    container_name: &str,
) -> Result<String> {
    let name = container_name;

    info!(
        name = %name,
        image = %image,
        network = %network_id,
        service_id = %service_id,
        port = spec.port,
        volumes = spec.volumes.len(),
        "criando container"
    );

    let env: Vec<String> = resolved_env
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect();

    debug!(
        name = %name,
        env_keys = ?resolved_env.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
        "env vars configuradas"
    );

    let mounts: Vec<Mount> = spec
        .volumes
        .iter()
        .map(|v| {
            debug!(
                host = %v.host_path,
                container = %v.container_path,
                ro = v.read_only,
                "montando volume"
            );
            Mount {
                target: Some(v.container_path.clone()),
                source: Some(v.host_path.clone()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(v.read_only),
                ..Default::default()
            }
        })
        .collect();

    let mut labels = HashMap::new();
    labels.insert("rustploy.managed".to_string(), "true".to_string());
    labels.insert("rustploy.service_id".to_string(), service_id.to_string());
    labels.insert(
        "rustploy.deployment_id".to_string(),
        deployment_id.to_string(),
    );

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

    debug!(
        name = %name,
        mem_limit = ?mem_limit,
        cpu_shares = ?cpu_shares,
        "limites de recurso"
    );

    let host_config = HostConfig {
        // network_mode substitui a bridge padrão pela rede user-defined do projeto.
        // Equivalente a `docker run --network <rede>`: o Docker configura o DNS
        // embebido (127.0.0.11) imediatamente, sem depender de network connect.
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

    let cmd = if spec.run_command.is_some() || !spec.run_args.is_empty() {
        let mut parts: Vec<String> = spec
            .run_command
            .as_deref()
            .map(|c| vec![c.to_string()])
            .unwrap_or_default();
        parts.extend(spec.run_args.iter().cloned());
        Some(parts)
    } else {
        None
    };

    // Replica o comportamento de `docker run --network <rede>`:
    // a CLI envia TANTO network_mode no HostConfig COMO o endpoint em NetworkingConfig.
    // Sem NetworkingConfig, o Docker pode não configurar o DNS embebido (127.0.0.11)
    // correctamente mesmo com network_mode definido.
    let mut endpoints = HashMap::new();
    endpoints.insert(network_id.to_string(), EndpointSettings::default());

    let config = Config {
        image: Some(image.to_string()),
        env: Some(env),
        labels: Some(labels),
        host_config: Some(host_config),
        cmd,
        exposed_ports: Some({
            let mut m = HashMap::new();
            m.insert(format!("{}/tcp", spec.port), HashMap::new());
            m
        }),
        networking_config: Some(NetworkingConfig { endpoints_config: endpoints }),
        ..Default::default()
    };

    let opts = CreateContainerOptions {
        name: name,
        platform: None,
    };
    let response = docker.create_container(Some(opts), config).await?;
    info!(
        name = %name,
        container_id = %response.id,
        "container criado com sucesso"
    );
    if !response.warnings.is_empty() {
        warn!(name = %name, warnings = ?response.warnings, "Docker retornou warnings");
    }
    Ok(response.id)
}

pub async fn start(docker: &Docker, container_id: &str) -> Result<()> {
    info!(container_id = %format!("...{}", &container_id[..container_id.len().min(10)]), "iniciando container");
    docker
        .start_container(container_id, None::<StartContainerOptions<String>>)
        .await?;
    info!(container_id = %format!("...{}", &container_id[..container_id.len().min(10)]), "container em execução");
    Ok(())
}

pub async fn stop_graceful(docker: &Docker, container_id: &str, timeout: i64) -> Result<()> {
    info!(container_id = %format!("...{}", &container_id[..container_id.len().min(10)]), timeout = timeout, "parando container");
    let opts = StopContainerOptions { t: timeout };
    docker.stop_container(container_id, Some(opts)).await?;
    info!(container_id = %format!("...{}", &container_id[..container_id.len().min(10)]), "container parado");
    Ok(())
}

pub async fn rename(docker: &Docker, container_id: &str, new_name: &str) -> Result<()> {
    info!(container_id = %format!("...{}", &container_id[..container_id.len().min(10)]), new_name = %new_name, "renomeando container");
    let opts = RenameContainerOptions {
        name: new_name.to_string(),
    };
    docker.rename_container(container_id, opts).await?;
    info!(container_id = %format!("...{}", &container_id[..container_id.len().min(10)]), new_name = %new_name, "renomeado");
    Ok(())
}

pub async fn remove(docker: &Docker, container_id: &str) -> Result<()> {
    info!(container_id = %format!("...{}", &container_id[..container_id.len().min(10)]), "removendo container");
    let opts = RemoveContainerOptions {
        force: true,
        v: true,
        ..Default::default()
    };
    docker.remove_container(container_id, Some(opts)).await?;
    info!(container_id = %format!("...{}", &container_id[..container_id.len().min(10)]), "removido");
    Ok(())
}

pub async fn inspect(
    docker: &Docker,
    container_id: &str,
) -> Result<bollard::models::ContainerInspectResponse> {
    debug!(container_id = %format!("...{}", &container_id[..container_id.len().min(10)]), "inspecionando");
    let resp = docker
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await?;
    let running = resp.state.as_ref().and_then(|s| s.running).unwrap_or(false);
    let status = resp.state.as_ref().and_then(|s| s.status.clone());
    debug!(
        container_id = %format!("...{}", &container_id[..container_id.len().min(10)]),
        running = running,
        status = ?status,
        "resultado"
    );
    Ok(resp)
}

pub async fn get_container_ip(
    docker: &Docker,
    container_id: &str,
    network_name: &str,
) -> Result<String> {
    // Usa `docker network inspect` (NetworkContainer.ipv4_address) em vez de
    // `docker container inspect` (EndpointSettings.ip_address).
    // EndpointSettings.ip_address vem vazio em alguns Docker/bollard combos;
    // NetworkContainer.ipv4_address é uma struct diferente e mais confiável.
    debug!(container_id = %format!("...{}", &container_id[..container_id.len().min(10)]), network = %network_name, "inspecionando rede");

    let net_info = docker
        .inspect_network::<String>(network_name, None)
        .await
        .map_err(|e| anyhow!("falha ao inspecionar rede {network_name}: {e}"))?;

    let net_containers = net_info.containers.unwrap_or_default();

    info!(
        container_id = %format!("...{}", &container_id[..container_id.len().min(10)]),
        network = %network_name,
        count = net_containers.len(),
        ids = ?net_containers.keys().map(|k| &k[..k.len().min(12)]).collect::<Vec<_>>(),
        "containers encontrados na rede"
    );

    // Chave do mapa é o container ID completo (64 hex chars)
    let nc = net_containers
        .get(container_id)
        .or_else(|| {
            net_containers
                .iter()
                .find(|(k, _)| k.starts_with(container_id) || container_id.starts_with(k.as_str()))
                .map(|(_, v)| v)
        })
        .ok_or_else(|| {
            let ids: Vec<String> = net_containers
                .keys()
                .map(|k| k[..k.len().min(12)].to_string())
                .collect();
            anyhow!("container não encontrado na rede {network_name} (presentes: {ids:?})")
        })?;

    info!(
        container_id = %format!("...{}", &container_id[..container_id.len().min(10)]),
        network = %network_name,
        ipv4 = ?nc.ipv4_address,
        mac = ?nc.mac_address,
        "NetworkContainer encontrado"
    );

    // ipv4_address vem no formato CIDR "172.18.0.2/16" — extrai só o IP
    let ip = nc
        .ipv4_address
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|s| s.split('/').next().unwrap_or(s).to_string())
        .ok_or_else(|| anyhow!("sem IPv4 para container na rede {network_name}"))?;

    info!(
        container_id = %format!("...{}", &container_id[..container_id.len().min(10)]),
        network = %network_name,
        ip = %ip, "IP resolvido"
    );
    Ok(ip)
}

/// Returns the last `tail` lines of stdout+stderr from a container (best-effort).
pub async fn get_container_logs(docker: &Docker, container_id: &str, tail: usize) -> Vec<String> {
    let opts = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        tail: tail.to_string(),
        ..Default::default()
    };
    let mut stream = docker.logs(container_id, Some(opts));
    let mut lines = Vec::new();
    while let Some(Ok(output)) = stream.next().await {
        let text = output.to_string();
        for line in text.lines() {
            if !line.is_empty() {
                lines.push(line.to_string());
            }
        }
    }
    lines
}

pub async fn find_all_by_service_id(docker: &Docker, service_id: &str) -> Result<Vec<String>> {
    use bollard::container::ListContainersOptions;
    debug!(service_id = %service_id, "buscando containers por service_id");
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec![format!("rustploy.service_id={service_id}")]);
    let opts = ListContainersOptions { all: true, filters, ..Default::default() };
    let list = docker.list_containers(Some(opts)).await?;
    let ids: Vec<String> = list.into_iter().filter_map(|c| c.id).collect();
    debug!(service_id = %service_id, count = ids.len(), "containers encontrados");
    Ok(ids)
}

/// Container gerenciado pelo rustploy, no formato leve que o GUI exibe
/// (id + nome + estado). Serializado direto no snapshot HTTP/JSON.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ManagedContainer {
    /// ID completo do container (o GUI encurta para exibição).
    pub id: String,
    /// Nome do container (sem a barra inicial que o Docker devolve), ex. `rp_web`.
    pub name: String,
    /// Estado do container: `running`, `exited`, `created`, ...
    pub state: String,
}

/// Lista **todos** os containers gerenciados pelo rustploy numa única chamada
/// ao Docker e os agrupa por `service_id` (label `rustploy.service_id`). Usado
/// pelo snapshot para anexar, a cada serviço, os containers que ele está de fato
/// executando — cobre réplicas, staging em andamento e serviços Compose. Erros
/// de Docker degradam para um mapa vazio (o snapshot segue sem a informação).
pub async fn list_managed_grouped(docker: &Docker) -> HashMap<String, Vec<ManagedContainer>> {
    use bollard::container::ListContainersOptions;
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec!["rustploy.managed=true".to_string()]);
    let opts = ListContainersOptions { all: true, filters, ..Default::default() };
    let list = match docker.list_containers(Some(opts)).await {
        Ok(l) => l,
        Err(e) => {
            warn!(error = %e, "containers::list_managed_grouped: falha ao listar containers");
            return HashMap::new();
        }
    };
    let mut out: HashMap<String, Vec<ManagedContainer>> = HashMap::new();
    for c in list {
        let Some(service_id) = c
            .labels
            .as_ref()
            .and_then(|l| l.get("rustploy.service_id"))
            .cloned()
        else {
            continue;
        };
        let name = c
            .names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_default();
        out.entry(service_id).or_default().push(ManagedContainer {
            id: c.id.unwrap_or_default(),
            name,
            state: c.state.unwrap_or_default(),
        });
    }
    out
}

/// Returns container IDs for a service excluding those from the given deployment.
pub async fn find_old_containers(
    docker: &Docker,
    service_id: &str,
    exclude_deployment_id: &str,
) -> Result<Vec<String>> {
    use bollard::container::ListContainersOptions;
    debug!(service_id = %service_id, "containers::find_old_containers: buscando");
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec![format!("rustploy.service_id={service_id}")]);
    let opts = ListContainersOptions { all: true, filters, ..Default::default() };
    let list = docker.list_containers(Some(opts)).await?;
    let ids: Vec<String> = list
        .into_iter()
        .filter(|c| {
            let dep = c.labels.as_ref()
                .and_then(|l| l.get("rustploy.deployment_id"))
                .map(|s| s.as_str())
                .unwrap_or("");
            dep != exclude_deployment_id
        })
        .filter_map(|c| c.id)
        .collect();
    debug!(service_id = %service_id, count = ids.len(), "containers antigos encontrados");
    Ok(ids)
}

pub async fn find_by_name(docker: &Docker, name: &str) -> Result<Option<String>> {
    use bollard::container::ListContainersOptions;
    debug!(name = %name, "buscando container por nome");
    let mut filters = HashMap::new();
    filters.insert("name".to_string(), vec![format!("^/{name}$")]);
    let opts = ListContainersOptions {
        all: true,
        filters,
        ..Default::default()
    };
    let containers = docker.list_containers(Some(opts)).await?;
    let found = containers.into_iter().next().and_then(|c| c.id);
    match &found {
        Some(id) => {
            debug!(name = %name, container_id = %id, "container encontrado por nome")
        }
        None => debug!(name = %name, "container não encontrado"),
    }
    Ok(found)
}

pub async fn find_by_prefix(docker: &Docker, prefix: &str) -> Result<Option<String>> {
    use bollard::container::ListContainersOptions;
    debug!(prefix = %prefix, "buscando container por prefix");
    let mut filters = HashMap::new();
    filters.insert("name".to_string(), vec![format!("^/{prefix}")]);
    let opts = ListContainersOptions {
        all: true,
        filters,
        ..Default::default()
    };
    let containers = docker.list_containers(Some(opts)).await?;
    // Pega o primeiro que encontrar
    let found = containers.into_iter().next().and_then(|c| c.id);
    Ok(found)
}
