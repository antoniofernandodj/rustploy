pub mod handlers;
pub mod http_api;
pub mod routes;
pub mod server;
pub mod public_routes;

use crate::{
    db::Db, docker::DockerClient, event_bus::EventBus, ingress::{IngressController, TlsManager},
    secrets::SecretsManager,
};
use shared::{DockerImageInfo, DockerNetworkInfo, DockerVolumeInfo};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::Mutex as AsyncMutex;

/// Pending OAuth handshakes: CSRF `state` → `provider_id`, consumed by the
/// `/oauth/gitea/callback` route once the user authorizes.
pub type OAuthStates = Arc<Mutex<HashMap<String, String>>>;

/// Handles de abort para deploys activos: deployment_id → AbortHandle.
/// Permite cancelar a task do executor ao receber DeployAbort.
pub type ActiveDeploys = Arc<Mutex<HashMap<String, tokio::task::AbortHandle>>>;

/// How long the host-wide Docker inventory (`docker system df` + the network
/// cross-reference) stays cached before the next request re-hits the Docker
/// Engine. These calls are slow (hundreds of ms to seconds) and the 2s status
/// poll would otherwise fire them every tick; the inventory changes rarely, so
/// a generous TTL cuts almost all of that Docker load. Busted early by the
/// prune handlers so a cleanup is reflected at once.
const DOCKER_CACHE_TTL: Duration = Duration::from_secs(300);

/// Single-slot value cache with a TTL. The async lock is held across a
/// miss-refresh so concurrent callers coalesce into ONE upstream call instead
/// of a thundering herd — the whole point, since the upstream here is a slow
/// `docker system df`.
pub struct TtlCache<T> {
    ttl: Duration,
    slot: AsyncMutex<Option<(Instant, T)>>,
}

impl<T: Clone> TtlCache<T> {
    fn new(ttl: Duration) -> Self {
        Self { ttl, slot: AsyncMutex::new(None) }
    }

    /// Returns the cached value if still within the TTL, otherwise runs
    /// `refresh`, stores and returns it. On a `refresh` error any previously
    /// cached (now stale) value is left untouched and the error is propagated.
    pub async fn get_or_refresh<F, Fut, E>(&self, refresh: F) -> Result<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        let mut slot = self.slot.lock().await;
        if let Some((at, v)) = slot.as_ref()
            && at.elapsed() < self.ttl
        {
            return Ok(v.clone());
        }
        let v = refresh().await?;
        *slot = Some((Instant::now(), v.clone()));
        Ok(v)
    }

    /// Drops the cached value so the next `get_or_refresh` fetches fresh.
    pub async fn invalidate(&self) {
        *self.slot.lock().await = None;
    }
}

/// Caches the slow host-wide Docker inventory calls so the 2s status poll (and
/// every Docker-tab refresh) serves from RAM instead of re-hitting the Docker
/// Engine. See [`DOCKER_CACHE_TTL`].
pub struct DockerCache {
    /// `docker system df` feeds BOTH the Images and Volumes tabs — cached
    /// together so a single refresh serves both handlers.
    pub df: TtlCache<(Vec<DockerImageInfo>, Vec<DockerVolumeInfo>)>,
    /// The network list cross-referenced against every container's attachments.
    pub networks: TtlCache<Vec<DockerNetworkInfo>>,
}

impl DockerCache {
    fn new() -> Self {
        Self {
            df: TtlCache::new(DOCKER_CACHE_TTL),
            networks: TtlCache::new(DOCKER_CACHE_TTL),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub docker: Arc<DockerClient>,
    pub ingress: Arc<IngressController>,
    pub bus: Arc<EventBus>,
    pub secrets: Arc<SecretsManager>,
    pub tls: Arc<TlsManager>,
    pub db_path: PathBuf,
    pub backup_dir: PathBuf,
    pub drain_secs: u64,
    /// Config do listener HTTP da API. Guardada porque é dela que sai a URL
    /// pública do daemon (ver [`AppState::public_base_url`]) — desde a
    /// unificação, webhook e callback OAuth são servidos por esse mesmo
    /// listener, então a base deles é a base da API.
    pub api: shared::ApiConfig,
    pub started_at: std::time::Instant,
    pub oauth_states: OAuthStates,
    pub active_deploys: ActiveDeploys,
    /// Fila global de deploys (um por vez). `deploy_start` enfileira aqui e o
    /// worker (`crate::deploy::queue::run_worker`) puxa serialmente.
    pub deploy_queue: Arc<crate::deploy::queue::DeployQueue>,
    /// TTL cache for the slow host-wide Docker inventory (see [`DockerCache`]).
    pub docker_cache: Arc<DockerCache>,
    /// Storage do registry OCI embutido — o MESMO (mesma `commit_lock`) que o
    /// listener HTTP do registry usa, para o handler `RegistryGc`. `None`
    /// quando `[registry]` está desabilitado na config.
    pub registry_storage: Option<Arc<crate::registry::storage::RegistryStorage>>,
    /// Token interno `rp-internal`, regenerado a cada boot (ver
    /// `crate::registry::internal_token`), usado pelo `DeployExecutor` pra se
    /// autenticar sozinho ao puxar imagens do registry embutido.
    pub registry_internal_token: Option<Arc<str>>,
}

impl AppState {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        db: Arc<Db>,
        docker: Arc<DockerClient>,
        ingress: Arc<IngressController>,
        bus: Arc<EventBus>,
        secrets: Arc<SecretsManager>,
        tls: Arc<TlsManager>,
        db_path: PathBuf,
        backup_dir: PathBuf,
        drain_secs: u64,
        api: shared::ApiConfig,
        registry_storage: Option<Arc<crate::registry::storage::RegistryStorage>>,
        registry_internal_token: Option<Arc<str>>,
    ) -> Self {
        Self {
            db,
            docker,
            ingress,
            bus,
            secrets,
            tls,
            db_path,
            backup_dir,
            drain_secs,
            api,
            started_at: std::time::Instant::now(),
            oauth_states: Arc::new(Mutex::new(HashMap::new())),
            active_deploys: Arc::new(Mutex::new(HashMap::new())),
            deploy_queue: crate::deploy::queue::DeployQueue::new(),
            docker_cache: Arc::new(DockerCache::new()),
            registry_storage,
            registry_internal_token,
        }
    }

    /// URL pública do daemon, sem barra final: base do webhook
    /// (`{base}/webhook/{service_id}/{token}`) e do callback OAuth
    /// (`{base}/oauth/gitea/callback`). Derivada de `[api]` — não há setting no
    /// banco para sobrescrevê-la; para mudá-la, configure `api.domain`.
    pub fn public_base_url(&self) -> String {
        self.api.public_base_url(&outbound_ip())
    }
}

/// Detecta o IP de saída da máquina conectando um socket UDP em 8.8.8.8:80
/// (sem enviar dados) e lendo o endereço local escolhido pelo kernel. Usado só
/// como host de fallback quando `api.domain` não está configurado.
fn outbound_ip() -> String {
    use std::net::UdpSocket;
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "localhost".to_string())
}
