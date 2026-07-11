//! Docker-wide inventory for the Docker tab: every image/volume/network on
//! the host (not just rustploy-managed resources), plus a robust "stop
//! everything rustploy manages" command.

use crate::{api::AppState, docker};
use shared::{
    DockerContainerInfo, DockerImageInfo, DockerNetworkInfo, DockerVolumeInfo,
    Response as RpResponse, ServiceSource, ServiceStatus,
};
use std::collections::HashMap;

/// Best-effort project/service ownership lookup, built once per request from
/// the current DB state. Two ways a Docker resource points back to a service:
/// - Git-built images/containers are tagged `rp_<safe_name>:...` (see
///   `deploy/executor.rs`'s `BuildingImage` step) — matched by `safe_name`.
/// - Registry-sourced services reference the image string verbatim.
/// - Project networks are named `rp_net_<project_id_short>` (see
///   `docker/networks.rs::project_network_name`).
struct ServiceIndex {
    by_safe_name: HashMap<String, (String, String)>,
    by_registry_image: HashMap<String, (String, String)>,
    by_project_short: HashMap<String, String>,
    /// service_id → (nome do projeto, nome do serviço), para atribuir um
    /// container pelo label `rustploy.service_id` (exato). Ver `list_containers`.
    by_service_id: HashMap<String, (String, String)>,
}

impl ServiceIndex {
    async fn build(state: &AppState) -> Self {
        let projects = crate::db::projects::list(&state.db).await.unwrap_or_default();
        let project_names: HashMap<String, String> =
            projects.iter().map(|p| (p.id.clone(), p.name.clone())).collect();
        let by_project_short = projects
            .iter()
            .map(|p| (docker::networks::id_short(&p.id).to_string(), p.name.clone()))
            .collect();

        let services = crate::db::services::list_all(&state.db).await.unwrap_or_default();
        let mut by_safe_name = HashMap::new();
        let mut by_registry_image = HashMap::new();
        let mut by_service_id = HashMap::new();
        for s in &services {
            let project_name = project_names
                .get(&s.spec.project_id)
                .cloned()
                .unwrap_or_else(|| s.spec.project_id.clone());
            by_service_id.insert(s.id.clone(), (project_name.clone(), s.spec.name.clone()));
            match &s.spec.source {
                ServiceSource::Git(_) => {
                    by_safe_name.insert(s.spec.safe_name(), (project_name, s.spec.name.clone()));
                }
                ServiceSource::Registry { image } => {
                    by_registry_image.insert(image.clone(), (project_name, s.spec.name.clone()));
                }
                ServiceSource::Compose(_) => {}
            }
        }
        Self { by_safe_name, by_registry_image, by_project_short, by_service_id }
    }

    /// Resolves an image's owner from its tags: exact match for registry
    /// images, `rp_<safe_name>:...` prefix match for Git-built ones.
    fn image_owner(&self, tags: &[String]) -> (Option<String>, Option<String>) {
        for tag in tags {
            if let Some((proj, svc)) = self.by_registry_image.get(tag) {
                return (Some(proj.clone()), Some(svc.clone()));
            }
            if let Some(safe) = tag.split(':').next().and_then(|repo| repo.strip_prefix("rp_"))
                && let Some((proj, svc)) = self.by_safe_name.get(safe)
            {
                return (Some(proj.clone()), Some(svc.clone()));
            }
        }
        (None, None)
    }

    /// Resolves a network's owning project from the `rp_net_<short>` naming
    /// convention. `None` for non-rustploy networks.
    fn network_project(&self, name: &str) -> Option<String> {
        let short = name.strip_prefix("rp_net_")?;
        self.by_project_short.get(short).cloned()
    }
}

/// Lists every image on the host via `docker system df`, the one Docker
/// Engine endpoint that always computes `Containers` (in-use count) — the
/// plain image-list endpoint leaves it `-1` unless separately requested.
/// Served from the TTL cache (`state.docker_cache.df`) so the 2s status poll
/// doesn't re-run `docker system df` every tick.
pub async fn list_images(state: AppState) -> RpResponse {
    match state.docker_cache.df.get_or_refresh(|| compute_df_inventory(&state)).await {
        Ok((images, _)) => RpResponse::DockerImages(images),
        Err(e) => RpResponse::err("DockerError", e),
    }
}

/// Lists every volume on the host via `docker system df`, which (unlike the
/// plain volume-list endpoint) populates `UsageData` — the reference count
/// that determines whether a volume is "in use". Shares the cached `df` snapshot
/// with [`list_images`].
pub async fn list_volumes(state: AppState) -> RpResponse {
    match state.docker_cache.df.get_or_refresh(|| compute_df_inventory(&state)).await {
        Ok((_, volumes)) => RpResponse::DockerVolumes(volumes),
        Err(e) => RpResponse::err("DockerError", e),
    }
}

/// Runs `docker system df` once and turns it into both the image and volume
/// inventories (ownership attributed from the current DB state). The cache
/// miss-path of [`list_images`]/[`list_volumes`].
async fn compute_df_inventory(
    state: &AppState,
) -> Result<(Vec<DockerImageInfo>, Vec<DockerVolumeInfo>), String> {
    let df = state.docker.inner.df().await.map_err(|e| e.to_string())?;
    let idx = ServiceIndex::build(state).await;
    let images: Vec<DockerImageInfo> = df
        .images
        .unwrap_or_default()
        .into_iter()
        .map(|img| {
            let (project, service) = idx.image_owner(&img.repo_tags);
            DockerImageInfo {
                id: img.id,
                tags: img.repo_tags,
                size_bytes: img.size.max(0) as u64,
                created: chrono::DateTime::from_timestamp(img.created, 0).unwrap_or_default(),
                containers: img.containers,
                project,
                service,
            }
        })
        .collect();
    let volumes: Vec<DockerVolumeInfo> = df
        .volumes
        .unwrap_or_default()
        .into_iter()
        .map(|v| {
            let ref_count = v.usage_data.as_ref().map(|u| u.ref_count).unwrap_or(-1);
            DockerVolumeInfo {
                name: v.name,
                driver: v.driver,
                mountpoint: v.mountpoint,
                in_use: ref_count > 0,
                ref_count,
                size_bytes: v.usage_data.map(|u| u.size).unwrap_or(-1),
            }
        })
        .collect();
    Ok((images, volumes))
}

/// Lists every network on the host. `in_use` is computed by cross-referencing
/// every container's attached networks (by name) — the plain network-list
/// endpoint never populates its own `Containers` field (only `network
/// inspect` does, and doing that per-network would be an N+1 round trip).
/// Served from the TTL cache (`state.docker_cache.networks`).
pub async fn list_networks(state: AppState) -> RpResponse {
    match state.docker_cache.networks.get_or_refresh(|| compute_networks(&state)).await {
        Ok(infos) => RpResponse::DockerNetworks(infos),
        Err(e) => RpResponse::err("DockerError", e),
    }
}

/// The cache miss-path of [`list_networks`]: lists networks and cross-references
/// container attachments to compute each network's in-use count.
async fn compute_networks(state: &AppState) -> Result<Vec<DockerNetworkInfo>, String> {
    use bollard::{container::ListContainersOptions, network::ListNetworksOptions};

    let networks = state
        .docker
        .inner
        .list_networks(None::<ListNetworksOptions<String>>)
        .await
        .map_err(|e| e.to_string())?;

    let containers = state
        .docker
        .inner
        .list_containers(Some(ListContainersOptions::<String> { all: true, ..Default::default() }))
        .await
        .unwrap_or_default();
    let mut attached: HashMap<String, usize> = HashMap::new();
    for c in &containers {
        if let Some(nets) = c.network_settings.as_ref().and_then(|ns| ns.networks.as_ref()) {
            for name in nets.keys() {
                *attached.entry(name.clone()).or_insert(0) += 1;
            }
        }
    }

    let idx = ServiceIndex::build(state).await;
    Ok(networks
        .into_iter()
        .map(|n| {
            let name = n.name.unwrap_or_default();
            let container_count = attached.get(&name).copied().unwrap_or(0);
            DockerNetworkInfo {
                id: n.id.unwrap_or_default(),
                project: idx.network_project(&name),
                name,
                driver: n.driver.unwrap_or_default(),
                scope: n.scope.unwrap_or_default(),
                in_use: container_count > 0,
                container_count,
            }
        })
        .collect())
}

/// Lists every container on the host (running + stopped) for the Docker tab's
/// Containers sub-tab. Host-wide, não só rustploy: `managed`/`project`/`service`
/// são atribuição best-effort pelo label `rustploy.service_id` (exato, quando
/// presente). Não é cacheado — estado de container muda a cada start/stop.
pub async fn list_containers(state: AppState) -> RpResponse {
    use bollard::container::ListContainersOptions;
    let containers = match state
        .docker
        .inner
        .list_containers(Some(ListContainersOptions::<String> { all: true, ..Default::default() }))
        .await
    {
        Ok(c) => c,
        Err(e) => return RpResponse::err("DockerError", e.to_string()),
    };
    let idx = ServiceIndex::build(&state).await;
    let infos: Vec<DockerContainerInfo> = containers
        .into_iter()
        .map(|c| {
            let name = c
                .names
                .as_ref()
                .and_then(|n| n.first())
                .map(|n| n.trim_start_matches('/').to_string())
                .unwrap_or_default();
            let labels = c.labels.unwrap_or_default();
            let managed = labels.get("rustploy.managed").map(|v| v == "true").unwrap_or(false);
            let (project, service) = labels
                .get("rustploy.service_id")
                .and_then(|sid| idx.by_service_id.get(sid))
                .map(|(p, s)| (Some(p.clone()), Some(s.clone())))
                .unwrap_or((None, None));
            DockerContainerInfo {
                id: c.id.unwrap_or_default(),
                name,
                image: c.image.unwrap_or_default(),
                state: c.state.unwrap_or_default(),
                status: c.status.unwrap_or_default(),
                managed,
                project,
                service,
            }
        })
        .collect();
    RpResponse::DockerContainers(infos)
}

/// Stops every rustploy-managed service, regardless of what the DB's status
/// column currently claims — reuses `service_stop::handle` (which does the
/// real Docker-level container lookup by label/name, not just
/// `live_container_id`) instead of only touching services already marked
/// `Running`/`Degraded`, so nothing is missed due to state drift. Scoped to
/// rustploy's own services; never touches unrelated containers on the host.
pub async fn stop_all_managed(state: AppState) -> RpResponse {
    let services = match crate::db::services::list_all(&state.db).await {
        Ok(s) => s,
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };
    let mut count = 0u32;
    for svc in services {
        if matches!(svc.status, ServiceStatus::Stopped) {
            continue;
        }
        let resp = super::service_stop::handle(state.clone(), svc.id).await;
        if matches!(resp, RpResponse::Ok) {
            count += 1;
        }
    }
    RpResponse::StopAllResult { count }
}
