use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RustployConfig {
    pub daemon: DaemonConfig,
    pub ingress: IngressConfig,
    pub docker: DockerConfig,
    pub deploy: DeployConfig,
    pub metrics: MetricsConfig,
    pub secrets: SecretsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub socket_path: String,
    pub db_path: String,
    pub log_level: String,
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
            },
            ingress: IngressConfig {
                http_port: 80,
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
        }
    }
}

impl RustployConfig {
    pub fn load() -> Self {
        let paths = [
            std::env::var("RUSTPLOY_CONFIG").ok().map(std::path::PathBuf::from),
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
        if let Ok(v) = std::env::var("RUSTPLOY_SOCKET_PATH") {
            cfg.daemon.socket_path = v;
        }
        if let Ok(v) = std::env::var("RUSTPLOY_DB_PATH") {
            cfg.daemon.db_path = v;
        }
        if let Ok(v) = std::env::var("RUSTPLOY_LOG_LEVEL") {
            cfg.daemon.log_level = v;
        }
        cfg
    }
}

fn dirs_config_path() -> Option<std::path::PathBuf> {
    std::env::var("HOME").ok().map(|home| {
        std::path::PathBuf::from(home)
            .join(".config")
            .join("rustploy")
            .join("config.toml")
    })
}
