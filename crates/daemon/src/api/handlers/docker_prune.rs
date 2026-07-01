use crate::api::AppState;
use bollard::{
    container::PruneContainersOptions,
    image::PruneImagesOptions,
    network::PruneNetworksOptions,
    volume::PruneVolumesOptions,
};
use shared::Response as RpResponse;

pub async fn prune_containers(state: AppState) -> RpResponse {
    match state
        .docker
        .inner
        .prune_containers(None::<PruneContainersOptions<String>>)
        .await
    {
        Ok(r) => RpResponse::PruneResult {
            count: r.containers_deleted.map(|v| v.len() as u32).unwrap_or(0),
            reclaimed_bytes: r.space_reclaimed.unwrap_or(0) as u64,
        },
        Err(e) => RpResponse::err("DockerError", e.to_string()),
    }
}

/// `all=true` mirrors `docker volume prune --all`: considers every unused
/// volume, not just anonymous ones. The `all` filter isn't in bollard
/// 0.17.1's documented list for this endpoint, but `filters` is a plain
/// map serialized straight to JSON, so the Engine API (1.42+) still honors it.
pub async fn prune_volumes(state: AppState, all: bool) -> RpResponse {
    let mut filters = std::collections::HashMap::new();
    if all {
        filters.insert("all", vec!["true"]);
    }
    match state
        .docker
        .inner
        .prune_volumes(Some(PruneVolumesOptions { filters }))
        .await
    {
        Ok(r) => RpResponse::PruneResult {
            count: r.volumes_deleted.map(|v| v.len() as u32).unwrap_or(0),
            reclaimed_bytes: r.space_reclaimed.unwrap_or(0) as u64,
        },
        Err(e) => RpResponse::err("DockerError", e.to_string()),
    }
}

/// `all=true` mirrors `docker image prune -a`: removes every image unused by
/// any container, not just dangling/untagged ones (`dangling=true` is
/// Docker's own default when the filter is omitted).
pub async fn prune_images(state: AppState, all: bool) -> RpResponse {
    let mut filters = std::collections::HashMap::new();
    if all {
        filters.insert("dangling", vec!["false"]);
    }
    match state
        .docker
        .inner
        .prune_images(Some(PruneImagesOptions { filters }))
        .await
    {
        Ok(r) => RpResponse::PruneResult {
            count: r.images_deleted.map(|v| v.len() as u32).unwrap_or(0),
            reclaimed_bytes: r.space_reclaimed.unwrap_or(0) as u64,
        },
        Err(e) => RpResponse::err("DockerError", e.to_string()),
    }
}

/// Usa `docker builder prune -f` via subprocess — a API REST do BuildKit
/// não está exposta pelo bollard 0.17.
pub async fn prune_build_cache(_state: AppState) -> RpResponse {
    let output = tokio::process::Command::new("docker")
        .args(["builder", "prune", "-f"])
        .output()
        .await;

    match output {
        Ok(out) if out.status.success() => {
            // Tenta extrair "Total reclaimed space: X.XXX MB" da saída
            let text = String::from_utf8_lossy(&out.stdout);
            let reclaimed_bytes = parse_reclaimed_space(&text);
            RpResponse::PruneResult { count: 0, reclaimed_bytes }
        }
        Ok(out) => {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            RpResponse::err("DockerError", msg)
        }
        Err(e) => RpResponse::err("DockerError", e.to_string()),
    }
}

fn parse_reclaimed_space(output: &str) -> u64 {
    for line in output.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Total reclaimed space:") {
            let rest = rest.trim();
            // exemplos: "1.234 GB", "512 MB", "0B"
            if let Some((num, unit)) = rest.split_once(char::is_whitespace) {
                let n: f64 = num.parse().unwrap_or(0.0);
                return match unit.to_uppercase().as_str() {
                    "GB" | "GIB" => (n * 1_000_000_000.0) as u64,
                    "MB" | "MIB" => (n * 1_000_000.0) as u64,
                    "KB" | "KIB" => (n * 1_000.0) as u64,
                    _ => n as u64,
                };
            }
        }
    }
    0
}

pub async fn prune_networks(state: AppState) -> RpResponse {
    match state
        .docker
        .inner
        .prune_networks(None::<PruneNetworksOptions<String>>)
        .await
    {
        Ok(r) => RpResponse::PruneResult {
            count: r.networks_deleted.map(|v| v.len() as u32).unwrap_or(0),
            reclaimed_bytes: 0,
        },
        Err(e) => RpResponse::err("DockerError", e.to_string()),
    }
}
