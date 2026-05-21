mod api;
mod db;
mod deploy;
mod docker;
mod event_bus;
mod ingress;
mod metrics;
mod secrets;

use api::AppState;
use anyhow::Result;
use event_bus::EventBus;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use hyper_util::service::TowerToHyperService;
use ingress::IngressController;
use shared::{Event, RustployConfig};
use std::{path::PathBuf, sync::Arc};
use tokio::net::UnixListener;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let config = RustployConfig::load();
    init_logging(&config.daemon.log_level);

    info!(
        version = env!("CARGO_PKG_VERSION"),
        socket = config.daemon.socket_path,
        "rustployd starting"
    );

    // Database
    let db_path = PathBuf::from(&config.daemon.db_path);
    let db = Arc::new(db::connect(&db_path).await?);
    info!("database connected");

    // Docker
    let docker = Arc::new(docker::DockerClient::connect(&config.docker.socket_path)?);
    if let Err(e) = docker.ping().await {
        error!(error = %e, "docker engine unreachable");
        std::process::exit(1);
    }
    info!("docker engine connected");

    let bus = Arc::new(EventBus::new());
    let ingress = Arc::new(IngressController::new());

    let master_key = PathBuf::from(&config.secrets.master_key_path);
    let secrets = Arc::new(secrets::SecretsManager::new(&master_key)?);

    // Start ingress proxy (async task)
    let routes = ingress.table_handle();
    let http_port = config.ingress.http_port;
    let https_port = config.ingress.https_port;
    tokio::spawn(async move {
        ingress::proxy::start_proxy(routes, http_port, https_port).await;
    });

    // Recovery
    deploy::recovery::recover(
        db.clone(),
        docker.clone(),
        ingress.clone(),
        bus.clone(),
        secrets.clone(),
        db_path.clone(),
        config.deploy.drain_secs,
    )
    .await;

    // Metrics background task
    {
        let docker_inner = docker.inner.clone();
        let db2 = db.clone();
        let bus2 = bus.clone();
        let interval = config.metrics.interval_secs;
        tokio::spawn(async move {
            metrics::collect_loop(Arc::new(docker_inner), db2, bus2, interval).await;
        });
    }

    bus.publish(Event::DaemonReady {
        version: env!("CARGO_PKG_VERSION").to_string(),
    });

    let state = AppState::new(db, docker, ingress, bus, secrets, db_path, config.deploy.drain_secs);
    let app = api::routes::build(state);

    // Bind UDS
    let socket_path = PathBuf::from(&config.daemon.socket_path);
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)?;
    }
    let listener = UnixListener::bind(&socket_path)?;
    // Allow any local user to connect (group/world write).
    // Production deployments should use 0o660 with a dedicated group.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o666))?;
    }
    info!(socket = ?socket_path, "listening");

    loop {
        let (stream, _) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let svc = TowerToHyperService::new(app.clone());

        tokio::spawn(async move {
            if let Err(e) = auto::Builder::new(TokioExecutor::new())
                .serve_connection(io, svc)
                .await
            {
                warn!(error = %e, "connection error");
            }
        });
    }
}

fn init_logging(level: &str) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    tracing_subscriber::fmt().with_env_filter(filter).json().init();
}
