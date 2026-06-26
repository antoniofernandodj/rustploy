pub mod handlers;
pub mod routes;
pub mod server;
pub mod webhook_server;

use crate::{
    db::Db, docker::DockerClient, event_bus::EventBus, ingress::{IngressController, TlsManager},
    secrets::SecretsManager,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// Pending OAuth handshakes: CSRF `state` → `provider_id`, consumed by the
/// `/oauth/gitea/callback` route once the user authorizes.
pub type OAuthStates = Arc<Mutex<HashMap<String, String>>>;

/// Handles de abort para deploys activos: deployment_id → AbortHandle.
/// Permite cancelar a task do executor ao receber DeployAbort.
pub type ActiveDeploys = Arc<Mutex<HashMap<String, tokio::task::AbortHandle>>>;

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
    pub webhook_port: u16,
    pub started_at: std::time::Instant,
    pub oauth_states: OAuthStates,
    pub active_deploys: ActiveDeploys,
}

impl AppState {
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
        webhook_port: u16,
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
            webhook_port,
            started_at: std::time::Instant::now(),
            oauth_states: Arc::new(Mutex::new(HashMap::new())),
            active_deploys: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
