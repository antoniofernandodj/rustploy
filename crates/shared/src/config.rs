use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Process-wide configuration singleton. Loaded once (files + env vars) on first
/// access and shared everywhere via [`RustployConfig::global`].
static CONFIG: OnceLock<RustployConfig> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvBackupConfig {
    /// Directório onde os snapshots são gravados.
    /// Padrão: <db_path>/env_backups/
    pub dir: Option<String>,
    /// Intervalo entre backups em segundos. Padrão: 60.
    #[serde(default = "default_env_backup_interval")]
    pub interval_secs: u64,
}

fn default_env_backup_interval() -> u64 { 60 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustployConfig {
    pub daemon: DaemonConfig,
    pub ingress: IngressConfig,
    pub docker: DockerConfig,
    pub deploy: DeployConfig,
    pub metrics: MetricsConfig,
    pub secrets: SecretsConfig,
    #[serde(default)]
    pub rwp: RwpConfig,
    #[serde(default)]
    pub env_backup: EnvBackupConfig,
}

/// Configuration for the RWP remote administrative channel (TCP).
/// Disabled by default; when enabled without a token it only binds to
/// loopback. Binding to a non-loopback address requires a token to be set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RwpConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_rwp_bind")]
    pub bind_address: String,
    #[serde(default = "default_rwp_port")]
    pub port: u16,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default = "default_rwp_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_rwp_max_frame_size")]
    pub max_frame_size: usize,
    #[serde(default = "default_rwp_read_timeout_secs")]
    pub read_timeout_secs: u64,
    #[serde(default = "default_rwp_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
}

fn default_rwp_bind() -> String {
    "127.0.0.1".into()
}
fn default_rwp_port() -> u16 {
    8787
}
fn default_rwp_max_connections() -> usize {
    8
}
fn default_rwp_max_frame_size() -> usize {
    1024 * 1024
}
fn default_rwp_read_timeout_secs() -> u64 {
    15
}
fn default_rwp_idle_timeout_secs() -> u64 {
    120
}

impl Default for RwpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind_address: default_rwp_bind(),
            port: default_rwp_port(),
            token: None,
            max_connections: default_rwp_max_connections(),
            max_frame_size: default_rwp_max_frame_size(),
            read_timeout_secs: default_rwp_read_timeout_secs(),
            idle_timeout_secs: default_rwp_idle_timeout_secs(),
        }
    }
}

impl RwpConfig {
    /// True when the configured bind address is not a loopback address.
    pub fn is_public_bind(&self) -> bool {
        match self.bind_address.parse::<std::net::IpAddr>() {
            Ok(ip) => !ip.is_loopback(),
            // Hostnames or "0.0.0.0"/"::" that fail to parse as loopback are
            // treated as public to stay on the safe side.
            Err(_) => self.bind_address != "localhost",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub socket_path: String,
    pub db_path: String,
    pub log_level: String,
    pub webhook_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressConfig {
    pub http_port: u16,
    pub https_port: u16,
    pub bind_address: String,
    pub acme: AcmeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcmeConfig {
    pub enabled: bool,
    pub email: Option<String>,
    pub directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    pub socket_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployConfig {
    pub drain_secs: u64,
    pub image_cache: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    pub interval_secs: u64,
    pub history_points: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    pub master_key_path: String,
}

impl Default for RustployConfig {
    fn default() -> Self {
        Self {
            daemon: DaemonConfig {
                socket_path: "/run/rustploy/rustploy.sock".into(),
                db_path: "/var/lib/rustploy/db".into(),
                log_level: "info".into(),
                // 8788 fica ao lado do RWP (8787). Evita 9000/9001, comuns em
                // MinIO/rustfs e outros serviços S3.
                webhook_port: 8788,
            },
            ingress: IngressConfig {
                http_port: 8080,
                https_port: 443,
                bind_address: "0.0.0.0".into(),
                acme: AcmeConfig {
                    enabled: true,
                    email: None,
                    directory: "https://acme-v02.api.letsencrypt.org/directory".into(),
                },
            },
            docker: DockerConfig {
                socket_path: "/var/run/docker.sock".into(),
            },
            deploy: DeployConfig {
                drain_secs: 10,
                image_cache: 2,
            },
            metrics: MetricsConfig {
                interval_secs: 2,
                history_points: 60,
            },
            secrets: SecretsConfig {
                master_key_path: "/etc/rustploy/master.key".into(),
            },
            rwp: RwpConfig::default(),
            env_backup: EnvBackupConfig::default(),
        }
    }
}

impl RustployConfig {
    /// Returns the process-wide config, loading it on first call.
    ///
    /// This is the single entry point every binary should use so that all
    /// `RUSTPLOY_*` environment variables are read in exactly one place.
    pub fn global() -> &'static RustployConfig {
        CONFIG.get_or_init(Self::load)
    }

    pub fn load() -> Self {
        let paths = [
            std::env::var("RUSTPLOY_CONFIG")
                .ok()
                .map(std::path::PathBuf::from),
            Some(std::path::PathBuf::from("/etc/rustploy/config.toml")),
            dirs_config_path(),
        ];

        for path in paths.into_iter().flatten() {
            if let Ok(contents) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = toml::from_str(&contents) {
                    return Self::apply_env_overrides(cfg);
                }
            }
        }

        Self::apply_env_overrides(Self::default())
    }

    fn apply_env_overrides(mut cfg: Self) -> Self {
        // `RUSTPLOY_SOCKET` is the historical client-side alias for the socket
        // path; accept both so there is a single source of truth.
        if let Ok(v) = std::env::var("RUSTPLOY_SOCKET_PATH").or_else(|_| std::env::var("RUSTPLOY_SOCKET")) {
            cfg.daemon.socket_path = v;
        }
        if let Ok(v) = std::env::var("RUSTPLOY_DB_PATH") {
            cfg.daemon.db_path = v;
        }
        if let Ok(v) = std::env::var("RUSTPLOY_LOG_LEVEL") {
            cfg.daemon.log_level = v;
        }
        if let Ok(v) = std::env::var("RUSTPLOY_HTTP_PORT") {
            if let Ok(p) = v.parse() {
                cfg.ingress.http_port = p;
            }
        }
        if let Ok(v) = std::env::var("RUSTPLOY_WEBHOOK_PORT") {
            if let Ok(p) = v.parse() {
                cfg.daemon.webhook_port = p;
            }
        }
        if let Ok(v) = std::env::var("RUSTPLOY_RWP_ENABLED") {
            cfg.rwp.enabled = matches!(v.as_str(), "1" | "true" | "yes" | "on");
        }
        if let Ok(v) = std::env::var("RUSTPLOY_RWP_BIND") {
            cfg.rwp.bind_address = v;
        }
        if let Ok(v) = std::env::var("RUSTPLOY_RWP_PORT") {
            if let Ok(p) = v.parse() {
                cfg.rwp.port = p;
            }
        }
        if let Ok(v) = std::env::var("RUSTPLOY_RWP_TOKEN") {
            cfg.rwp.token = Some(v).filter(|s| !s.is_empty());
        }
        cfg
    }

    /// Default RWP address a remote client should dial, derived from config.
    pub fn rwp_address(&self) -> String {
        format!("{}:{}", self.rwp.bind_address, self.rwp.port)
    }

    /// Ordered list of Unix socket paths a local client should try, from the
    /// most specific (configured/override) to the writable fallback. Centralizes
    /// every `HOME`/`RUSTPLOY_SOCKET` read for socket resolution.
    pub fn client_socket_candidates(&self) -> Vec<String> {
        let mut out = vec![self.daemon.socket_path.clone()];
        if let Some(home) = user_home() {
            out.push(format!("{home}/.local/share/rustploy/rustploy.sock"));
        }
        out.dedup();
        out
    }
}

/// The single place the process reads `$HOME`.
pub fn user_home() -> Option<String> {
    std::env::var("HOME").ok().filter(|s| !s.is_empty())
}

/// `~/.local/share/rustploy`, falling back to `/tmp` when `$HOME` is unset.
pub fn fallback_data_dir() -> PathBuf {
    let home = user_home().unwrap_or_else(|| "/tmp".into());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("rustploy")
}

fn dirs_config_path() -> Option<PathBuf> {
    user_home().map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join("rustploy")
            .join("config.toml")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A partial `[rwp]` block (only the fields the installer/admin cares about)
    /// must parse, with the remaining fields falling back to their defaults.
    #[test]
    fn partial_rwp_block_uses_field_defaults() {
        let toml_str = r#"
[rwp]
enabled = true
token = "abc123"
"#;
        let cfg: RwpConfig = toml::from_str(toml_str)
            .map(|w: WrapRwp| w.rwp)
            .expect("partial [rwp] must parse");
        assert!(cfg.enabled);
        assert_eq!(cfg.token.as_deref(), Some("abc123"));
        assert_eq!(cfg.port, 8787);
        assert_eq!(cfg.bind_address, "127.0.0.1");
        assert_eq!(cfg.max_connections, 8);
        assert_eq!(cfg.idle_timeout_secs, 120);
    }

    #[derive(Deserialize)]
    struct WrapRwp {
        rwp: RwpConfig,
    }
}
