use anyhow::{Result, anyhow};
use bollard::{
    Docker,
    errors::Error as BollardError,
    container::{
        Config,
        CreateContainerOptions,
        InspectContainerOptions,
        LogsOptions,
        NetworkingConfig,
        RemoveContainerOptions,
        RenameContainerOptions,
        StartContainerOptions,
        StopContainerOptions,
    },
    models::{
        EndpointSettings,
        HostConfig,
        Mount,
        MountTypeEnum,
        RestartPolicy,
        RestartPolicyNameEnum
    },
};
use futures::StreamExt;
use shared::ServiceSpec;
use std::collections::HashMap;
use tracing::{debug, info, warn};

pub fn staging_name(
    service_name: &str,
    deployment_id_short: &str
) -> String {
    replica_staging_name(service_name, deployment_id_short, 0)
}

pub fn _live_name(service_name: &str) -> String {
    replica_live_name(service_name, 0)
}

pub fn replica_staging_name(
    service_name: &str,
    dep_short: &str,
    idx: u32
) -> String {
    let safe_name = shared::normalize_name(service_name);
    if idx == 0 {
        format!("rp_{safe_name}_staging_{dep_short}")
    } else {
        format!("rp_{safe_name}_staging_{dep_short}_r{idx}")
    }
}

pub fn replica_live_name(
    service_name: &str,
    idx: u32
) -> String {
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
        env_keys = ?resolved_env
            .iter()
            .map(
                |(k, _)| k.as_str()
            )
            .collect::<Vec<_>>(),
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
    labels.insert(
        "rustploy.managed".to_string(),
        "true".to_string()
    );

    labels.insert(
        "rustploy.service_id".to_string(),
        service_id.to_string()
    );

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
        networking_config: Some(
            NetworkingConfig {
                endpoints_config: endpoints
            }
        ),
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
        warn!(
            name = %name,
            warnings = ?response.warnings,
            "Docker retornou warnings"
        );
    }
    Ok(response.id)
}

pub async fn start(
    docker: &Docker,
    container_id: &str
) -> Result<()> {
    info!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        "iniciando container"
    );
    
    docker
        .start_container(
            container_id,
            None::<StartContainerOptions<String>>
        )
        .await?;

    info!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        "container em execução"
    );
    Ok(())
}

/// Retorna `true` se o erro do bollard for um 404 (container inexistente).
///
/// Um `container_id`/nome desatualizado no DB (ex.: após uma promoção que
/// removeu o container antigo mas não persistiu o novo id a tempo) não deve
/// ser tratado como falha ao parar/remover — o estado desejado já é
/// satisfeito.
fn is_not_found(e: &BollardError) -> bool {
    matches!(e, BollardError::DockerResponseServerError { status_code: 404, .. })
}

pub async fn stop_graceful(
    docker: &Docker,
    container_id: &str,
    timeout: i64
) -> Result<()> {
    info!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        timeout = timeout,
        "parando container"
    );

    let opts = StopContainerOptions { t: timeout };
    if let Err(e) = docker.stop_container(container_id, Some(opts)).await {
        if is_not_found(&e) {
            warn!(
                container_id = %format!(
                    "...{}",
                    &container_id[..container_id.len().min(10)]
                ),
                "container já não existe, ignorando"
            );
            return Ok(());
        }
        return Err(e.into());
    }

    info!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        "container parado"
    );

    Ok(())
}

pub async fn rename(
    docker: &Docker,
    container_id: &str,
    new_name: &str
) -> Result<()> {
    info!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        new_name = %new_name, "renomeando container"
    );

    let opts = RenameContainerOptions {
        name: new_name.to_string(),
    };

    docker.rename_container(container_id, opts).await?;
    info!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        new_name = %new_name, "renomeado"
    );
    Ok(())
}

pub async fn remove(
    docker: &Docker,
    container_id: &str
) -> Result<()> {
    info!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        "removendo container"
    );

    let opts = RemoveContainerOptions {
        force: true,
        v: true,
        ..Default::default()
    };
    if let Err(e) = docker.remove_container(container_id, Some(opts)).await {
        if is_not_found(&e) {
            warn!(
                container_id = %format!(
                    "...{}",
                    &container_id[..container_id.len().min(10)]
                ),
                "container já não existe, ignorando"
            );
            return Ok(());
        }
        return Err(e.into());
    }
    info!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]),
            "removido"
        );
    Ok(())
}

pub async fn inspect(
    docker: &Docker,
    container_id: &str,
) -> Result<bollard::models::ContainerInspectResponse> {
    debug!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        "inspecionando"
    );

    let resp = docker
        .inspect_container(container_id, None::<InspectContainerOptions>)
        .await?;
    let running = resp.state.as_ref().and_then(|s| s.running).unwrap_or(false);
    let status = resp.state.as_ref().and_then(|s| s.status.clone());
    debug!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
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
    debug!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        network = %network_name,
        "inspecionando rede"
    );

    let net_info = docker
        .inspect_network::<String>(network_name, None)
        .await
        .map_err(|e| anyhow!("falha ao inspecionar rede {network_name}: {e}"))?;

    let net_containers = net_info.containers.unwrap_or_default();

    info!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        network = %network_name,
        count = net_containers.len(),
        ids = ?net_containers
            .keys()
            .map(|k| &k[..k.len().min(12)])
            .collect::<Vec<_>>(),
        "containers encontrados na rede"
    );

    // Chave do mapa é o container ID completo (64 hex chars)
    let nc = net_containers
        .get(container_id)
        .or_else(|| {
            net_containers
                .iter()
                .find(
                    |(k, _)| k
                        .starts_with(
                            container_id
                        ) || container_id.starts_with(
                            k.as_str()
                        )
                )
                .map(|(_, v)| v)
        })
        .ok_or_else(|| {
            let ids: Vec<String> = net_containers
                .keys()
                .map(|k| k[..k.len().min(12)].to_string())
                .collect();
            anyhow!(
                "container não encontrado na rede {network_name} (presentes: {ids:?})"
            )
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
        .ok_or_else(
            || anyhow!("sem IPv4 para container na rede {network_name}")
        )?;

    info!(
        container_id = %format!(
            "...{}",
            &container_id[..container_id.len().min(10)]
        ),
        network = %network_name,
        ip = %ip, "IP resolvido"
    );
    Ok(ip)
}

/// Returns the last `tail` lines of stdout+stderr from a container (best-effort).
pub async fn get_container_logs(
    docker: &Docker,
    container_id: &str,
    tail: usize
) -> Vec<String> {

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

pub async fn find_all_by_service_id(
    docker: &Docker,
    service_id: &str
) -> Result<Vec<String>> {
    use bollard::container::ListContainersOptions;
    debug!(
        service_id = %service_id,
        "buscando containers por service_id"
    );
    let mut filters = HashMap::new();
    filters.insert(
        "label".to_string(),
        vec![format!("rustploy.service_id={service_id}")]
    );
    let opts = ListContainersOptions {
        all: true,
        filters, ..Default::default()
    };
    let list = docker.list_containers(Some(opts)).await?;
    let ids: Vec<String> = list
        .into_iter()
        .filter_map(|c| c.id).collect();

    debug!(
        service_id = %service_id,
        count = ids.len(),
        "containers encontrados"
    );
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

/// Índice de containers do host resolvido numa única listagem, com duas chaves:
/// por `rustploy.service_id` (serviços normais — live/staging/réplicas) e por
/// `com.docker.compose.project` (serviços Compose, cujos containers são criados
/// pelo `docker compose` e **não** carregam os labels `rustploy.*`). O snapshot
/// consulta com [`ContainerIndex::for_service`], que tenta o service_id e cai
/// para o nome de projeto Compose.
#[derive(Default)]
pub struct ContainerIndex {
    by_service_id: HashMap<String, Vec<ManagedContainer>>,
    by_compose_project: HashMap<String, Vec<ManagedContainer>>,
}

impl ContainerIndex {
    /// Containers de um serviço: pelos labels `rustploy.service_id`, ou — quando
    /// vazio (serviço Compose) — pelo `com.docker.compose.project` derivado de
    /// `compose_project_name(id, name)`.
    pub fn for_service(
        &self,
        service_id: &str,
        service_name: &str
    ) -> Vec<ManagedContainer> {
        if let Some(v) = self.by_service_id.get(service_id) {
            return v.clone();
        }
        let project = shared::compose_project_name(
            service_id,
            service_name
        );
        self.by_compose_project.get(&project)
            .cloned()
            .unwrap_or_default()
    }
}

/// Lista **todos** os containers do host numa única chamada e os indexa por
/// service_id e por projeto Compose (ver [`ContainerIndex`]). Erros de Docker
/// degradam para um índice vazio (o snapshot segue sem a informação).
pub async fn index_containers(
    docker: &Docker
) -> ContainerIndex {
    use bollard::container::ListContainersOptions;
    let opts = ListContainersOptions::<String> {
        all: true,
        ..Default::default()
    };
    let list = match docker.list_containers(Some(opts)).await {
        Ok(l) => l,
        Err(e) => {
            warn!(
                error = %e, 
                "containers::index_containers: falha ao listar containers"
            );
            return ContainerIndex::default();
        }
    };
    let mut idx = ContainerIndex::default();
    for c in list {
        let labels = c.labels.clone().unwrap_or_default();
        let name = c
            .names
            .as_ref()
            .and_then(|n| n.first())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_default();

        let mc = ManagedContainer {
            id: c.id.unwrap_or_default(),
            name,
            state: c.state.unwrap_or_default(),
        };
        if let Some(service_id) = labels.get("rustploy.service_id") {
            idx.by_service_id.entry(service_id.clone()).or_default().push(mc);
        } else if let Some(project) = labels.get("com.docker.compose.project") {
            idx.by_compose_project.entry(project.clone()).or_default().push(mc);
        }
    }
    idx
}

/// Returns container IDs for a service excluding those from the given deployment.
pub async fn find_old_containers(
    docker: &Docker,
    service_id: &str,
    exclude_deployment_id: &str,
) -> Result<Vec<String>> {
    use bollard::container::ListContainersOptions;
    debug!(
        service_id = %service_id,
        "containers::find_old_containers: buscando"
    );
    let mut filters = HashMap::new();
    filters.insert(
        "label".to_string(),
        vec![format!("rustploy.service_id={service_id}")]
    );
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

    debug!(
        service_id = %service_id,
        count = ids.len(),
        "containers antigos encontrados"
    );
    Ok(ids)
}

pub async fn find_by_name(
    docker: &Docker,
    name: &str
) -> Result<Option<String>> {
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
            debug!(
                name = %name,
                container_id = %id,
                "container encontrado por nome"
            )
        }
        None => debug!(
            name = %name,
            "container não encontrado"
        ),
    }
    Ok(found)
}

/// Containers **rodando** cujo nome começa com `prefix`, como `(id, nome)`,
/// ordenados por nome.
///
/// A ordenação não é cosmética: é o que torna a escolha do alvo de ingress
/// determinística. A ordem que o Docker devolve não é estável, e uma stack
/// Compose que troca de container-alvo entre dois boots é exatamente o bug que
/// isto existe para não repetir.
pub async fn list_running_by_prefix(
    docker: &Docker,
    prefix: &str,
) -> Result<Vec<(String, String)>> {
    use bollard::container::ListContainersOptions;
    let mut filters = HashMap::new();
    filters.insert("name".to_string(), vec![format!("^/{prefix}")]);
    let opts = ListContainersOptions {
        all: false, // só running: container parado não serve de backend
        filters,
        ..Default::default()
    };
    let mut out: Vec<(String, String)> = docker
        .list_containers(Some(opts))
        .await?
        .into_iter()
        .filter_map(|c| {
            let id = c.id?;
            let name = c
                .names
                .unwrap_or_default()
                .into_iter()
                .next()?
                .trim_start_matches('/')
                .to_string();
            Some((id, name))
        })
        .collect();
    out.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(out)
}

/// Portas TCP que o container declara expor (`EXPOSE` da imagem + `expose:`/
/// `ports:` do compose).
async fn exposed_tcp_ports(docker: &Docker, container_id: &str) -> Vec<u16> {
    let Ok(info) = inspect(docker, container_id).await else {
        return Vec::new();
    };
    let mut out: Vec<u16> = info
        .config
        .as_ref()
        .and_then(|c| c.exposed_ports.as_ref())
        .map(|ports| {
            ports
                .keys()
                // chave no formato "8000/tcp"
                .filter_map(|k| {
                    let (num, proto) = k.split_once('/').unwrap_or((k.as_str(), "tcp"));
                    (proto == "tcp").then(|| num.parse::<u16>().ok()).flatten()
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_unstable();
    out
}

/// Índice do candidato cujo nome é o do serviço `svc` dentro do compose.
///
/// O Compose nomeia `<projeto>-<serviço>-<réplica>`; aceita também o nome sem
/// sufixo de réplica, que é o que aparece com `container_name:` fixo.
fn match_ingress_service(
    candidates: &[(String, String)],
    prefix: &str,
    svc: &str,
) -> Option<usize> {
    let exact = format!("{prefix}{svc}");
    let replica = format!("{prefix}{svc}-");
    candidates
        .iter()
        .position(|(_, n)| *n == exact || n.starts_with(&replica))
}

/// Índice do primeiro candidato que expõe alguma das portas pedidas.
///
/// A ordem de `want_ports` manda: a primeira porta pedida que alguém expuser
/// decide, para que um serviço com dois domínios em portas diferentes caia
/// sempre no mesmo container em vez de alternar.
fn match_exposed_port(exposed: &[Vec<u16>], want_ports: &[u16]) -> Option<usize> {
    want_ports
        .iter()
        .find_map(|want| exposed.iter().position(|ports| ports.contains(want)))
}

/// Escolhe qual container de uma stack Compose recebe o tráfego do ingress.
///
/// Uma stack tem N containers e só um atende o domínio — num Supabase
/// self-hosted são nove, e quem serve a porta 8000 é o `kong`. Até 2026-08-28
/// isto era `find_by_prefix`, que "pega o primeiro que encontrar": a rota caía
/// num container arbitrário da stack e o domínio respondia **502**.
///
/// Ordem de decisão:
/// 1. `ingress_service` do `ComposeSource`, quando preenchido — o compose
///    nomeia os containers `<projeto>-<serviço>-<n>`;
/// 2. o container que expõe alguma das `want_ports` (as portas que os domínios
///    do serviço pedem) — resolve o caso comum sem configuração nenhuma;
/// 3. o primeiro por nome, com `warn` no log — último recurso, para não deixar
///    o serviço sem rota alguma.
pub async fn find_compose_ingress_container(
    docker: &Docker,
    project_name: &str,
    ingress_service: Option<&str>,
    want_ports: &[u16],
) -> Result<Option<String>> {
    let prefix = format!("{project_name}-");
    let candidates = list_running_by_prefix(docker, &prefix).await?;
    if candidates.is_empty() {
        return Ok(None);
    }

    // 1. Alvo declarado pelo usuário.
    if let Some(svc) = ingress_service.map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(i) = match_ingress_service(&candidates, &prefix, svc) {
            let (id, name) = &candidates[i];
            info!(container = %name, ingress_service = %svc, "compose ingress: alvo declarado");
            return Ok(Some(id.clone()));
        }
        // Não abortamos: um nome errado no spec não deve derrubar a stack
        // inteira. Mas o log tem de dizer por que a escolha não foi a pedida.
        warn!(
            ingress_service = %svc,
            project = %project_name,
            candidatos = ?candidates.iter().map(|(_, n)| n).collect::<Vec<_>>(),
            "compose ingress: `ingress_service` não existe na stack — caindo para a descoberta por porta"
        );
    }

    // 2. Quem expõe a porta que o domínio pede.
    if !want_ports.is_empty() {
        let mut portas_por_container: Vec<(String, String, Vec<u16>)> =
            Vec::with_capacity(candidates.len());
        for (id, name) in &candidates {
            let ports = exposed_tcp_ports(docker, id).await;
            portas_por_container.push((id.clone(), name.clone(), ports));
        }
        let so_portas: Vec<Vec<u16>> = portas_por_container
            .iter()
            .map(|(_, _, p)| p.clone())
            .collect();
        if let Some(i) = match_exposed_port(&so_portas, want_ports) {
            let (id, name, _) = &portas_por_container[i];
            info!(container = %name, "compose ingress: alvo por porta exposta");
            return Ok(Some(id.clone()));
        }
        warn!(
            project = %project_name,
            ?want_ports,
            candidatos = ?portas_por_container
                .iter()
                .map(|(_, n, p)| format!("{n}{p:?}"))
                .collect::<Vec<_>>(),
            "compose ingress: nenhum container expõe a porta pedida — usando o primeiro por nome"
        );
    }

    // 3. Último recurso, determinístico.
    Ok(candidates.into_iter().next().map(|(id, _)| id))
}


#[cfg(test)]
mod tests_ingress {
    use super::*;

    fn cands(nomes: &[&str]) -> Vec<(String, String)> {
        nomes
            .iter()
            .map(|n| (format!("id_{n}"), n.to_string()))
            .collect()
    }

    /// A stack real que motivou tudo isto: nove containers de um Supabase
    /// self-hosted, e só o `kong` atende a porta 8000 do domínio.
    const STACK: &[&str] = &[
        "rp_01m11z0g_db-auth-1",
        "rp_01m11z0g_db-db-1",
        "rp_01m11z0g_db-functions-1",
        "rp_01m11z0g_db-imgproxy-1",
        "rp_01m11z0g_db-kong-1",
        "rp_01m11z0g_db-meta-1",
        "rp_01m11z0g_db-rest-1",
        "rp_01m11z0g_db-storage-1",
        "rp_01m11z0g_db-supavisor-1",
    ];

    #[test]
    fn alvo_declarado_acha_o_container_da_replica() {
        let c = cands(STACK);
        let i = match_ingress_service(&c, "rp_01m11z0g_db-", "kong").unwrap();
        assert_eq!(c[i].1, "rp_01m11z0g_db-kong-1");
    }

    #[test]
    fn alvo_declarado_aceita_nome_sem_sufixo_de_replica() {
        let c = cands(&["rp_x-proxy", "rp_x-app-1"]);
        let i = match_ingress_service(&c, "rp_x-", "proxy").unwrap();
        assert_eq!(c[i].1, "rp_x-proxy");
    }

    /// Um `ingress_service` errado não pode casar por acidente com o prefixo
    /// de outro serviço: `db` não é `db-db`.
    #[test]
    fn alvo_declarado_inexistente_nao_casa() {
        let c = cands(&["rp_x-kong-1", "rp_x-storage-1"]);
        assert!(match_ingress_service(&c, "rp_x-", "nginx").is_none());
    }

    /// O caso do 502: sem alvo declarado, quem expõe a porta do domínio vence,
    /// mesmo estando no meio da lista.
    #[test]
    fn porta_do_dominio_escolhe_o_gateway_e_nao_o_primeiro() {
        // mesma ordem de STACK; só o índice 4 (kong) expõe 8000
        let exposed = vec![
            vec![9999],      // auth
            vec![5432],      // db
            vec![9000],      // functions
            vec![8080],      // imgproxy
            vec![8000, 8443],// kong
            vec![8080],      // meta
            vec![3000],      // rest
            vec![5000],      // storage
            vec![4000, 5452],// supavisor
        ];
        assert_eq!(match_exposed_port(&exposed, &[8000]), Some(4));
    }

    #[test]
    fn sem_ninguem_expondo_a_porta_nao_ha_escolha_por_porta() {
        let exposed = vec![vec![5432], vec![5000]];
        assert_eq!(match_exposed_port(&exposed, &[8000]), None);
    }

    /// Duas portas pedidas: manda a ordem de `want_ports`, não a dos containers.
    #[test]
    fn a_primeira_porta_pedida_decide() {
        let exposed = vec![vec![443], vec![8000]];
        assert_eq!(match_exposed_port(&exposed, &[8000, 443]), Some(1));
        assert_eq!(match_exposed_port(&exposed, &[443, 8000]), Some(0));
    }
}
