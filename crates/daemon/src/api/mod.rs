pub mod handlers;
pub mod routes;
pub mod server;

use crate::{
    db::Db,
    docker::DockerClient,
    event_bus::EventBus,
    ingress::IngressController,
    secrets::SecretsManager,
};
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub docker: Arc<DockerClient>,
    pub ingress: Arc<IngressController>,
    pub bus: Arc<EventBus>,
    pub secrets: Arc<SecretsManager>,
    pub db_path: PathBuf,
    pub drain_secs: u64,
    pub started_at: std::time::Instant,
}

impl AppState {
    pub fn new(
        db: Arc<Db>,
        docker: Arc<DockerClient>,
        ingress: Arc<IngressController>,
        bus: Arc<EventBus>,
        secrets: Arc<SecretsManager>,
        db_path: PathBuf,
        drain_secs: u64,
    ) -> Self {
        Self {
            db,
            docker,
            ingress,
            bus,
            secrets,
            db_path,
            drain_secs,
            started_at: std::time::Instant::now(),
        }
    }
}
