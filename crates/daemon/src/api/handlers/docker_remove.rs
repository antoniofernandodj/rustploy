//! Remoção INDIVIDUAL de um recurso Docker (o par por-item dos `docker_prune`):
//! um container, imagem, volume ou rede específicos, escolhidos na aba Docker.
//! Sem `force` — o Docker recusa remover recursos em uso e o erro é propagado
//! ao cliente (a UI mostra a mensagem), evitando remoção acidental do que está
//! em uso. Invalida os caches de inventário afetados para a aba refletir na hora.

use crate::api::AppState;
use bollard::{
    container::RemoveContainerOptions, image::RemoveImageOptions, volume::RemoveVolumeOptions,
};
use shared::Response as RpResponse;

/// `docker rm <id>` (sem `-f`): remove um container PARADO. `v: true` remove os
/// volumes anônimos associados (como o prune). Um container rodando é recusado
/// pelo Docker (409) — a UI só oferece "Remover" nos parados, mas o daemon não
/// força de qualquer forma.
pub async fn remove_container(state: AppState, id: String) -> RpResponse {
    let opts = RemoveContainerOptions { v: true, ..Default::default() };
    match state.docker.inner.remove_container(&id, Some(opts)).await {
        Ok(()) => {
            // Remover um container muda contagens de imagem em-uso e anexos de rede.
            state.docker_cache.df.invalidate().await;
            state.docker_cache.networks.invalidate().await;
            RpResponse::Ok
        }
        Err(e) => RpResponse::err("DockerError", e.to_string()),
    }
}

/// `docker rmi <id>` (sem `-f`): remove uma imagem sem uso. Uma imagem
/// referenciada por algum container é recusada pelo Docker.
pub async fn remove_image(state: AppState, id: String) -> RpResponse {
    let opts = RemoveImageOptions { force: false, noprune: false };
    match state.docker.inner.remove_image(&id, Some(opts), None).await {
        Ok(_) => {
            state.docker_cache.df.invalidate().await;
            RpResponse::Ok
        }
        Err(e) => RpResponse::err("DockerError", e.to_string()),
    }
}

/// `docker volume rm <name>` (sem `-f`): remove um volume sem uso.
pub async fn remove_volume(state: AppState, name: String) -> RpResponse {
    match state.docker.inner.remove_volume(&name, Some(RemoveVolumeOptions { force: false })).await {
        Ok(()) => {
            state.docker_cache.df.invalidate().await;
            RpResponse::Ok
        }
        Err(e) => RpResponse::err("DockerError", e.to_string()),
    }
}

/// `docker network rm <id>`: remove uma rede sem uso. Uma rede com container
/// anexado (ou uma predefinida: bridge/host/none) é recusada pelo Docker.
pub async fn remove_network(state: AppState, id: String) -> RpResponse {
    match state.docker.inner.remove_network(&id).await {
        Ok(()) => {
            state.docker_cache.networks.invalidate().await;
            RpResponse::Ok
        }
        Err(e) => RpResponse::err("DockerError", e.to_string()),
    }
}
