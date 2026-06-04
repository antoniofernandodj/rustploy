use crate::models::*;
use serde::{Deserialize, Serialize};

/// First frame sent by the client on every new connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientFrame {
    /// Single RPC call: daemon replies with one `Response` frame then closes.
    Rpc(Command),
    /// Event stream: client sends this once, daemon replies with `Event`
    /// frames indefinitely until the connection is dropped.
    Subscribe { service_id: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    // Projects
    ProjectCreate {
        name: String,
        description: Option<String>,
    },
    ProjectDelete {
        id: String,
    },
    ProjectUpdate {
        id: String,
        name: String,
        description: Option<String>,
    },
    ProjectList,
    ProjectEnvSet {
        project_id: String,
        env_vars: Vec<EnvVar>,
    },

    // Services
    ServiceCreate(ServiceSpec),
    ServiceUpdate {
        id: String,
        spec: ServiceSpec,
    },
    ServiceDelete {
        id: String,
    },
    ServiceList {
        project_id: String,
    },
    ServiceGet {
        id: String,
    },

    // Deployments
    DeployStart {
        service_id: String,
    },
    DeployAbort {
        deployment_id: String,
    },
    DeployRollback {
        service_id: String,
    },
    DeployHistory {
        service_id: String,
        limit: usize,
    },

    // Service lifecycle
    ServiceStop {
        service_id: String,
    },
    ServiceReload {
        service_id: String,
    },

    // Global views
    RecentDeployments {
        limit: usize,
    },
    GetBuildLogs {
        deployment_id: String,
    },

    // Observability
    LogsGet {
        service_id: String,
        tail: usize,
    },
    LogsSubscribe {
        service_id: String,
        tail: usize,
    },
    LogsUnsubscribe {
        service_id: String,
    },
    MetricsSubscribe {
        service_id: String,
    },
    MetricsUnsubscribe {
        service_id: String,
    },

    // Webhooks
    GetWebhookUrl {
        service_id: String,
    },
    RegenerateWebhookToken {
        service_id: String,
    },
    GetDaemonSettings,
    SetDaemonSettings {
        webhook_base_url: Option<String>,
    },

    // Secrets
    SecretSet {
        project_id: String,
        name: String,
        value: String,
    },
    SecretDelete {
        project_id: String,
        name: String,
    },
    SecretList {
        project_id: String,
    },

    // Infrastructure
    Ping,
    DaemonStatus,
    DeployEngineStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    DeployStateChanged {
        deployment_id: String,
        service_id: String,
        state: DeployState,
        timestamp: chrono::DateTime<chrono::Utc>,
        message: Option<String>,
    },
    DeployProgress {
        deployment_id: String,
        service_id: String,
        phase: String,
        percent: u8,
        description: String,
    },
    /// Output from `docker build` — belongs to a specific deployment.
    BuildLog {
        deployment_id: String,
        service_id: String,
        line: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    /// stdout/stderr of the running container — belongs to the service.
    LogLine {
        service_id: String,
        container_id: String,
        stream: LogStream,
        line: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    ContainerMetrics(ContainerMetricsPoint),
    ServiceStatusChanged {
        service_id: String,
        status: ServiceStatus,
    },
    DaemonReady {
        version: String,
    },
    Error {
        code: String,
        message: String,
    },
}

impl Event {
    pub fn matches(&self, service_id: &str) -> bool {
        match self {
            Event::DeployStateChanged {
                service_id: sid, ..
            } => sid == service_id,
            Event::DeployProgress {
                service_id: sid, ..
            } => sid == service_id,
            Event::BuildLog {
                service_id: sid, ..
            } => sid == service_id,
            Event::LogLine {
                service_id: sid, ..
            } => sid == service_id,
            Event::ContainerMetrics(m) => m.service_id == service_id,
            Event::ServiceStatusChanged {
                service_id: sid, ..
            } => sid == service_id,
            Event::DaemonReady { .. } | Event::Error { .. } => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub stream: LogStream,
    pub line: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildLogLine {
    pub stream: LogStream,
    pub line: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Response {
    Ok,
    Project(Project),
    Projects(Vec<Project>),
    Service(Service),
    Services(Vec<Service>),
    Deployment(Deployment),
    Deployments(Vec<Deployment>),
    Logs(Vec<LogEntry>),
    BuildLogs(Vec<BuildLogLine>),
    DeploymentSummaries(Vec<DeploymentSummary>),
    DaemonStatus(DaemonStatus),
    DeployEngineStatus(DeployEngineSummary),
    Pong { uptime_secs: u64 },
    WebhookUrl(Option<String>),
    DaemonSettings { webhook_base_url: Option<String> },
    SecretNames(Vec<String>),
    Err { code: String, message: String },
}

impl Response {
    pub fn err(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Err {
            code: code.into(),
            message: message.into(),
        }
    }
}
