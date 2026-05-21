use chrono::{DateTime, Utc};
use shared::{
    Command, ContainerMetricsPoint, Deployment, DeployState, Event, GitSource, Healthcheck,
    Project, ResourceLimits, Response, Service, ServiceSource, ServiceSpec,
};
use std::collections::{HashMap, VecDeque};

pub const MAX_LOG_LINES: usize = 2000;
pub const MAX_METRIC_POINTS: usize = 60;

#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    Sidebar,
    Content,
}

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    HomeDeployments,
    HomeMonitoring,
    HomeSchedules,
    HomePingoraFs,
    HomeDocker,
    HomeDeployEngine,
    HomeRequests,
    ProjectDetail,
    ServiceDetail,
    ServiceForm,
    SettingsWebServer,
    SettingsProfile,
    SettingsUsers,
    SettingsAuditLogs,
    SettingsSshKeys,
    SettingsTags,
    SettingsGit,
    SettingsRegistry,
    SettingsS3,
    SettingsCerts,
    SettingsSso,
    Account,
    Confirm { message: String, action: ConfirmAction },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    DeleteProject(String),
    DeleteService(String),
    AbortDeploy(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum SidebarItem {
    HomeDeployments,
    HomeMonitoring,
    HomeSchedules,
    HomePingoraFs,
    HomeDocker,
    HomeDeployEngine,
    HomeRequests,
    NewProject,
    Project(usize),
    SettingsWebServer,
    SettingsProfile,
    SettingsUsers,
    SettingsAuditLogs,
    SettingsSshKeys,
    SettingsTags,
    SettingsGit,
    SettingsRegistry,
    SettingsS3,
    SettingsCerts,
    SettingsSso,
    Account,
}

impl SidebarItem {
    pub fn label(&self, projects: &[Project]) -> String {
        match self {
            Self::HomeDeployments => "  Deployments".into(),
            Self::HomeMonitoring => "  Monitoring".into(),
            Self::HomeSchedules => "  Schedules".into(),
            Self::HomePingoraFs => "  Pingora FS".into(),
            Self::HomeDocker => "  Docker".into(),
            Self::HomeDeployEngine => "  Deploy Engine".into(),
            Self::HomeRequests => "  Requests".into(),
            Self::NewProject => "  + New Project".into(),
            Self::Project(i) => projects
                .get(*i)
                .map(|p| format!("  {}", p.name))
                .unwrap_or_else(|| "  ?".into()),
            Self::SettingsWebServer => "  Web Server".into(),
            Self::SettingsProfile => "  Profile".into(),
            Self::SettingsUsers => "  Users".into(),
            Self::SettingsAuditLogs => "  Audit Logs".into(),
            Self::SettingsSshKeys => "  SSH Keys".into(),
            Self::SettingsTags => "  Tags".into(),
            Self::SettingsGit => "  Git".into(),
            Self::SettingsRegistry => "  Registry".into(),
            Self::SettingsS3 => "  S3 Destinations".into(),
            Self::SettingsCerts => "  Certificates".into(),
            Self::SettingsSso => "  SSO".into(),
            Self::Account => "ACCOUNT".into(),
        }
    }

    pub fn to_view(&self) -> Option<View> {
        Some(match self {
            Self::HomeDeployments => View::HomeDeployments,
            Self::HomeMonitoring => View::HomeMonitoring,
            Self::HomeSchedules => View::HomeSchedules,
            Self::HomePingoraFs => View::HomePingoraFs,
            Self::HomeDocker => View::HomeDocker,
            Self::HomeDeployEngine => View::HomeDeployEngine,
            Self::HomeRequests => View::HomeRequests,
            Self::SettingsWebServer => View::SettingsWebServer,
            Self::SettingsProfile => View::SettingsProfile,
            Self::SettingsUsers => View::SettingsUsers,
            Self::SettingsAuditLogs => View::SettingsAuditLogs,
            Self::SettingsSshKeys => View::SettingsSshKeys,
            Self::SettingsTags => View::SettingsTags,
            Self::SettingsGit => View::SettingsGit,
            Self::SettingsRegistry => View::SettingsRegistry,
            Self::SettingsS3 => View::SettingsS3,
            Self::SettingsCerts => View::SettingsCerts,
            Self::SettingsSso => View::SettingsSso,
            Self::Account => View::Account,
            Self::NewProject | Self::Project(_) => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceTab {
    General,
    Environment,
    Domains,
    Deployments,
    Logs,
    Patches,
}

impl ServiceTab {
    pub fn all() -> &'static [ServiceTab] {
        &[
            ServiceTab::General,
            ServiceTab::Environment,
            ServiceTab::Domains,
            ServiceTab::Deployments,
            ServiceTab::Logs,
            ServiceTab::Patches,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ServiceTab::General => "General",
            ServiceTab::Environment => "Environment",
            ServiceTab::Domains => "Domains",
            ServiceTab::Deployments => "Deployments",
            ServiceTab::Logs => "Logs",
            ServiceTab::Patches => "Patches",
        }
    }

    pub fn index(&self) -> usize {
        Self::all().iter().position(|t| t == self).unwrap_or(0)
    }

    pub fn next(&self) -> ServiceTab {
        let all = Self::all();
        let idx = (self.index() + 1) % all.len();
        all[idx].clone()
    }

    pub fn prev(&self) -> ServiceTab {
        let all = Self::all();
        let idx = if self.index() == 0 { all.len() - 1 } else { self.index() - 1 };
        all[idx].clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum GeneralTabField {
    #[default]
    BtnDeploy,
    BtnReload,
    BtnRebuild,
    BtnStop,
    RepoUrl,
    Branch,
    BuildPath,
    WatchPaths,
    Submodules,
    AddSshKeys,
    ProviderSave,
    DockerFile,
    DockerContextPath,
    DockerBuildStage,
    BuildSave,
}

impl GeneralTabField {
    const COUNT: usize = 15;

    pub fn next(self) -> Self {
        Self::from_idx((self as usize + 1) % Self::COUNT)
    }

    pub fn prev(self) -> Self {
        let i = self as usize;
        Self::from_idx(if i == 0 { Self::COUNT - 1 } else { i - 1 })
    }

    fn from_idx(i: usize) -> Self {
        match i {
            0 => Self::BtnDeploy,
            1 => Self::BtnReload,
            2 => Self::BtnRebuild,
            3 => Self::BtnStop,
            4 => Self::RepoUrl,
            5 => Self::Branch,
            6 => Self::BuildPath,
            7 => Self::WatchPaths,
            8 => Self::Submodules,
            9 => Self::AddSshKeys,
            10 => Self::ProviderSave,
            11 => Self::DockerFile,
            12 => Self::DockerContextPath,
            13 => Self::DockerBuildStage,
            _ => Self::BuildSave,
        }
    }

    pub fn is_text_field(self) -> bool {
        matches!(
            self,
            Self::RepoUrl
                | Self::Branch
                | Self::BuildPath
                | Self::WatchPaths
                | Self::DockerFile
                | Self::DockerContextPath
                | Self::DockerBuildStage
        )
    }

    pub fn is_button(self) -> bool {
        matches!(
            self,
            Self::BtnDeploy
                | Self::BtnReload
                | Self::BtnRebuild
                | Self::BtnStop
                | Self::AddSshKeys
                | Self::ProviderSave
                | Self::BuildSave
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct GeneralTabState {
    pub focused_field: GeneralTabField,
    pub repo_url: String,
    pub branch: String,
    pub build_path: String,
    pub watch_paths: String,
    pub submodules: bool,
    pub dockerfile: String,
    pub context_path: String,
    pub build_stage: String,
}

impl GeneralTabState {
    pub fn from_service(svc: &Service) -> Self {
        match &svc.spec.source {
            ServiceSource::Git(g) => Self {
                focused_field: GeneralTabField::BtnDeploy,
                repo_url: g.url.clone(),
                branch: g.branch.clone(),
                build_path: g.root_path.clone(),
                watch_paths: g.watch_paths.join(", "),
                submodules: g.submodules,
                dockerfile: g.dockerfile_path.clone(),
                context_path: g.build_context.clone(),
                build_stage: g.build_stage.clone().unwrap_or_default(),
            },
            ServiceSource::Registry { image } => Self {
                focused_field: GeneralTabField::BtnDeploy,
                repo_url: image.clone(),
                branch: String::new(),
                build_path: ".".into(),
                watch_paths: String::new(),
                submodules: false,
                dockerfile: "Dockerfile".into(),
                context_path: ".".into(),
                build_stage: String::new(),
            },
        }
    }

    pub fn focused_text_mut(&mut self) -> Option<&mut String> {
        match self.focused_field {
            GeneralTabField::RepoUrl => Some(&mut self.repo_url),
            GeneralTabField::Branch => Some(&mut self.branch),
            GeneralTabField::BuildPath => Some(&mut self.build_path),
            GeneralTabField::WatchPaths => Some(&mut self.watch_paths),
            GeneralTabField::DockerFile => Some(&mut self.dockerfile),
            GeneralTabField::DockerContextPath => Some(&mut self.context_path),
            GeneralTabField::DockerBuildStage => Some(&mut self.build_stage),
            _ => None,
        }
    }

    pub fn to_git_source(&self, existing: &GitSource) -> GitSource {
        GitSource {
            url: self.repo_url.clone(),
            branch: self.branch.clone(),
            root_path: self.build_path.clone(),
            watch_paths: self
                .watch_paths
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            submodules: self.submodules,
            dockerfile_path: self.dockerfile.clone(),
            build_context: self.context_path.clone(),
            build_stage: if self.build_stage.is_empty() { None } else { Some(self.build_stage.clone()) },
            credentials: existing.credentials.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ServiceFormField {
    #[default]
    Name,
    Port,
    Domain,
    RepoUrl,
    Branch,
    BuildPath,
    WatchPaths,
    Submodules,
    DockerFile,
    DockerContextPath,
    DockerBuildStage,
    BtnCreate,
    BtnCancel,
}

impl ServiceFormField {
    const COUNT: usize = 13;

    pub fn next(self) -> Self {
        Self::from_idx((self as usize + 1) % Self::COUNT)
    }

    pub fn prev(self) -> Self {
        let i = self as usize;
        Self::from_idx(if i == 0 { Self::COUNT - 1 } else { i - 1 })
    }

    fn from_idx(i: usize) -> Self {
        match i {
            0 => Self::Name,
            1 => Self::Port,
            2 => Self::Domain,
            3 => Self::RepoUrl,
            4 => Self::Branch,
            5 => Self::BuildPath,
            6 => Self::WatchPaths,
            7 => Self::Submodules,
            8 => Self::DockerFile,
            9 => Self::DockerContextPath,
            10 => Self::DockerBuildStage,
            11 => Self::BtnCreate,
            _ => Self::BtnCancel,
        }
    }

    pub fn is_text_field(self) -> bool {
        matches!(
            self,
            Self::Name
                | Self::Port
                | Self::Domain
                | Self::RepoUrl
                | Self::Branch
                | Self::BuildPath
                | Self::WatchPaths
                | Self::DockerFile
                | Self::DockerContextPath
                | Self::DockerBuildStage
        )
    }
}

#[derive(Debug, Clone)]
pub struct ServiceFormState {
    pub project_id: String,
    pub name: String,
    pub port: String,
    pub domain: String,
    pub repo_url: String,
    pub branch: String,
    pub build_path: String,
    pub watch_paths: String,
    pub submodules: bool,
    pub dockerfile: String,
    pub context_path: String,
    pub build_stage: String,
    pub focused_field: ServiceFormField,
}

impl ServiceFormState {
    pub fn new(project_id: String) -> Self {
        Self {
            project_id,
            name: String::new(),
            port: "8080".into(),
            domain: String::new(),
            repo_url: String::new(),
            branch: "main".into(),
            build_path: ".".into(),
            watch_paths: String::new(),
            submodules: false,
            dockerfile: "Dockerfile".into(),
            context_path: ".".into(),
            build_stage: String::new(),
            focused_field: ServiceFormField::Name,
        }
    }

    pub fn focused_text_mut(&mut self) -> Option<&mut String> {
        match self.focused_field {
            ServiceFormField::Name => Some(&mut self.name),
            ServiceFormField::Port => Some(&mut self.port),
            ServiceFormField::Domain => Some(&mut self.domain),
            ServiceFormField::RepoUrl => Some(&mut self.repo_url),
            ServiceFormField::Branch => Some(&mut self.branch),
            ServiceFormField::BuildPath => Some(&mut self.build_path),
            ServiceFormField::WatchPaths => Some(&mut self.watch_paths),
            ServiceFormField::DockerFile => Some(&mut self.dockerfile),
            ServiceFormField::DockerContextPath => Some(&mut self.context_path),
            ServiceFormField::DockerBuildStage => Some(&mut self.build_stage),
            _ => None,
        }
    }

    pub fn to_spec(&self) -> ServiceSpec {
        let port = self.port.parse::<u16>().unwrap_or(8080);
        ServiceSpec {
            name: self.name.clone(),
            project_id: self.project_id.clone(),
            source: ServiceSource::Git(GitSource {
                url: self.repo_url.clone(),
                branch: self.branch.clone(),
                root_path: self.build_path.clone(),
                watch_paths: self
                    .watch_paths
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                submodules: self.submodules,
                dockerfile_path: self.dockerfile.clone(),
                build_context: self.context_path.clone(),
                build_stage: if self.build_stage.is_empty() {
                    None
                } else {
                    Some(self.build_stage.clone())
                },
                credentials: None,
            }),
            port,
            domain: self.domain.clone(),
            env_vars: vec![],
            volumes: vec![],
            healthcheck: Healthcheck::default(),
            replicas: 1,
            resources: ResourceLimits::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EnvTabState {
    pub cursor: usize,
    pub editing: bool,
    pub edit_key: String,
    pub edit_value: String,
    pub edit_field: EnvEditField,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum EnvEditField {
    #[default]
    Key,
    Value,
}

#[derive(Debug)]
pub struct PendingCommand {
    pub command: Command,
    pub context: CmdContext,
}

#[derive(Debug)]
pub enum CmdContext {
    None,
    LoadProjects,
    LoadServices,
    CreateProject,
    DeleteProject,
    CreateService,
    UpdateService,
    DeleteService,
    Deploy,
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

#[derive(Debug, Clone)]
pub struct DeployProgressState {
    pub deployment_id: String,
    pub service_id: String,
    pub current_state: DeployState,
    pub percent: u8,
    pub description: String,
    pub states_seen: Vec<DeployState>,
}

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

    pub service_tab: ServiceTab,
    pub general_tab: GeneralTabState,
    pub env_tab: EnvTabState,
    pub deployment_cursor: usize,
    pub log_cursor: usize,

    pub service_form: Option<ServiceFormState>,

    pub deploy_progress: HashMap<String, DeployProgressState>,
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

            service_tab: ServiceTab::General,
            general_tab: GeneralTabState::default(),
            env_tab: EnvTabState::default(),
            deployment_cursor: 0,
            log_cursor: 0,

            service_form: None,

            deploy_progress: HashMap::new(),
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
            SidebarItem::HomePingoraFs,
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
        let filtered = self.filtered_services();
        filtered.into_iter().nth(self.service_cursor)
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
        self.env_tab = EnvTabState::default();
        self.deployment_cursor = 0;
        self.log_cursor = 0;
        self.view = View::ServiceDetail;
        self.focus = Focus::Content;
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

            Event::DeployProgress { deployment_id, percent, description, .. } => {
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
        self.focus == Focus::Sidebar && !self.creating_project
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
        DeployState::RollingBack | DeployState::Failed => 0,
        DeployState::Pruning => 100,
    }
}
