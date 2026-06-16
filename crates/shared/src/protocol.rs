use crate::manifest::ApplyReport;
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
        acme_email: Option<String>,
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

    // Infra-as-Code (manifesto declarativo)
    /// Reconcilia projetos/serviços a partir de manifestos YAML já interpolados
    /// pelo cliente (um documento `ProjectManifest` por string). Aditivo:
    /// cria/atualiza, nunca deleta. Não dispara deploy.
    ///
    /// Os manifestos trafegam como YAML (e não como structs) porque o postcard é
    /// um formato não auto-descritivo e quebra com `skip_serializing_if`/defaults;
    /// o daemon faz o parse com `serde_yaml`.
    ManifestApply {
        manifests: Vec<String>,
        /// Deleta serviços que existem no projeto mas não constam no manifesto.
        prune: bool,
        /// Dispara deploy dos serviços criados/alterados após sincronizar.
        deploy: bool,
    },
    /// Exporta o estado atual de um projeto como manifesto YAML (secrets redigidos).
    ManifestExport {
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
    DaemonSettings { webhook_base_url: Option<String>, acme_email: Option<String> },
    SecretNames(Vec<String>),
    ManifestReport(ApplyReport),
    /// Manifesto YAML serializado (resposta de `ManifestExport`).
    Manifest(String),
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

// ---------------------------------------------------------------------------
// RWP — Rustploy Wire Protocol (remote administrative channel over TCP)
//
// Same `[u32 LE length][postcard payload]` framing as the local UDS channel,
// but wrapped in a thin envelope that adds a version handshake and optional
// token authentication. It reuses `Command`, `Response` and `Event` directly,
// so every command the TUI can issue is available remotely with no extra code.
// ---------------------------------------------------------------------------

/// Bumped on any breaking change to `RwpFrame` / `RwpReply` shape.
pub const RWP_PROTOCOL_VERSION: u16 = 1;

/// A frame sent by a remote client to the daemon over RWP.
///
/// Expected lifecycle on a connection:
/// `Hello` → (`Authenticate` if the daemon requires it) → then either an
/// indefinite sequence of `Rpc`/`Ping`, or a single `Subscribe` that turns the
/// connection into an event stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RwpFrame {
    /// First frame on every connection. Negotiates protocol version.
    Hello {
        protocol_version: u16,
        client_version: String,
    },
    /// Sent after `Hello` when the daemon reported `auth_required = true`.
    Authenticate { token: String },
    /// A single administrative call; the daemon replies with `RwpReply::Response`.
    Rpc(Command),
    /// Turns the connection into a one-way stream of `RwpReply::Event` frames.
    Subscribe { service_id: Option<String> },
    /// Liveness probe; the daemon replies with `RwpReply::Pong`.
    Ping,
}

/// A frame sent by the daemon to a remote client over RWP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RwpReply {
    /// Response to `Hello`. `auth_required` tells the client whether it must
    /// send `Authenticate` before any other frame is accepted.
    HelloAck {
        protocol_version: u16,
        daemon_version: String,
        auth_required: bool,
    },
    /// Authentication accepted; the connection may now issue commands.
    AuthOk,
    Response(Response),
    Event(Event),
    Pong { uptime_secs: u64 },
    Error(RwpError),
}

/// Protocol-level error (distinct from a command-level `Response::Err`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RwpError {
    pub code: String,
    pub message: String,
}

impl RwpError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
