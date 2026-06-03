pub use crate::models::*;

use shared::{
    Command, ContainerMetricsPoint, Deployment, DeploymentSummary, DeployState, Event, Project,
    Response, Service, ServiceStatus,
};
use std::collections::{HashMap, VecDeque};

pub struct App {
    pub focus: Focus,
    pub sidebar_cursor: usize,
    pub view: View,

    pub projects: Vec<Project>,
    pub services: Vec<Service>,
    pub active_project_id: Option<String>,
    pub active_service_id: Option<String>,

    pub creating_project: bool,
    pub new_proj_name: String,
    pub new_proj_desc: String,
    pub new_proj_field: usize,

    pub service_cursor: usize,
    pub service_filter: String,
    pub service_filtering: bool,

    pub project_detail_tab: ProjectDetailTab,
    pub project_env_tab: EnvTabState,

    pub service_tab: ServiceTab,
    pub general_tab: GeneralTabState,
    pub healthcheck_tab: HealthcheckTabState,
    pub domains_tab: DomainsTabState,
    pub advanced_tab: AdvancedTabState,
    pub env_tab: EnvTabState,
    pub deployment_cursor: usize,
    pub build_log_scroll: usize, // offset from top; usize::MAX = follow tail
    pub log_cursor: usize,

    pub new_service: Option<NewServiceState>,

    pub home_deployments: Vec<DeploymentSummary>,
    pub service_deployments: Vec<Deployment>,
    pub deploy_progress: HashMap<String, DeployProgressState>,
    /// Build output per deployment_id — populated from Event::BuildLog.
    pub build_logs: HashMap<String, VecDeque<LogLine>>,
    /// Container stdout/stderr per service_id — populated from Event::LogLine.
    pub logs: HashMap<String, VecDeque<LogLine>>,
    pub metrics: HashMap<String, VecDeque<ContainerMetricsPoint>>,

    pub notification: Option<Notification>,
    pub last_deployment: Option<Deployment>,

    pub pending_commands: Vec<PendingCommand>,
}

impl App {
    pub fn new() -> Self {
        Self {
            focus: Focus::Sidebar,
            sidebar_cursor: 0,
            view: View::HomeDeployments,

            projects: vec![],
            services: vec![],
            active_project_id: None,
            active_service_id: None,

            creating_project: false,
            new_proj_name: String::new(),
            new_proj_desc: String::new(),
            new_proj_field: 0,

            service_cursor: 0,
            service_filter: String::new(),
            service_filtering: false,

            project_detail_tab: ProjectDetailTab::default(),
            project_env_tab: EnvTabState::default(),

            service_tab: ServiceTab::General,
            general_tab: GeneralTabState::default(),
            healthcheck_tab: HealthcheckTabState::default(),
            domains_tab: DomainsTabState::default(),
            advanced_tab: AdvancedTabState::default(),
            env_tab: EnvTabState::default(),
            deployment_cursor: 0,
            build_log_scroll: usize::MAX,
            log_cursor: 0,

            new_service: None,

            home_deployments: vec![],
            service_deployments: vec![],
            deploy_progress: HashMap::new(),
            build_logs: HashMap::new(),
            logs: HashMap::new(),
            metrics: HashMap::new(),

            notification: None,
            last_deployment: None,

            pending_commands: vec![],
        }
    }

    pub fn selectable_sidebar_items(&self) -> Vec<SidebarItem> {
        let mut items = vec![
            SidebarItem::HomeDeployments,
            SidebarItem::HomeMonitoring,
            SidebarItem::HomeSchedules,
            SidebarItem::HomeIngress,
            SidebarItem::HomeDocker,
            SidebarItem::HomeDeployEngine,
            SidebarItem::HomeRequests,
            SidebarItem::NewProject,
        ];
        for i in 0..self.projects.len() {
            items.push(SidebarItem::Project(i));
        }
        items.extend([
            SidebarItem::SettingsWebServer,
            SidebarItem::SettingsProfile,
            SidebarItem::SettingsUsers,
            SidebarItem::SettingsAuditLogs,
            SidebarItem::SettingsSshKeys,
            SidebarItem::SettingsTags,
            SidebarItem::SettingsGit,
            SidebarItem::SettingsRegistry,
            SidebarItem::SettingsS3,
            SidebarItem::SettingsCerts,
            SidebarItem::SettingsSso,
            SidebarItem::Account,
        ]);
        items
    }

    pub fn current_sidebar_item(&self) -> Option<SidebarItem> {
        self.selectable_sidebar_items().into_iter().nth(self.sidebar_cursor)
    }

    pub fn sidebar_move_up(&mut self) {
        if self.sidebar_cursor > 0 {
            self.sidebar_cursor -= 1;
        }
    }

    pub fn sidebar_move_down(&mut self) {
        let max = self.selectable_sidebar_items().len().saturating_sub(1);
        if self.sidebar_cursor < max {
            self.sidebar_cursor += 1;
        }
    }

    pub fn sidebar_select(&mut self) {
        let item = match self.current_sidebar_item() {
            Some(i) => i,
            None => return,
        };

        match &item {
            SidebarItem::HomeDeployments => {
                self.view = View::HomeDeployments;
                self.focus = Focus::Content;
                self.home_deployments.clear();
                self.pending_commands.push(PendingCommand {
                    command: Command::RecentDeployments { limit: 30 },
                    context: CmdContext::LoadHomeDeployments,
                });
            }
            SidebarItem::NewProject => {
                self.creating_project = true;
                self.new_proj_name = String::new();
                self.new_proj_desc = String::new();
                self.new_proj_field = 0;
            }
            SidebarItem::Project(idx) => {
                if let Some(project) = self.projects.get(*idx) {
                    let pid = project.id.clone();
                    self.active_project_id = Some(pid.clone());
                    self.view = View::ProjectDetail;
                    self.project_detail_tab = ProjectDetailTab::Services;
                    self.project_env_tab = EnvTabState::default();
                    self.service_cursor = 0;
                    self.service_filter = String::new();
                    self.service_filtering = false;
                    self.pending_commands.push(PendingCommand {
                        command: Command::ServiceList { project_id: pid },
                        context: CmdContext::LoadServices,
                    });
                }
                self.focus = Focus::Content;
            }
            other => {
                if let Some(view) = other.to_view() {
                    self.view = view;
                }
                self.focus = Focus::Content;
            }
        }
    }

    pub fn filtered_services(&self) -> Vec<&Service> {
        if self.service_filter.is_empty() {
            self.services.iter().collect()
        } else {
            let f = self.service_filter.to_lowercase();
            self.services.iter().filter(|s| s.spec.name.to_lowercase().contains(&f)).collect()
        }
    }

    pub fn current_service(&self) -> Option<&Service> {
        self.filtered_services().into_iter().nth(self.service_cursor)
    }

    pub fn current_project(&self) -> Option<&Project> {
        let pid = self.active_project_id.as_deref()?;
        self.projects.iter().find(|p| p.id == pid)
    }

    pub fn current_active_service(&self) -> Option<&Service> {
        let sid = self.active_service_id.as_deref()?;
        self.services.iter().find(|s| s.id == sid)
    }

    pub fn open_service(&mut self, svc: &Service) {
        self.active_service_id = Some(svc.id.clone());
        self.service_tab = ServiceTab::General;
        self.general_tab = GeneralTabState::from_service(svc);
        self.healthcheck_tab = HealthcheckTabState::from_service(svc);
        self.domains_tab = DomainsTabState::from_service(svc);
        self.advanced_tab = AdvancedTabState::from_service(svc);
        self.env_tab = EnvTabState::default();
        self.deployment_cursor = 0;
        self.build_log_scroll = usize::MAX;
        self.log_cursor = 0;
        self.service_deployments = vec![];
        self.view = View::ServiceDetail;
        self.focus = Focus::Content;
        self.pending_commands.push(PendingCommand {
            command: Command::DeployHistory { service_id: svc.id.clone(), limit: 10 },
            context: CmdContext::LoadDeployments,
        });
        self.pending_commands.push(PendingCommand {
            command: Command::LogsGet { service_id: svc.id.clone(), tail: 500 },
            context: CmdContext::LoadLogs,
        });
    }

    pub fn set_notification(&mut self, msg: impl Into<String>, is_error: bool) {
        self.notification = Some(Notification {
            message: msg.into(),
            is_error,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(4),
        });
    }

    pub fn tick(&mut self) {
        if let Some(n) = &self.notification {
            if n.expires_at <= std::time::Instant::now() {
                self.notification = None;
            }
        }
    }

    pub fn apply_event(&mut self, event: Event) {
        match event {
            Event::ServiceStatusChanged { service_id, status } => {
                if let Some(svc) = self.services.iter_mut().find(|s| s.id == service_id) {
                    svc.status = status;
                }
            }

            Event::DeployStateChanged { deployment_id, service_id, state, message, .. } => {
                if matches!(state, DeployState::RollingBack) {
                    let reason = message.as_deref().unwrap_or("motivo desconhecido");
                    self.set_notification(format!("Deploy falhou: {reason}"), true);
                }

                // Atualiza na home de deployments se estiver carregada.
                if let Some(s) = self.home_deployments.iter_mut().find(|s| s.deployment.id == deployment_id) {
                    s.deployment.state = state.clone();
                }

                if let Some(dep) =
                    self.service_deployments.iter_mut().find(|d| d.id == deployment_id)
                {
                    dep.state = state.clone();
                } else if self.active_service_id.as_deref() == Some(&service_id) {
                    // Evento chegou antes de Response::Deployment — insere placeholder
                    // que será sobrescrito quando a resposta RPC chegar.
                    self.service_deployments.insert(0, Deployment {
                        id: deployment_id.clone(),
                        service_id: service_id.clone(),
                        image: String::new(),
                        state: state.clone(),
                        states_log: vec![],
                        started_at: chrono::Utc::now(),
                        finished_at: None,
                    });
                }

                let entry = self.deploy_progress.entry(deployment_id.clone()).or_insert_with(
                    || DeployProgressState {
                        deployment_id: deployment_id.clone(),
                        service_id: service_id.clone(),
                        current_state: state.clone(),
                        percent: 0,
                        description: String::new(),
                        states_seen: vec![],
                    },
                );
                entry.states_seen.push(entry.current_state.clone());
                entry.current_state = state;
                entry.percent = state_to_percent(&entry.current_state);
            }

            Event::DeployProgress { deployment_id, percent, description, .. } => {
                if let Some(p) = self.deploy_progress.get_mut(&deployment_id) {
                    p.percent = percent;
                    p.description = description;
                }
            }

            Event::BuildLog { deployment_id, line, timestamp, .. } => {
                let buf = self.build_logs.entry(deployment_id).or_default();
                if buf.len() >= MAX_LOG_LINES {
                    buf.pop_front();
                }
                buf.push_back(LogLine { timestamp, text: line, is_stderr: false });
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

    pub fn handle_response(&mut self, resp: Response, ctx: CmdContext) {
        match (resp, ctx) {
            (Response::Projects(projects), CmdContext::LoadProjects) => {
                self.projects = projects;
            }
            (Response::Services(services), CmdContext::LoadServices) => {
                self.services = services;
            }
            (Response::Project(p), CmdContext::CreateProject) => {
                self.projects.push(p);
                self.set_notification("Projeto criado", false);
            }
            (Response::Project(p), CmdContext::UpdateProjectEnv) => {
                if let Some(existing) = self.projects.iter_mut().find(|x| x.id == p.id) {
                    *existing = p;
                }
                self.set_notification("Env vars do projeto atualizadas", false);
            }
            (Response::Service(s), CmdContext::CreateService) => {
                self.services.push(s);
                self.view = View::ProjectDetail;
                self.set_notification("Serviço criado", false);
            }
            (Response::Service(s), CmdContext::UpdateService) => {
                if let Some(existing) = self.services.iter_mut().find(|x| x.id == s.id) {
                    *existing = s.clone();
                }
                if self.active_service_id.as_deref() == Some(&s.id) {
                    self.general_tab = GeneralTabState::from_service(&s);
                    self.healthcheck_tab = HealthcheckTabState::from_service(&s);
                    self.domains_tab = DomainsTabState::from_service(&s);
                    self.advanced_tab = AdvancedTabState::from_service(&s);
                }
                self.set_notification("Serviço atualizado", false);
            }
            (Response::Ok, CmdContext::DeleteProject) => {
                if let Some(pid) = &self.active_project_id.clone() {
                    self.projects.retain(|p| &p.id != pid);
                    self.services.clear();
                    self.active_project_id = None;
                    self.view = View::HomeDeployments;
                    self.focus = Focus::Sidebar;
                }
                self.set_notification("Projeto removido", false);
            }
            (Response::Ok, CmdContext::DeleteService) => {
                if let Some(sid) = &self.active_service_id.clone() {
                    self.services.retain(|s| &s.id != sid);
                    self.active_service_id = None;
                    self.view = View::ProjectDetail;
                }
                self.set_notification("Serviço removido", false);
            }
            (Response::Deployments(deps), CmdContext::LoadDeployments) => {
                if let Some(first) = deps.first() {
                    self.pending_commands.push(PendingCommand {
                        command: Command::GetBuildLogs { deployment_id: first.id.clone() },
                        context: CmdContext::LoadBuildLogs,
                    });
                }
                self.service_deployments = deps;
            }
            (Response::DeploymentSummaries(summaries), CmdContext::LoadHomeDeployments) => {
                self.home_deployments = summaries;
            }
            (Response::BuildLogs(entries), CmdContext::LoadBuildLogs) => {
                // deposit into whichever deployment is currently selected
                if let Some(dep) = self.service_deployments.get(
                    self.deployment_cursor.min(self.service_deployments.len().saturating_sub(1))
                ) {
                    let buf = self.build_logs.entry(dep.id.clone()).or_default();
                    buf.clear();
                    for e in entries {
                        buf.push_back(LogLine { timestamp: e.timestamp, text: e.line, is_stderr: false });
                    }
                }
            }
            (Response::Logs(entries), CmdContext::LoadLogs) => {
                if let Some(sid) = &self.active_service_id.clone() {
                    let buf = self.logs.entry(sid.clone()).or_default();
                    buf.clear();
                    for e in entries {
                        buf.push_back(LogLine {
                            timestamp: e.timestamp,
                            text: e.line,
                            is_stderr: e.stream == shared::protocol::LogStream::Stderr,
                        });
                    }
                }
            }
            (Response::Ok, CmdContext::ServiceStop) => {
                if let Some(sid) = self.active_service_id.clone() {
                    if let Some(svc) = self.services.iter_mut().find(|s| s.id == sid) {
                        svc.status = ServiceStatus::Stopped;
                    }
                }
                self.set_notification("Serviço parado", false);
            }
            (Response::Err { message, .. }, CmdContext::ServiceStop) => {
                // Reverte para Running se o stop falhou.
                if let Some(sid) = self.active_service_id.clone() {
                    if let Some(svc) = self.services.iter_mut().find(|s| s.id == sid) {
                        svc.status = ServiceStatus::Running;
                    }
                }
                self.set_notification(message, true);
            }
            (Response::Ok, CmdContext::ServiceReload) => {
                if let Some(sid) = self.active_service_id.clone() {
                    if let Some(svc) = self.services.iter_mut().find(|s| s.id == sid) {
                        svc.status = ServiceStatus::Running;
                    }
                }
                self.set_notification("Container reiniciado", false);
            }
            (Response::Deployment(dep), CmdContext::Deploy) => {
                self.logs.remove(&dep.service_id);
                // Aplica estado já conhecido do stream (eventos podem ter chegado antes).
                let mut dep = dep;
                if let Some(progress) = self.deploy_progress.get(&dep.id) {
                    dep.state = progress.current_state.clone();
                }
                // Substitui placeholder inserido pelo evento, ou insere no topo.
                if let Some(pos) = self.service_deployments.iter().position(|d| d.id == dep.id) {
                    self.service_deployments[pos] = dep.clone();
                } else {
                    self.service_deployments.insert(0, dep.clone());
                }
                // Adiciona/atualiza na home com nomes resolvidos do estado local.
                let svc = self.services.iter().find(|s| s.id == dep.service_id);
                let service_name = svc.map(|s| s.spec.name.clone()).unwrap_or_default();
                let project_name = svc
                    .and_then(|s| self.projects.iter().find(|p| p.id == s.spec.project_id))
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                let summary = DeploymentSummary { deployment: dep.clone(), service_name, project_name };
                if let Some(pos) = self.home_deployments.iter().position(|s| s.deployment.id == dep.id) {
                    self.home_deployments[pos] = summary;
                } else {
                    self.home_deployments.insert(0, summary);
                }
                self.last_deployment = Some(dep);
                self.set_notification("Deploy iniciado ✓", false);
            }
            (Response::Ok, CmdContext::Deploy) => {
                self.set_notification("Deploy iniciado", false);
            }
            (Response::Err { message, .. }, _) => {
                self.set_notification(message, true);
            }
            _ => {}
        }
    }

    pub fn can_quit(&self) -> bool {
        self.focus == Focus::Sidebar && !self.creating_project && self.new_service.is_none()
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
        DeployState::Stopped => 100,
        DeployState::RollingBack | DeployState::Failed => 0,
        DeployState::Pruning => 100,
    }
}
