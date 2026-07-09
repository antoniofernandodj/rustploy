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
    pub api: ApiConfig,
    #[serde(default)]
    pub env_backup: EnvBackupConfig,
}

/// Configuration for the HTTP/JSON + SSE control API — o canal administrativo remoto.
/// Binds to loopback by default and is meant to sit behind the ingress proxy,
/// which terminates TLS for `rustploy.chiquitos.tech` and forwards to it.
/// Binding to a non-loopback address requires a token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    #[serde(default = "default_api_enabled")]
    pub enabled: bool,
    #[serde(default = "default_api_bind")]
    pub bind_address: String,
    #[serde(default = "default_api_port")]
    pub port: u16,
    #[serde(default)]
    pub token: Option<String>,
    /// Domínio público da própria API. Quando definido (não vazio), o listener
    /// da API termina TLS **nesta mesma porta** com um certificado Let's Encrypt
    /// provisionado automaticamente via ACME (requer ACME habilitado + porta 80
    /// acessível pela internet para o desafio HTTP-01). Vazio/`None` = HTTP puro,
    /// para uso local ou atrás de um proxy externo.
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default = "default_api_max_connections")]
    pub max_connections: usize,
}

fn default_api_enabled() -> bool {
    true
}
fn default_api_bind() -> String {
    "127.0.0.1".into()
}
fn default_api_port() -> u16 {
    9797
}
fn default_api_max_connections() -> usize {
    32
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            enabled: default_api_enabled(),
            bind_address: default_api_bind(),
            port: default_api_port(),
            token: None,
            domain: None,
            max_connections: default_api_max_connections(),
        }
    }
}

impl ApiConfig {
    /// True when the configured bind address is not a loopback address.
    pub fn is_public_bind(&self) -> bool {
        match self.bind_address.parse::<std::net::IpAddr>() {
            Ok(ip) => !ip.is_loopback(),
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
                // Porta dedicada de webhook. Evita 9000/9001, comuns em
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
            api: ApiConfig::default(),
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
        // HTTP/JSON + SSE control API.
        if let Ok(v) = std::env::var("RUSTPLOY_API_ENABLED") {
            cfg.api.enabled = matches!(v.as_str(), "1" | "true" | "yes" | "on");
        }
        if let Ok(v) = std::env::var("RUSTPLOY_API_BIND") {
            cfg.api.bind_address = v;
        }
        if let Ok(v) = std::env::var("RUSTPLOY_API_PORT") {
            if let Ok(p) = v.parse() {
                cfg.api.port = p;
            }
        }
        if let Ok(v) = std::env::var("RUSTPLOY_API_TOKEN") {
            cfg.api.token = Some(v).filter(|s| !s.is_empty());
        }
        if let Ok(v) = std::env::var("RUSTPLOY_API_DOMAIN") {
            cfg.api.domain = Some(v).filter(|s| !s.is_empty());
        }
        cfg
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

    /// A partial `[api]` block (only the fields the installer/admin cares about)
    /// must parse, with the remaining fields falling back to their defaults.
    #[test]
    fn partial_api_block_uses_field_defaults() {
        let toml_str = r#"
[api]
token = "abc123"
"#;
        let cfg: ApiConfig = toml::from_str(toml_str)
            .map(|w: WrapApi| w.api)
            .expect("partial [api] must parse");
        assert_eq!(cfg.token.as_deref(), Some("abc123"));
        assert_eq!(cfg.port, 9797);
        assert_eq!(cfg.bind_address, "127.0.0.1");
        assert_eq!(cfg.max_connections, 32);
    }

    #[derive(Deserialize)]
    struct WrapApi {
        api: ApiConfig,
    }
}
