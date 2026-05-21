use chrono::{DateTime, Utc};
use shared::{ContainerMetricsPoint, Deployment, DeployState, Event, Project, Service};
use std::collections::{HashMap, VecDeque};

pub const MAX_LOG_LINES: usize = 2000;
pub const MAX_METRIC_POINTS: usize = 60;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Dashboard,
    ServiceDetail,
    DeployProgress(String),
    Logs(String),
    Metrics(String),
    Confirm { message: String, action: ConfirmAction },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    DeleteProject(String),
    DeleteService(String),
    AbortDeploy(String),
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: DateTime<Utc>,
    pub text: String,
    pub is_stderr: bool,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub message: String,
    pub is_error: bool,
    pub expires_at: std::time::Instant,
}

pub struct App {
    pub screen: Screen,
    pub projects: Vec<Project>,
    pub services: Vec<Service>,
    pub selected_project: usize,
    pub selected_service: usize,
    pub deploy_progress: HashMap<String, DeployProgressState>,
    pub logs: HashMap<String, VecDeque<LogLine>>,
    pub metrics: HashMap<String, VecDeque<ContainerMetricsPoint>>,
    pub notification: Option<Notification>,
    pub last_deployment: Option<Deployment>,
}

#[derive(Debug, Clone)]
pub struct DeployProgressState {
    pub deployment_id: String,
    pub service_id: String,
    pub current_state: DeployState,
    pub percent: u8,
    pub description: String,
    pub states_seen: Vec<DeployState>,
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Dashboard,
            projects: vec![],
            services: vec![],
            selected_project: 0,
            selected_service: 0,
            deploy_progress: HashMap::new(),
            logs: HashMap::new(),
            metrics: HashMap::new(),
            notification: None,
            last_deployment: None,
        }
    }

    pub fn apply_event(&mut self, event: Event) {
        match event {
            Event::ServiceStatusChanged { service_id, status } => {
                if let Some(svc) = self.services.iter_mut().find(|s| s.id == service_id) {
                    svc.status = status;
                }
            }

            Event::DeployStateChanged { deployment_id, service_id, state, .. } => {
                let entry = self
                    .deploy_progress
                    .entry(deployment_id.clone())
                    .or_insert_with(|| DeployProgressState {
                        deployment_id: deployment_id.clone(),
                        service_id: service_id.clone(),
                        current_state: state.clone(),
                        percent: 0,
                        description: String::new(),
                        states_seen: vec![],
                    });
                entry.states_seen.push(entry.current_state.clone());
                entry.current_state = state;
                entry.percent = state_to_percent(&entry.current_state);
            }

            Event::DeployProgress { deployment_id, service_id: _, percent, description, .. } => {
                if let Some(p) = self.deploy_progress.get_mut(&deployment_id) {
                    p.percent = percent;
                    p.description = description;
                }
            }

            Event::LogLine { service_id, stream, line, timestamp, .. } => {
                let buf = self.logs.entry(service_id).or_default();
                if buf.len() >= MAX_LOG_LINES {
                    buf.pop_front();
                }
                buf.push_back(LogLine {
                    timestamp,
                    text: line,
                    is_stderr: stream == shared::protocol::LogStream::Stderr,
                });
            }

            Event::ContainerMetrics(m) => {
                let buf = self.metrics.entry(m.service_id.clone()).or_default();
                if buf.len() >= MAX_METRIC_POINTS {
                    buf.pop_front();
                }
                buf.push_back(m);
            }

            Event::Error { message, .. } => {
                self.set_notification(message, true);
            }

            Event::DaemonReady { version } => {
                self.set_notification(format!("daemon {version} ready"), false);
            }
        }
    }

    pub fn set_notification(&mut self, msg: impl Into<String>, is_error: bool) {
        self.notification = Some(Notification {
            message: msg.into(),
            is_error,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(4),
        });
    }

    pub fn current_project(&self) -> Option<&Project> {
        self.projects.get(self.selected_project)
    }

    pub fn current_service(&self) -> Option<&Service> {
        self.services.get(self.selected_service)
    }

    pub fn tick(&mut self) {
        if let Some(n) = &self.notification {
            if n.expires_at <= std::time::Instant::now() {
                self.notification = None;
            }
        }
    }
}

fn state_to_percent(state: &DeployState) -> u8 {
    match state {
        DeployState::Pending => 5,
        DeployState::ResolvingDeps => 10,
        DeployState::PullingImage => 30,
        DeployState::CloningRepo => 20,
        DeployState::BuildingImage => 50,
        DeployState::Staging => 65,
        DeployState::HealthcheckPolling => 75,
        DeployState::SwappingIn => 85,
        DeployState::Draining => 90,
        DeployState::Promoting => 95,
        DeployState::Live => 100,
        DeployState::RollingBack => 0,
        DeployState::Failed => 0,
        DeployState::Pruning => 100,
    }
}
