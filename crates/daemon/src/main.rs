mod api;
mod db;
mod deploy;
mod docker;
mod event_bus;
mod health;
mod ingress;
mod logs;
mod metrics;
mod secrets;
mod watchdog;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

use anyhow::Result;
use api::AppState;
use event_bus::EventBus;
use ingress::IngressController;
use shared::{Event, RustployConfig};
use socket2::{Domain, Socket, Type};
use std::{os::unix::net::UnixListener as StdUnixListener, path::PathBuf, sync::Arc};
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

    // Database — resolve path with fallback
    let db_path = resolve_data_path(&config.daemon.db_path);
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

    let master_key = resolve_master_key_path(&config.secrets.master_key_path);
    let secrets = Arc::new(secrets::SecretsManager::new(&master_key)?);

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

    // Container log streaming task
    {
        let docker_inner = Arc::new(docker.inner.clone());
        let db2 = db.clone();
        let bus2 = bus.clone();
        tokio::spawn(async move {
            logs::stream_loop(docker_inner, db2, bus2).await;
        });
    }

    bus.publish(Event::DaemonReady {
        version: env!("CARGO_PKG_VERSION").to_string(),
    });

    let state = AppState::new(
        db,
        docker,
        ingress.clone(),
        bus,
        secrets,
        db_path,
        config.deploy.drain_secs,
        config.daemon.webhook_port,
    );

    // Watchdog: detecta containers parados/removidos, tenta restart e redeploy
    {
        let state2 = state.clone();
        tokio::spawn(async move {
            watchdog::watchdog_loop(state2).await;
        });
    }

    // Ingress Proxy: roteamento de domínios e portas
    {
        let routes = ingress.table_handle();
        let http_port = config.ingress.http_port;
        let https_port = config.ingress.https_port;
        tokio::spawn(async move {
            ingress::proxy::start_proxy(routes, http_port, https_port).await;
        });
    }

    // Servidor HTTP para receber webhooks de CI/CD
    {
        let state2 = state.clone();
        let webhook_port = config.daemon.webhook_port;
        tokio::spawn(async move {
            api::webhook_server::run(state2, webhook_port).await;
        });
    }

    // Bind UDS — try configured path, fall back to ~/.local/share/rustploy/
    let socket_path = resolve_socket_path(&config.daemon.socket_path);
    info!(socket = ?socket_path, "listening");

    // Use socket2 to tune UDS buffers before binding. 256 KiB is large enough
    // to absorb a burst of streaming events without blocking the sender, while
    // staying well within L2 cache on typical server hardware.
    let socket = Socket::new(Domain::UNIX, Type::STREAM, None)?;
    socket.set_recv_buffer_size(256 * 1024)?;
    socket.set_send_buffer_size(256 * 1024)?;
    socket.bind(&socket2::SockAddr::unix(&socket_path)?)?;
    socket.listen(128)?;
    let std_listener: StdUnixListener = socket.into();
    std_listener.set_nonblocking(true)?;
    let listener = UnixListener::from_std(std_listener)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o666))?;
    }

    loop {
        let (stream, _) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = api::server::handle_connection(stream, state).await {
                warn!(error = %e, "connection error");
            }
        });
    }
}

fn init_logging(level: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .init();
}

fn fallback_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("rustploy")
}

/// Tries to prepare `configured` for use as a Unix socket path.
/// Falls back to `~/.local/share/rustploy/rustploy.sock` if the
/// configured directory is not writable.
fn resolve_socket_path(configured: &str) -> PathBuf {
    let path = PathBuf::from(configured);
    if can_prepare_socket(&path) {
        return path;
    }
    let fallback = fallback_dir().join("rustploy.sock");
    warn!(
        primary = %path.display(),
        fallback = %fallback.display(),
        "socket path not writable, using fallback"
    );
    let _ = std::fs::create_dir_all(fallback.parent().unwrap());
    if fallback.exists() {
        let _ = std::fs::remove_file(&fallback);
    }
    fallback
}

fn can_prepare_socket(path: &PathBuf) -> bool {
    let parent = match path.parent() {
        Some(p) => p,
        None => return false,
    };
    if std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    if path.exists() {
        if std::fs::remove_file(path).is_err() {
            return false;
        }
    } else {
        // Probe write access by touching a temp file
        let probe = parent.join(".rustploy_probe");
        if std::fs::write(&probe, b"").is_err() {
            return false;
        }
        let _ = std::fs::remove_file(probe);
    }
    true
}

/// Tries to use `configured` as the data directory.
/// Falls back to `~/.local/share/rustploy/db` if the path is not writable.
fn resolve_data_path(configured: &str) -> PathBuf {
    let path = PathBuf::from(configured);
    if can_write_dir(&path) {
        return path;
    }
    let fallback = fallback_dir().join("db");
    warn!(
        primary = %path.display(),
        fallback = %fallback.display(),
        "db path not writable, using fallback"
    );
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

/// Tries to use `configured` as the master key path.
/// Falls back to `~/.local/share/rustploy/master.key` if the directory is
/// not writable (e.g. `/etc/rustploy/` requires root).
fn resolve_master_key_path(configured: &str) -> PathBuf {
    let path = PathBuf::from(configured);
    // If the key already exists and is readable, use it as-is.
    if path.exists() {
        return path;
    }
    // Otherwise we need to be able to create it — check parent writability.
    let parent = path.parent().unwrap_or(&path);
    if can_write_dir(parent) {
        return path;
    }
    let fallback = fallback_dir().join("master.key");
    warn!(
        primary = %path.display(),
        fallback = %fallback.display(),
        "master key directory not writable, using fallback"
    );
    let _ = std::fs::create_dir_all(fallback.parent().unwrap());
    fallback
}

/// Returns true only when `dir` (or its path) is both creatable and writable.
/// Unlike a bare `create_dir_all` check, this actually probes write access
/// even when the directory already exists (e.g. created by a previous root run).
fn can_write_dir(dir: &std::path::Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".rustploy_write_probe");
    if std::fs::write(&probe, b"").is_err() {
        return false;
    }
    let _ = std::fs::remove_file(probe);
    true
}
