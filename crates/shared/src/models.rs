use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Variáveis de ambiente herdadas por todos os serviços deste projeto no deploy.
    #[serde(default)]
    pub env_vars: Vec<EnvVar>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceSpec {
    pub name: String,
    pub project_id: String,
    pub source: ServiceSource,
    pub port: u16,
    #[serde(default)]
    pub host_port: Option<u16>,
    pub domain: Option<String>,
    pub env_vars: Vec<EnvVar>,
    pub volumes: Vec<VolumeMount>,
    pub healthcheck: Healthcheck,
    pub replicas: u32,
    pub resources: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceSource {
    Registry { image: String },
    Git(GitSource),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitSource {
    pub url: String,
    pub branch: String,
    pub root_path: String,
    pub watch_paths: Vec<String>,
    pub submodules: bool,
    pub dockerfile_path: String,
    pub build_context: String,
    pub build_stage: Option<String>,
    pub credentials: Option<String>,
}

impl Default for GitSource {
    fn default() -> Self {
        Self {
            url: String::new(),
            branch: "main".into(),
            root_path: ".".into(),
            watch_paths: vec![],
            submodules: false,
            dockerfile_path: "Dockerfile".into(),
            build_context: ".".into(),
            build_stage: None,
            credentials: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Service {
    pub id: String,
    pub spec: ServiceSpec,
    pub status: ServiceStatus,
    pub live_container_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceStatus {
    Stopped,
    Stopping,
    Deploying,
    Running,
    Degraded,
    Error(String),
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(f, "Stopped"),
            Self::Stopping => write!(f, "Stopping"),
            Self::Deploying => write!(f, "Deploying"),
            Self::Running => write!(f, "Running"),
            Self::Degraded => write!(f, "Degraded"),
            Self::Error(msg) => write!(f, "Error: {msg}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Deployment {
    pub id: String,
    pub service_id: String,
    pub image: String,
    pub state: DeployState,
    pub states_log: Vec<StateTransition>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeployState {
    Pending,
    ResolvingDeps,
    PullingImage,
    CloningRepo,
    BuildingImage,
    Staging,
    HealthcheckPolling,
    SwappingIn,
    Draining,
    Promoting,
    Live,
    Stopped,
    RollingBack,
    Failed,
    Pruning,
}

impl DeployState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Live | Self::Stopped | Self::Failed | Self::Pruning)
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Pending => "Pending",
            Self::ResolvingDeps => "ResolvingDeps",
            Self::PullingImage => "PullingImage",
            Self::CloningRepo => "CloningRepo",
            Self::BuildingImage => "BuildingImage",
            Self::Staging => "Staging",
            Self::HealthcheckPolling => "HealthcheckPolling",
            Self::SwappingIn => "SwappingIn",
            Self::Draining => "Draining",
            Self::Promoting => "Promoting",
            Self::Live => "Live",
            Self::Stopped => "Stopped",
            Self::RollingBack => "RollingBack",
            Self::Failed => "Failed",
            Self::Pruning => "Pruning",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateTransition {
    pub from: DeployState,
    pub to: DeployState,
    pub at: DateTime<Utc>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnvVar {
    pub key: String,
    pub value: EnvVarValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EnvVarValue {
    Plain(String),
    Secret(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VolumeMount {
    pub host_path: String,
    pub container_path: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Healthcheck {
    pub kind: HealthcheckKind,
    pub interval_secs: u32,
    pub timeout_secs: u32,
    pub retries: u32,
    pub start_period_secs: u32,
}

impl Default for Healthcheck {
    fn default() -> Self {
        Self {
            kind: HealthcheckKind::Tcp,
            interval_secs: 5,
            timeout_secs: 3,
            retries: 10,
            start_period_secs: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthcheckKind {
    Http { path: String, expected_status: u16 },
    Tcp,
    DockerNative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentSummary {
    pub deployment: Deployment,
    pub service_name: String,
    pub project_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ResourceLimits {
    pub cpu_shares: u64,
    pub mem_limit_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub version: String,
    pub uptime_secs: u64,
    pub services_running: usize,
    pub services_total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMetricsPoint {
    pub service_id: String,
    pub container_id: String,
    pub cpu_percent: f64,
    pub mem_used_bytes: u64,
    pub mem_limit_bytes: u64,
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
    pub timestamp: DateTime<Utc>,
}
