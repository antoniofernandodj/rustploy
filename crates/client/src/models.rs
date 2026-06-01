use chrono::{DateTime, Utc};
use shared::{
    Command, ComposeSource, DeployState, EnvVar, EnvVarValue, GitSource, Healthcheck,
    HealthcheckKind, Project, ResourceLimits, Service, ServiceSource, ServiceSpec,
};

pub const MAX_LOG_LINES: usize = 2000;
pub const MAX_METRIC_POINTS: usize = 60;

// ── Focus / View ─────────────────────────────────────────────────────────────

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
    HomeIngress,
    HomeDocker,
    HomeDeployEngine,
    HomeRequests,
    ProjectDetail,
    ServiceDetail,
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

// ── Sidebar ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum SidebarItem {
    HomeDeployments,
    HomeMonitoring,
    HomeSchedules,
    HomeIngress,
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
            Self::HomeIngress => "  Ingress".into(),
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
            Self::HomeIngress => View::HomeIngress,
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

// ── Service tabs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceTab {
    General,
    Environment,
    Domains,
    Deployments,
    Healthcheck,
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
            ServiceTab::Healthcheck,
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
            ServiceTab::Healthcheck => "Healthcheck",
            ServiceTab::Logs => "Logs",
            ServiceTab::Patches => "Patches",
        }
    }

    pub fn index(&self) -> usize {
        Self::all().iter().position(|t| t == self).unwrap_or(0)
    }

    pub fn next(&self) -> ServiceTab {
        let all = Self::all();
        all[(self.index() + 1) % all.len()].clone()
    }

    pub fn prev(&self) -> ServiceTab {
        let all = Self::all();
        let idx = if self.index() == 0 { all.len() - 1 } else { self.index() - 1 };
        all[idx].clone()
    }
}

// ── General tab ───────────────────────────────────────────────────────────────

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
    Port,
    AddSshKeys,
    ProviderSave,
    DockerFile,
    DockerContextPath,
    DockerBuildStage,
    BuildSave,
}

impl GeneralTabField {
    const COUNT: usize = 16;

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
            9 => Self::Port,
            10 => Self::AddSshKeys,
            11 => Self::ProviderSave,
            12 => Self::DockerFile,
            13 => Self::DockerContextPath,
            14 => Self::DockerBuildStage,
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
                | Self::Port
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
    pub port: String,
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
                port: svc.spec.port.to_string(),
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
                port: svc.spec.port.to_string(),
                dockerfile: "Dockerfile".into(),
                context_path: ".".into(),
                build_stage: String::new(),
            },
            ServiceSource::Compose(_) => Self {
                focused_field: GeneralTabField::BtnDeploy,
                repo_url: String::new(),
                branch: String::new(),
                build_path: String::new(),
                watch_paths: String::new(),
                submodules: false,
                port: svc.spec.port.to_string(),
                dockerfile: String::new(),
                context_path: String::new(),
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
            GeneralTabField::Port => Some(&mut self.port),
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
            build_stage: if self.build_stage.is_empty() {
                None
            } else {
                Some(self.build_stage.clone())
            },
            credentials: existing.credentials.clone(),
        }
    }
}

// ── Healthcheck tab ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum HcField {
    #[default]
    Kind,
    HttpPath,
    ExpectedStatus,
    Interval,
    Timeout,
    Retries,
    StartPeriod,
    Save,
}

impl HcField {
    const COUNT: usize = 8;

    pub fn next(self) -> Self {
        Self::from_idx((self as usize + 1) % Self::COUNT)
    }

    pub fn prev(self) -> Self {
        let i = self as usize;
        Self::from_idx(if i == 0 { Self::COUNT - 1 } else { i - 1 })
    }

    fn from_idx(i: usize) -> Self {
        match i {
            0 => Self::Kind,
            1 => Self::HttpPath,
            2 => Self::ExpectedStatus,
            3 => Self::Interval,
            4 => Self::Timeout,
            5 => Self::Retries,
            6 => Self::StartPeriod,
            _ => Self::Save,
        }
    }

    pub fn is_text(self) -> bool {
        matches!(
            self,
            Self::HttpPath
                | Self::ExpectedStatus
                | Self::Interval
                | Self::Timeout
                | Self::Retries
                | Self::StartPeriod
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum DomainsField {
    #[default]
    Domain,
    HostPort,
    Save,
}

impl DomainsField {
    pub fn next(self) -> Self {
        match self {
            Self::Domain => Self::HostPort,
            Self::HostPort => Self::Save,
            Self::Save => Self::Domain,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            Self::Domain => Self::Save,
            Self::HostPort => Self::Domain,
            Self::Save => Self::HostPort,
        }
    }
    pub fn is_text(self) -> bool {
        matches!(self, Self::Domain | Self::HostPort)
    }
}

#[derive(Debug, Clone, Default)]
pub struct DomainsTabState {
    pub focused: DomainsField,
    pub domain: String,
    pub host_port: String,
}

impl DomainsTabState {
    pub fn from_service(svc: &Service) -> Self {
        Self {
            focused: DomainsField::Domain,
            domain: svc.spec.domain.clone().unwrap_or_default(),
            host_port: svc.spec.host_port
                .map(|p| p.to_string())
                .unwrap_or_default(),
        }
    }

    pub fn focused_text_mut(&mut self) -> Option<&mut String> {
        match self.focused {
            DomainsField::Domain => Some(&mut self.domain),
            DomainsField::HostPort => Some(&mut self.host_port),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HealthcheckTabState {
    pub focused: HcField,
    pub kind: String,
    pub http_path: String,
    pub expected_status: String,
    pub interval: String,
    pub timeout: String,
    pub retries: String,
    pub start_period: String,
}

impl Default for HealthcheckTabState {
    fn default() -> Self {
        Self {
            focused: HcField::Kind,
            kind: "Tcp".into(),
            http_path: String::new(),
            expected_status: "200".into(),
            interval: "5".into(),
            timeout: "3".into(),
            retries: "10".into(),
            start_period: "5".into(),
        }
    }
}

impl HealthcheckTabState {
    pub fn from_service(svc: &Service) -> Self {
        let hc = &svc.spec.healthcheck;
        let (kind, http_path, expected_status) = match &hc.kind {
            HealthcheckKind::Tcp => ("Tcp".into(), String::new(), "200".into()),
            HealthcheckKind::Http { path, expected_status } => {
                ("Http".into(), path.clone(), expected_status.to_string())
            }
            HealthcheckKind::DockerNative => ("DockerNative".into(), String::new(), "200".into()),
        };
        Self {
            focused: HcField::Kind,
            kind,
            http_path,
            expected_status,
            interval: hc.interval_secs.to_string(),
            timeout: hc.timeout_secs.to_string(),
            retries: hc.retries.to_string(),
            start_period: hc.start_period_secs.to_string(),
        }
    }

    pub fn cycle_kind(&mut self) {
        self.kind = match self.kind.as_str() {
            "Tcp" => "Http".into(),
            "Http" => "DockerNative".into(),
            _ => "Tcp".into(),
        };
    }

    pub fn focused_text_mut(&mut self) -> Option<&mut String> {
        match self.focused {
            HcField::HttpPath => Some(&mut self.http_path),
            HcField::ExpectedStatus => Some(&mut self.expected_status),
            HcField::Interval => Some(&mut self.interval),
            HcField::Timeout => Some(&mut self.timeout),
            HcField::Retries => Some(&mut self.retries),
            HcField::StartPeriod => Some(&mut self.start_period),
            _ => None,
        }
    }

    pub fn to_healthcheck(&self) -> Healthcheck {
        let kind = match self.kind.as_str() {
            "Http" => HealthcheckKind::Http {
                path: if self.http_path.is_empty() {
                    "/".into()
                } else {
                    self.http_path.clone()
                },
                expected_status: self.expected_status.parse().unwrap_or(200),
            },
            "DockerNative" => HealthcheckKind::DockerNative,
            _ => HealthcheckKind::Tcp,
        };
        Healthcheck {
            kind,
            interval_secs: self.interval.parse().unwrap_or(5),
            timeout_secs: self.timeout.parse().unwrap_or(3),
            retries: self.retries.parse().unwrap_or(10),
            start_period_secs: self.start_period.parse().unwrap_or(5),
        }
    }
}

// ── New-service creation flow ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum NewServiceStep {
    PickType,
    PickDbType,
    ApplicationForm,
    DatabaseForm,
    ComposeForm,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceKind {
    Application,
    Database,
    Compose,
    Template,
}

impl ServiceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Application => "Application",
            Self::Database => "Database",
            Self::Compose => "Compose",
            Self::Template => "Template",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Application => "Web app via Git ou imagem",
            Self::Database => "Banco de dados gerenciado",
            Self::Compose => "Stack Docker Compose",
            Self::Template => "A partir de um preset",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DbKind {
    MongoDB,
    Postgres,
    MariaDB,
    MySQL,
    Redis,
}

impl DbKind {
    pub const ALL: &'static [DbKind] =
        &[DbKind::MongoDB, DbKind::Postgres, DbKind::MariaDB, DbKind::MySQL, DbKind::Redis];

    pub fn label(self) -> &'static str {
        match self {
            Self::MongoDB => "MongoDB",
            Self::Postgres => "PostgreSQL",
            Self::MariaDB => "MariaDB",
            Self::MySQL => "MySQL",
            Self::Redis => "Redis",
        }
    }

    pub fn default_image(self) -> &'static str {
        match self {
            Self::MongoDB => "mongo:8",
            Self::Postgres => "postgres:18",
            Self::MariaDB => "mariadb:11",
            Self::MySQL => "mysql:8",
            Self::Redis => "redis:7",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Self::MongoDB => 27017,
            Self::Postgres => 5432,
            Self::MariaDB | Self::MySQL => 3306,
            Self::Redis => 6379,
        }
    }

    pub fn field_count(self) -> usize {
        match self {
            Self::Postgres => 8,
            Self::MongoDB => 8,
            Self::MariaDB | Self::MySQL => 9,
            Self::Redis => 6,
        }
    }
}

#[derive(Debug)]
pub struct NewServiceState {
    pub project_id: String,
    pub step: NewServiceStep,
    pub type_cursor: usize,
    pub db_cursor: usize,
    pub db_kind: Option<DbKind>,
    pub name: String,
    pub app_name: String,
    pub description: String,
    pub db_name: String,
    pub db_user: String,
    pub db_password: String,
    pub db_root_password: String,
    pub docker_image: String,
    pub compose_file_path: String,
    pub use_replica_sets: bool,
    pub focused_field: usize,
    pub form_scroll: usize,
}

impl NewServiceState {
    pub fn new(project_id: String) -> Self {
        Self {
            project_id,
            step: NewServiceStep::PickType,
            type_cursor: 0,
            db_cursor: 0,
            db_kind: None,
            name: String::new(),
            app_name: String::new(),
            description: String::new(),
            db_name: String::new(),
            db_user: String::new(),
            db_password: String::new(),
            db_root_password: String::new(),
            docker_image: String::new(),
            compose_file_path: "docker-compose.yml".into(),
            use_replica_sets: false,
            focused_field: 0,
            form_scroll: 0,
        }
    }

    pub fn field_count(&self) -> usize {
        match self.step {
            NewServiceStep::ApplicationForm => 4,
            NewServiceStep::ComposeForm => 3,
            NewServiceStep::DatabaseForm => {
                self.db_kind.map(|d| d.field_count()).unwrap_or(0)
            }
            _ => 0,
        }
    }

    pub fn next_field(&mut self) {
        let max = self.field_count();
        if max > 0 {
            self.focused_field = (self.focused_field + 1) % max;
            self.sync_scroll();
        }
    }

    pub fn prev_field(&mut self) {
        let max = self.field_count();
        if max > 0 {
            if self.focused_field == 0 {
                self.focused_field = max - 1;
            } else {
                self.focused_field -= 1;
            }
            self.sync_scroll();
        }
    }

    fn scrollable_fields(&self) -> usize {
        self.field_count().saturating_sub(1)
    }

    fn sync_scroll(&mut self) {
        if !matches!(self.step, NewServiceStep::DatabaseForm | NewServiceStep::ComposeForm) {
            return;
        }
        if matches!(self.step, NewServiceStep::ComposeForm) {
            return;
        }
        const VISIBLE: usize = 4;
        let scrollable = self.scrollable_fields();
        let focused = self.focused_field.min(scrollable.saturating_sub(1));
        if focused < self.form_scroll {
            self.form_scroll = focused;
        } else if self.form_scroll + VISIBLE <= focused {
            self.form_scroll = focused + 1 - VISIBLE;
        }
        self.form_scroll = self.form_scroll.min(scrollable.saturating_sub(VISIBLE));
    }

    pub fn is_button(&self) -> bool {
        let max = self.field_count();
        max > 0 && self.focused_field == max - 1
    }

    pub fn is_checkbox(&self) -> bool {
        self.step == NewServiceStep::DatabaseForm
            && matches!(self.db_kind, Some(DbKind::MongoDB))
            && self.focused_field == 6
    }

    pub fn focused_text_mut(&mut self) -> Option<&mut String> {
        let field = self.focused_field;
        let step = self.step.clone();
        let db = self.db_kind;
        match step {
            NewServiceStep::ApplicationForm => match field {
                0 => Some(&mut self.name),
                1 => Some(&mut self.app_name),
                2 => Some(&mut self.description),
                _ => None,
            },
            NewServiceStep::ComposeForm => match field {
                0 => Some(&mut self.name),
                1 => Some(&mut self.app_name),
                _ => None,
            },
            NewServiceStep::DatabaseForm => match (db?, field) {
                (_, 0) => Some(&mut self.name),
                (_, 1) => Some(&mut self.app_name),
                (_, 2) => Some(&mut self.description),
                (DbKind::Postgres, 3) | (DbKind::MariaDB | DbKind::MySQL, 3) => {
                    Some(&mut self.db_name)
                }
                (DbKind::Postgres, 4) | (DbKind::MongoDB, 3) => Some(&mut self.db_user),
                (DbKind::MariaDB | DbKind::MySQL, 4) => Some(&mut self.db_user),
                (DbKind::Postgres, 5) | (DbKind::MongoDB, 4) => Some(&mut self.db_password),
                (DbKind::MariaDB | DbKind::MySQL, 5) => Some(&mut self.db_password),
                (DbKind::MariaDB | DbKind::MySQL, 6) => Some(&mut self.db_root_password),
                (DbKind::Postgres, 6) | (DbKind::MongoDB, 5) => Some(&mut self.docker_image),
                (DbKind::MariaDB | DbKind::MySQL, 7) => Some(&mut self.docker_image),
                (DbKind::Redis, 3) => Some(&mut self.db_password),
                (DbKind::Redis, 4) => Some(&mut self.docker_image),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn select_db_kind(&mut self) {
        let db = DbKind::ALL[self.db_cursor];
        self.db_kind = Some(db);
        self.docker_image = db.default_image().to_string();
        self.step = NewServiceStep::DatabaseForm;
        self.focused_field = 0;
    }

    fn db_env_vars(&self) -> Vec<EnvVar> {
        let plain = |k: &str, v: &str| EnvVar {
            key: k.to_string(),
            value: EnvVarValue::Plain(v.to_string()),
        };
        match self.db_kind {
            Some(DbKind::Postgres) => vec![
                plain("POSTGRES_DB", &self.db_name),
                plain("POSTGRES_USER", &self.db_user),
                plain("POSTGRES_PASSWORD", &self.db_password),
            ],
            Some(DbKind::MongoDB) => {
                let mut vars = vec![
                    plain("MONGO_INITDB_ROOT_USERNAME", &self.db_user),
                    plain("MONGO_INITDB_ROOT_PASSWORD", &self.db_password),
                ];
                if self.use_replica_sets {
                    vars.push(plain("MONGO_REPLICA_SET_NAME", "rs0"));
                }
                vars
            }
            Some(DbKind::MariaDB | DbKind::MySQL) => vec![
                plain("MYSQL_DATABASE", &self.db_name),
                plain("MYSQL_USER", &self.db_user),
                plain("MYSQL_PASSWORD", &self.db_password),
                plain("MYSQL_ROOT_PASSWORD", &self.db_root_password),
            ],
            Some(DbKind::Redis) if !self.db_password.is_empty() => {
                vec![plain("REDIS_PASSWORD", &self.db_password)]
            }
            _ => vec![],
        }
    }

    pub fn to_service_spec(&self) -> ServiceSpec {
        let svc_name =
            if !self.app_name.is_empty() { self.app_name.clone() } else { self.name.clone() };
        match self.step {
            NewServiceStep::ApplicationForm => ServiceSpec {
                name: svc_name,
                project_id: self.project_id.clone(),
                source: ServiceSource::Registry { image: String::new() },
                port: 80,
                host_port: None,
                domain: None,
                env_vars: vec![],
                volumes: vec![],
                healthcheck: Healthcheck::default(),
                replicas: 1,
                resources: ResourceLimits::default(),
            },
            NewServiceStep::ComposeForm => ServiceSpec {
                name: svc_name,
                project_id: self.project_id.clone(),
                source: ServiceSource::Compose(ComposeSource {
                    content: String::new(),
                }),
                port: 80,
                host_port: None,
                domain: None,
                env_vars: vec![],
                volumes: vec![],
                healthcheck: Healthcheck::default(),
                replicas: 1,
                resources: ResourceLimits::default(),
            },
            NewServiceStep::DatabaseForm => ServiceSpec {
                name: svc_name,
                project_id: self.project_id.clone(),
                source: ServiceSource::Registry { image: self.docker_image.clone() },
                port: self.db_kind.map(|d| d.default_port()).unwrap_or(5432),
                host_port: None,
                domain: None,
                env_vars: self.db_env_vars(),
                volumes: vec![],
                healthcheck: Healthcheck::default(),
                replicas: 1,
                resources: ResourceLimits::default(),
            },
            _ => unreachable!(),
        }
    }
}

// ── Project detail ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ProjectDetailTab {
    #[default]
    Services,
    Environment,
}

impl ProjectDetailTab {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Services => "Services",
            Self::Environment => "Environment",
        }
    }
}

// ── Env tab ───────────────────────────────────────────────────────────────────

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

// ── IPC plumbing ──────────────────────────────────────────────────────────────

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
    LoadDeployments,
    LoadHomeDeployments,
    LoadLogs,
    LoadBuildLogs,
    CreateProject,
    DeleteProject,
    UpdateProjectEnv,
    CreateService,
    UpdateService,
    DeleteService,
    Deploy,
    ServiceStop,
    ServiceReload,
    LoadWebhookUrl,
    RegenerateWebhook,
    LoadServerSettings,
    SaveServerSettings,
}

// ── Compose tab ───────────────────────────────────────────────────────────────

pub struct ComposeTabState {
    pub editing: bool,
    pub textarea: tui_textarea::TextArea<'static>,
}

impl ComposeTabState {
    pub fn new(content: &str) -> Self {
        let lines: Vec<String> = if content.is_empty() {
            vec![String::new()]
        } else {
            content.lines().map(String::from).collect()
        };
        let mut textarea = tui_textarea::TextArea::new(lines);
        textarea.set_cursor_style(ratatui::style::Style::default()); // cursor invisível por padrão
        textarea.set_line_number_style(ratatui::style::Style::default().fg(ratatui::style::Color::DarkGray));
        Self { editing: false, textarea }
    }

    pub fn content(&self) -> String {
        self.textarea.lines().join("\n")
    }

    pub fn set_editing(&mut self, editing: bool) {
        use ratatui::style::{Color, Modifier, Style};
        self.editing = editing;
        if editing {
            self.textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        } else {
            self.textarea.set_cursor_style(Style::default());
        }
    }
}

impl Default for ComposeTabState {
    fn default() -> Self {
        Self::new("")
    }
}

// ── Settings — Web Server ─────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ServerSettingsField {
    #[default]
    ServerDomain,
    Save,
}

impl ServerSettingsField {
    pub fn next(self) -> Self {
        match self {
            Self::ServerDomain => Self::Save,
            Self::Save => Self::ServerDomain,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            Self::ServerDomain => Self::Save,
            Self::Save => Self::ServerDomain,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ServerSettingsState {
    pub server_domain: String,
    pub focused: ServerSettingsField,
    pub loaded: bool,
}

// ── Runtime state ─────────────────────────────────────────────────────────────

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
