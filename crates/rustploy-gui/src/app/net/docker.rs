//! Host-wide Docker inventory maintenance (image/volume/network pruning) for
//! the Docker tab. Listing itself is folded into `poll_stream`'s 2s tick; this
//! module only covers the "clear unused" buttons.

use super::view;
use super::{outcome_toast, RwpClient};
use glacier_ui::EffectOutcome;
use shared::{Command, Response};

pub struct Docker {
    client: RwpClient,
}

impl Docker {
    pub fn new(client: RwpClient) -> Self {
        Self { client }
    }

    /// Removes unused (untagged, or referenced by no container) images — or,
    /// with `all`, every image unused by any container (`docker image prune
    /// -a`) — and refreshes the Images sub-tab so the result shows up
    /// immediately, without waiting for the next 2s poll tick.
    pub async fn prune_images(self, all: bool) -> EffectOutcome {
        let msg = match self.client.rpc(Command::PruneImages { all }).await {
            Ok(Response::PruneResult { count, reclaimed_bytes }) => {
                format!("{count} imagem(ns) removida(s) · {} liberados", view::fmt_bytes(reclaimed_bytes))
            }
            Ok(other) => view::resp_msg(&other),
            Err(e) => format!("erro: {e}"),
        };
        let mut pairs = vec![("docker_msg".into(), msg.clone())];
        if let Ok(Response::DockerImages(list)) = self.client.rpc(Command::DockerImages).await {
            pairs.push(("docker_images".into(), view::docker_images_json(&list, "")));
            pairs.push(("docker_images_count".into(), list.len().to_string()));
        }
        outcome_toast(pairs, &msg)
    }

    /// Removes volumes referenced by no container — or, with `all`, every
    /// unused volume rather than just anonymous ones (`docker volume prune
    /// --all`) — and refreshes the Volumes sub-tab.
    pub async fn prune_volumes(self, all: bool) -> EffectOutcome {
        let msg = match self.client.rpc(Command::PruneVolumes { all }).await {
            Ok(Response::PruneResult { count, reclaimed_bytes }) => {
                format!("{count} volume(s) removido(s) · {} liberados", view::fmt_bytes(reclaimed_bytes))
            }
            Ok(other) => view::resp_msg(&other),
            Err(e) => format!("erro: {e}"),
        };
        let mut pairs = vec![("docker_msg".into(), msg.clone())];
        if let Ok(Response::DockerVolumes(list)) = self.client.rpc(Command::DockerVolumes).await {
            pairs.push(("docker_volumes".into(), view::docker_volumes_json(&list, "")));
            pairs.push(("docker_volumes_count".into(), list.len().to_string()));
        }
        outcome_toast(pairs, &msg)
    }

    /// Removes networks attached to no container (rustploy's own per-project
    /// networks included, once their last service is gone) and refreshes the
    /// Networks sub-tab.
    pub async fn prune_networks(self) -> EffectOutcome {
        let msg = match self.client.rpc(Command::PruneNetworks).await {
            Ok(Response::PruneResult { count, .. }) => format!("{count} rede(s) removida(s)"),
            Ok(other) => view::resp_msg(&other),
            Err(e) => format!("erro: {e}"),
        };
        let mut pairs = vec![("docker_msg".into(), msg.clone())];
        if let Ok(Response::DockerNetworks(list)) = self.client.rpc(Command::DockerNetworks).await {
            pairs.push(("docker_networks".into(), view::docker_networks_json(&list, "")));
            pairs.push(("docker_networks_count".into(), list.len().to_string()));
        }
        outcome_toast(pairs, &msg)
    }
}
