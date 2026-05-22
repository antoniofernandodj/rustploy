use crate::models::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    // Projects
    ProjectCreate { name: String, description: Option<String> },
    ProjectDelete { id: String },
    ProjectList,
    ProjectEnvSet { project_id: String, env_vars: Vec<EnvVar> },

    // Services
    ServiceCreate(ServiceSpec),
    ServiceUpdate { id: String, spec: ServiceSpec },
    ServiceDelete { id: String },
    ServiceList { project_id: String },
    ServiceGet { id: String },

    // Deployments
    DeployStart { service_id: String },
    DeployAbort { deployment_id: String },
    DeployRollback { service_id: String },
    DeployHistory { service_id: String, limit: usize },

    // Observability
    LogsSubscribe { service_id: String, tail: usize },
    LogsUnsubscribe { service_id: String },
    MetricsSubscribe { service_id: String },
    MetricsUnsubscribe { service_id: String },

    // Infrastructure
    Ping,
    DaemonStatus,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogStream {
    Stdout,
    Stderr,
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
    DaemonStatus(DaemonStatus),
    Pong { uptime_secs: u64 },
    Err { code: String, message: String },
}

impl Response {
    pub fn err(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Err { code: code.into(), message: message.into() }
    }
}
