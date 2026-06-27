//! Application state, navigation enums, form buffers and message types.

use shared::{
    ComposeSource, ContainerMetricsPoint, DaemonStatus, DeployEngineSummary, Deployment,
    DeploymentSummary, EnvVar, EnvVarValue, GitAuthMode, GitBranch, GitProvider, GitRepo,
    GitSource, Healthcheck, HealthcheckKind, Project, ResourceLimits, Service, ServiceSource,
    compose_project_name,
    ServiceSpec,
};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;

pub const MAX_LOG_LINES: usize = 1000;
pub const MAX_METRIC_POINTS: usize = 60;

/// Default RWP port appended to a URL authority that omits one.
pub const DEFAULT_RWP_PORT: u16 = 8787;

/// The only scheme the remote client speaks.
pub const RWP_SCHEME: &str = "rwp://";

/// Canonicalizes user input into an `rwp://…` URL for display and storage.
///
/// Adds the `rwp://` scheme when the user typed a bare authority, and leaves
/// the authority (`host[:port]`) exactly as typed — the default port is only
/// materialized later by [`connect_target`] for the live connection, never
/// baked into what we show or persist.
pub fn normalize_url(input: &str) -> String {
    let a = input.trim();
    if a.is_empty() {
        return String::new();
    }
    if a.contains("://") {
        a.to_string()
    } else {
        format!("{RWP_SCHEME}{a}")
    }
}

/// Parses an `rwp://` URL into the `host:port` target for `TcpStream::connect`,
/// filling in [`DEFAULT_RWP_PORT`] when the authority omits one. Any other
/// scheme is rejected — the client only connects over RWP.
pub fn connect_target(url: &str) -> anyhow::Result<String> {
    let a = url.trim();
    let authority = match a.split_once("://") {
        Some((scheme, rest)) => {
            anyhow::ensure!(
                scheme.eq_ignore_ascii_case("rwp"),
                "esquema não suportado: {scheme}:// — use rwp://"
            );
            rest
        }
        None => a,
    };
    // Drop anything past the authority the user may have pasted (path/query).
    let authority = authority.split(['/', '?', '#']).next().unwrap_or("").trim();
    anyhow::ensure!(!authority.is_empty(), "URL sem host");
    Ok(append_default_port(authority))
}

/// Appends `:8787` to an authority that doesn't already carry an explicit port.
///
/// Handles the common forms an admin types: a hostname or IPv4 (`host` →
/// `host:8787`, `host:1234` unchanged) and a bracketed IPv6 literal
/// (`[::1]` → `[::1]:8787`, `[::1]:9000` unchanged). A bare IPv6 literal
/// (multiple colons, no brackets) is left untouched since a trailing port
/// can't be told apart from the address.
fn append_default_port(authority: &str) -> String {
    let a = authority;
    if let Some(rest) = a.strip_prefix('[') {
        // Bracketed IPv6: has a port only if "]:" appears after the literal.
        return if rest.contains("]:") {
            a.to_string()
        } else {
            format!("{a}:{DEFAULT_RWP_PORT}")
        };
    }
    // Bare IPv6 literal — ambiguous, leave as the user typed it.
    if a.matches(':').count() >= 2 {
        return a.to_string();
    }
    if a.contains(':') {
        a.to_string()
    } else {
        format!("{a}:{DEFAULT_RWP_PORT}")
    }
}

// ── Color palette (mirrors the TUI cyan/yellow/green accents) ─────────────────
pub mod palette {
    use iced::Color;
    pub const CYAN: Color = Color { r: 0.30, g: 0.78, b: 0.90, a: 1.0 };
    pub const YELLOW: Color = Color { r: 0.90, g: 0.78, b: 0.30, a: 1.0 };
    pub const GREEN: Color = Color { r: 0.40, g: 0.80, b: 0.45, a: 1.0 };
    pub const RED: Color = Color { r: 0.90, g: 0.40, b: 0.40, a: 1.0 };
    pub const MAGENTA: Color = Color { r: 0.80, g: 0.45, b: 0.80, a: 1.0 };
    pub const GRAY: Color = Color { r: 0.55, g: 0.58, b: 0.62, a: 1.0 };
    pub const WHITE: Color = Color { r: 0.90, g: 0.92, b: 0.94, a: 1.0 };
}

// ── Navigation ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarItem {
    HomeDeployments,
    HomeMonitoring,
    HomeSchedules,
    HomeIngress,
    HomeDocker,
    HomeDeployEngine,
    HomeRequests,
    Projects,
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
    pub const HOME: &'static [SidebarItem] = &[
        Self::HomeDeployments,
        Self::HomeMonitoring,
        Self::HomeSchedules,
        Self::HomeIngress,
        Self::HomeDocker,
        Self::HomeDeployEngine,
        Self::HomeRequests,
    ];
    pub const SETTINGS: &'static [SidebarItem] = &[
        Self::SettingsWebServer,
        Self::SettingsProfile,
        Self::SettingsUsers,
        Self::SettingsAuditLogs,
        Self::SettingsSshKeys,
        Self::SettingsTags,
        Self::SettingsGit,
        Self::SettingsRegistry,
        Self::SettingsS3,
        Self::SettingsCerts,
        Self::SettingsSso,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::HomeDeployments => "Deployments",
            Self::HomeMonitoring => "Monitoring",
            Self::HomeSchedules => "Schedules",
            Self::HomeIngress => "Ingress",
            Self::HomeDocker => "Docker",
            Self::HomeDeployEngine => "Deploy Engine",
            Self::HomeRequests => "Requests",
            Self::Projects => "Projects",
            Self::SettingsWebServer => "Web Server",
            Self::SettingsProfile => "Profile",
            Self::SettingsUsers => "Users",
            Self::SettingsAuditLogs => "Audit Logs",
            Self::SettingsSshKeys => "SSH Keys",
            Self::SettingsTags => "Tags",
            Self::SettingsGit => "Git",
            Self::SettingsRegistry => "Registry",
            Self::SettingsS3 => "S3 Destinations",
            Self::SettingsCerts => "Certificates",
            Self::SettingsSso => "SSO",
            Self::Account => "Account",
        }
    }

    pub fn to_view(self) -> View {
        match self {
            Self::HomeDeployments => View::HomeDeployments,
            Self::HomeMonitoring => View::HomeMonitoring,
            Self::HomeSchedules => View::HomeSchedules,
            Self::HomeIngress => View::HomeIngress,
            Self::HomeDocker => View::HomeDocker,
            Self::HomeDeployEngine => View::HomeDeployEngine,
            Self::HomeRequests => View::HomeRequests,
            Self::Projects => View::Projects,
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
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    HomeDeployments,
    HomeMonitoring,
    HomeSchedules,
    HomeIngress,
    HomeDocker,
    HomeDeployEngine,
    HomeRequests,
    Projects,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectTab {
    Services,
    Environment,
    Secrets,
    Settings,
}

impl ProjectTab {
    pub const ALL: &'static [ProjectTab] =
        &[Self::Services, Self::Environment, Self::Secrets, Self::Settings];
    pub fn label(self) -> &'static str {
        match self {
            Self::Services => "Services",
            Self::Environment => "Environment",
            Self::Secrets => "Secrets",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceTab {
    General,
    Connection,
    Environment,
    Domains,
    Deployments,
    Healthcheck,
    Logs,
    Patches,
    Advanced,
}

impl ServiceTab {
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Connection => "Connection",
            Self::Environment => "Environment",
            Self::Domains => "Domains",
            Self::Deployments => "Deployments",
            Self::Healthcheck => "Healthcheck",
            Self::Logs => "Logs",
            Self::Patches => "Patches",
            Self::Advanced => "Advanced",
        }
    }
}

/// Sub-tabs of the General tab's "Provider" section: the generic Git/registry
/// URL form, or the connected-Gitea picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderTab {
    Git,
    Gitea,
}

// ── Databases ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbKind {
    MongoDB,
    Postgres,
    MariaDB,
    MySQL,
    Redis,
}

impl DbKind {
    pub const ALL: &'static [DbKind] = &[
        Self::MongoDB,
        Self::Postgres,
        Self::MariaDB,
        Self::MySQL,
        Self::Redis,
    ];
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
    pub fn kind_id(self) -> &'static str {
        match self {
            Self::MongoDB => "mongodb",
            Self::Postgres => "postgres",
            Self::MariaDB => "mariadb",
            Self::MySQL => "mysql",
            Self::Redis => "redis",
        }
    }
    pub fn yaml_service_name(self) -> &'static str {
        match self {
            Self::MongoDB => "mongo",
            Self::Postgres => "postgres",
            Self::MariaDB => "mariadb",
            Self::MySQL => "mysql",
            Self::Redis => "redis",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "postgres" => Some(Self::Postgres),
            "mongodb" => Some(Self::MongoDB),
            "mariadb" => Some(Self::MariaDB),
            "mysql" => Some(Self::MySQL),
            "redis" => Some(Self::Redis),
            _ => None,
        }
    }

    /// Detecta o tipo de banco: campo `db_kind` do spec tem precedência;
    /// fallback para a env var legada `RUSTPLOY_DB_KIND`.
    pub fn detect(resolved_env_vars: &HashMap<String, EnvVar>, svc: &Service) -> Option<Self> {
        if let Some(kind) = &svc.spec.db_kind {
            return Self::from_str(kind);
        }
        resolved_env_vars
            .get("RUSTPLOY_DB_KIND")
            .and_then(|e| match &e.value {
                EnvVarValue::Plain(s) => Self::from_str(s),
                _ => None,
            })
    }
}

// ── New-service wizard ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsStep {
    PickType,
    PickDb,
    AppForm,
    DbForm,
    ComposeForm,
    PickTemplate,
    TemplateForm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceKind {
    Application,
    Database,
    Compose,
    Template,
}

impl ServiceKind {
    pub const ALL: &'static [ServiceKind] = &[
        Self::Application,
        Self::Database,
        Self::Compose,
        Self::Template,
    ];
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

#[derive(Debug, Default)]
pub struct NsForm {
    pub project_id: String,
    pub step: NsStep,
    pub db_kind: Option<DbKind>,
    pub name: String,
    pub app_name: String,
    pub description: String,
    pub db_name: String,
    pub db_user: String,
    pub db_password: String,
    pub db_root_password: String,
    pub docker_image: String,
    pub use_replica_sets: bool,
    pub template_cat: usize,
    pub template_search: String,
    pub selected_template: Option<&'static shared::templates::Template>,
    pub template_var_values: Vec<String>,
}

impl Default for NsStep {
    fn default() -> Self {
        Self::PickType
    }
}

impl NsForm {
    pub fn new(project_id: String) -> Self {
        Self {
            project_id,
            step: NsStep::PickType,
            ..Default::default()
        }
    }

    pub fn select_db(&mut self, db: DbKind) {
        self.db_kind = Some(db);
        self.docker_image = db.default_image().to_string();
        self.db_password = token_urlsafe(22);
        self.db_root_password = token_urlsafe(22);
        self.step = NsStep::DbForm;
    }

    pub fn select_template(&mut self, t: &'static shared::templates::Template) {
        self.selected_template = Some(t);
        self.template_var_values = t
            .variables
            .iter()
            .map(|v| v.default.unwrap_or("").to_string())
            .collect();
        self.name = t.name.to_lowercase().replace(' ', "-");
        self.step = NsStep::TemplateForm;
    }

    fn db_env_vars(&self) -> Vec<EnvVar> {
        let plain = |k: &str, v: &str| EnvVar {
            key: k.to_string(),
            value: EnvVarValue::Plain(v.to_string()),
        };
        let Some(kind) = self.db_kind else {
            return vec![];
        };
        let mut vars = vec![];
        match kind {
            DbKind::Postgres => {
                vars.push(plain("POSTGRES_DB", &self.db_name));
                vars.push(plain("POSTGRES_USER", &self.db_user));
                vars.push(plain("POSTGRES_PASSWORD", &self.db_password));
            }
            DbKind::MongoDB => {
                vars.push(plain("MONGO_INITDB_ROOT_USERNAME", &self.db_user));
                vars.push(plain("MONGO_INITDB_ROOT_PASSWORD", &self.db_password));
                if self.use_replica_sets {
                    vars.push(plain("MONGO_REPLICA_SET_NAME", "rs0"));
                }
            }
            DbKind::MariaDB | DbKind::MySQL => {
                vars.push(plain("MYSQL_DATABASE", &self.db_name));
                vars.push(plain("MYSQL_USER", &self.db_user));
                vars.push(plain("MYSQL_PASSWORD", &self.db_password));
                vars.push(plain("MYSQL_ROOT_PASSWORD", &self.db_root_password));
            }
            DbKind::Redis => {
                if !self.db_password.is_empty() {
                    vars.push(plain("REDIS_PASSWORD", &self.db_password));
                }
            }
        }
        vars
    }

    fn generate_db_compose(&self) -> String {
        let img = &self.docker_image;
        match self.db_kind {
            Some(DbKind::Postgres) => format!(
                "services:\n  postgres:\n    image: {img}\n    restart: unless-stopped\n    environment:\n      POSTGRES_DB: ${{POSTGRES_DB}}\n      POSTGRES_USER: ${{POSTGRES_USER}}\n      POSTGRES_PASSWORD: ${{POSTGRES_PASSWORD}}\n    volumes:\n      - pgdata:/var/lib/postgresql\n\nvolumes:\n  pgdata:\n"
            ),
            Some(DbKind::MongoDB) => {
                let replica = if self.use_replica_sets {
                    "      MONGO_REPLICA_SET_NAME: rs0\n"
                } else {
                    ""
                };
                format!(
                    "services:\n  mongo:\n    image: {img}\n    restart: unless-stopped\n    environment:\n      MONGO_INITDB_ROOT_USERNAME: ${{MONGO_INITDB_ROOT_USERNAME}}\n      MONGO_INITDB_ROOT_PASSWORD: ${{MONGO_INITDB_ROOT_PASSWORD}}\n{replica}    volumes:\n      - mongodata:/data/db\n\nvolumes:\n  mongodata:\n"
                )
            }
            Some(DbKind::MariaDB) => format!(
                "services:\n  mariadb:\n    image: {img}\n    restart: unless-stopped\n    environment:\n      MYSQL_DATABASE: ${{MYSQL_DATABASE}}\n      MYSQL_USER: ${{MYSQL_USER}}\n      MYSQL_PASSWORD: ${{MYSQL_PASSWORD}}\n      MYSQL_ROOT_PASSWORD: ${{MYSQL_ROOT_PASSWORD}}\n    volumes:\n      - mariadbdata:/var/lib/mysql\n\nvolumes:\n  mariadbdata:\n"
            ),
            Some(DbKind::MySQL) => format!(
                "services:\n  mysql:\n    image: {img}\n    restart: unless-stopped\n    environment:\n      MYSQL_DATABASE: ${{MYSQL_DATABASE}}\n      MYSQL_USER: ${{MYSQL_USER}}\n      MYSQL_PASSWORD: ${{MYSQL_PASSWORD}}\n      MYSQL_ROOT_PASSWORD: ${{MYSQL_ROOT_PASSWORD}}\n    volumes:\n      - mysqldata:/var/lib/mysql\n\nvolumes:\n  mysqldata:\n"
            ),
            Some(DbKind::Redis) => {
                let cmd = if self.db_password.is_empty() {
                    String::new()
                } else {
                    "    command: redis-server --requirepass ${REDIS_PASSWORD}\n".to_string()
                };
                format!(
                    "services:\n  redis:\n    image: {img}\n    restart: unless-stopped\n{cmd}    volumes:\n      - redisdata:/data\n\nvolumes:\n  redisdata:\n"
                )
            }
            None => String::new(),
        }
    }

    /// Builds the `ServiceSpec` to send for the current step. Returns None when
    /// the step is not a final form.
    pub fn to_spec(&self) -> Option<ServiceSpec> {
        let svc_name = if !self.app_name.is_empty() {
            self.app_name.clone()
        } else {
            self.name.clone()
        };
        let base = |source: ServiceSource, port: u16, env: Vec<EnvVar>, db_kind: Option<String>| ServiceSpec {
            name: svc_name.clone(),
            project_id: self.project_id.clone(),
            source,
            port,
            host_port: None,
            domain: None,
            tls_enabled: false,
            env_vars: env,
            volumes: vec![],
            healthcheck: Healthcheck::default(),
            replicas: 1,
            resources: ResourceLimits::default(),
            run_command: None,
            run_args: vec![],
            db_kind,
        };
        match self.step {
            NsStep::AppForm => Some(base(
                ServiceSource::Registry { image: String::new() },
                80,
                vec![],
                None,
            )),
            NsStep::ComposeForm => Some(base(
                ServiceSource::Compose(ComposeSource { content: String::new() }),
                80,
                vec![],
                None,
            )),
            NsStep::DbForm => Some(base(
                ServiceSource::Compose(ComposeSource {
                    content: self.generate_db_compose(),
                }),
                self.db_kind.map(|d| d.default_port()).unwrap_or(5432),
                self.db_env_vars(),
                self.db_kind.map(|d| d.kind_id().to_string()),
            )),
            NsStep::TemplateForm => {
                let t = self.selected_template?;
                let content = shared::templates::render_compose(t, &self.template_var_values);
                let name = if self.name.is_empty() {
                    t.name.to_lowercase().replace(' ', "-")
                } else {
                    self.name.clone()
                };
                Some(ServiceSpec {
                    name,
                    project_id: self.project_id.clone(),
                    source: ServiceSource::Compose(ComposeSource { content }),
                    port: t.default_port,
                    host_port: None,
                    domain: None,
                    tls_enabled: false,
                    env_vars: vec![],
                    volumes: vec![],
                    healthcheck: Healthcheck::default(),
                    replicas: 1,
                    resources: ResourceLimits::default(),
                    run_command: None,
                    run_args: vec![],
                    db_kind: None,
                })
            }
            _ => None,
        }
    }
}

// ── Form buffers for the service-detail tabs ──────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct GeneralForm {
    pub repo_url: String,
    pub branch: String,
    pub username: String,
    pub credentials: String,
    pub build_path: String,
    pub watch_paths: String,
    pub submodules: bool,
    pub port: String,
    pub dockerfile: String,
    pub context_path: String,
    pub build_stage: String,
    pub is_git: bool,
    pub provider_id: Option<String>,
}

impl GeneralForm {
    pub fn from_service(svc: &Service) -> Self {
        match &svc.spec.source {
            ServiceSource::Git(g) => Self {
                repo_url: g.url.clone(),
                branch: g.branch.clone(),
                username: g.username.clone().unwrap_or_default(),
                credentials: g.credentials.clone().unwrap_or_default(),
                build_path: g.root_path.clone(),
                watch_paths: g.watch_paths.join(", "),
                submodules: g.submodules,
                port: svc.spec.port.to_string(),
                dockerfile: g.dockerfile_path.clone(),
                context_path: g.build_context.clone(),
                build_stage: g.build_stage.clone().unwrap_or_default(),
                is_git: true,
                provider_id: g.provider_id.clone(),
            },
            ServiceSource::Registry { image } => Self {
                repo_url: image.clone(),
                branch: String::new(),
                build_path: ".".into(),
                port: svc.spec.port.to_string(),
                dockerfile: "Dockerfile".into(),
                context_path: ".".into(),
                is_git: false,
                provider_id: None,
                ..Default::default()
            },
            ServiceSource::Compose(_) => Self {
                port: svc.spec.port.to_string(),
                is_git: false,
                ..Default::default()
            },
        }
    }

    pub fn to_git_source(&self) -> GitSource {
        GitSource {
            url: self.repo_url.clone(),
            branch: if self.branch.trim().is_empty() {
                "main".into()
            } else {
                self.branch.clone()
            },
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
            build_stage: opt(&self.build_stage),
            credentials: opt(&self.credentials),
            username: opt(&self.username),
            provider_id: self.provider_id.clone(),
        }
    }
}

// ── Git providers (Gitea) ─────────────────────────────────────────────────────

/// Buffer for the "Connect Gitea" form on the Settings → Git screen.
#[derive(Debug, Clone)]
pub struct GpForm {
    pub name: String,
    pub base_url: String,
    pub mode: GitAuthMode,
    pub client_id: String,
    pub client_secret: String,
    pub pat: String,
}

impl Default for GpForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            base_url: String::new(),
            mode: GitAuthMode::OAuth,
            client_id: String::new(),
            client_secret: String::new(),
            pat: String::new(),
        }
    }
}

/// A connected account as a `pick_list` option (label = name + login).
#[derive(Debug, Clone)]
pub struct ProviderChoice {
    pub id: String,
    pub label: String,
}
impl std::fmt::Display for ProviderChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}
impl PartialEq for ProviderChoice {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ProviderChoice {}

/// A repository as a `pick_list` option (label = full_name).
#[derive(Debug, Clone)]
pub struct RepoChoice {
    pub full_name: String,
    pub clone_url: String,
    pub default_branch: String,
}
impl std::fmt::Display for RepoChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.full_name)
    }
}
impl PartialEq for RepoChoice {
    fn eq(&self, other: &Self) -> bool {
        self.full_name == other.full_name
    }
}
impl Eq for RepoChoice {}

/// State of the service-detail Gitea tab: the picked provider/repo/branch plus
/// the build settings. Saved as a `ServiceSource::Git` carrying `provider_id`.
#[derive(Debug, Clone, Default)]
pub struct GiteaForm {
    pub provider_id: Option<String>,
    pub repo_full_name: Option<String>,
    pub clone_url: String,
    pub branch: Option<String>,
    pub build_path: String,
    pub submodules: bool,
    pub watch_paths: Vec<String>,
    pub port: String,
    pub dockerfile: String,
    pub context_path: String,
}

impl GiteaForm {
    pub fn from_service(svc: &Service) -> Self {
        match &svc.spec.source {
            ServiceSource::Git(g) if g.provider_id.is_some() => Self {
                provider_id: g.provider_id.clone(),
                repo_full_name: None,
                clone_url: g.url.clone(),
                branch: Some(g.branch.clone()),
                build_path: g.root_path.clone(),
                submodules: g.submodules,
                watch_paths: g.watch_paths.clone(),
                port: svc.spec.port.to_string(),
                dockerfile: g.dockerfile_path.clone(),
                context_path: g.build_context.clone(),
            },
            _ => Self {
                build_path: ".".into(),
                port: svc.spec.port.to_string(),
                dockerfile: "Dockerfile".into(),
                context_path: ".".into(),
                ..Default::default()
            },
        }
    }

    pub fn to_git_source(&self) -> GitSource {
        GitSource {
            url: self.clone_url.clone(),
            branch: self.branch.clone().unwrap_or_else(|| "main".into()),
            root_path: if self.build_path.trim().is_empty() {
                ".".into()
            } else {
                self.build_path.clone()
            },
            watch_paths: self
                .watch_paths
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            submodules: self.submodules,
            dockerfile_path: if self.dockerfile.trim().is_empty() {
                "Dockerfile".into()
            } else {
                self.dockerfile.clone()
            },
            build_context: if self.context_path.trim().is_empty() {
                ".".into()
            } else {
                self.context_path.clone()
            },
            build_stage: None,
            credentials: None,
            username: None,
            provider_id: self.provider_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HealthForm {
    pub kind: String,
    pub http_path: String,
    pub expected_status: String,
    pub interval: String,
    pub timeout: String,
    pub retries: String,
    pub start_period: String,
}

impl HealthForm {
    pub fn from_service(svc: &Service) -> Self {
        let hc = &svc.spec.healthcheck;
        let (kind, path, status) = match &hc.kind {
            HealthcheckKind::None => ("None", String::new(), "200".to_string()),
            HealthcheckKind::Tcp => ("Tcp", String::new(), "200".to_string()),
            HealthcheckKind::Http { path, expected_status } => {
                ("Http", path.clone(), expected_status.to_string())
            }
            HealthcheckKind::DockerNative => ("DockerNative", String::new(), "200".to_string()),
        };
        Self {
            kind: kind.to_string(),
            http_path: path,
            expected_status: status,
            interval: hc.interval_secs.to_string(),
            timeout: hc.timeout_secs.to_string(),
            retries: hc.retries.to_string(),
            start_period: hc.start_period_secs.to_string(),
        }
    }

    pub fn to_healthcheck(&self) -> Healthcheck {
        let kind = match self.kind.as_str() {
            "None" => HealthcheckKind::None,
            "Http" => HealthcheckKind::Http {
                path: if self.http_path.is_empty() { "/".into() } else { self.http_path.clone() },
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

#[derive(Debug, Clone, Default)]
pub struct DomainsForm {
    pub domain: String,
    pub host_port: String,
    pub tls_enabled: bool,
}

impl DomainsForm {
    pub fn from_service(svc: &Service) -> Self {
        Self {
            domain: svc.spec.domain.clone().unwrap_or_default(),
            host_port: svc.spec.host_port.map(|p| p.to_string()).unwrap_or_default(),
            tls_enabled: svc.spec.tls_enabled,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AdvancedForm {
    pub replicas: String,
    pub run_command: String,
    pub run_args: Vec<String>,
}

impl AdvancedForm {
    pub fn from_service(svc: &Service) -> Self {
        Self {
            replicas: svc.spec.replicas.to_string(),
            run_command: svc.spec.run_command.clone().unwrap_or_default(),
            run_args: svc.spec.run_args.clone(),
        }
    }
}

/// Pre-computed database connection details, shown read-only/copyable in the
/// Connection tab. Stored in state so the text_inputs can borrow the strings.
#[derive(Debug, Clone)]
pub struct ConnInfo {
    pub db_label: String,
    pub host: String,
    pub port: String,
    pub url: String,
    pub fields: Vec<(String, String)>,
}

impl ConnInfo {
    pub fn from_service(project: &Project, svc: &Service) -> Option<Self> {
        let resolved_env_vars = shared::resolve_env_vars(project, svc);
        let db = DbKind::detect(&resolved_env_vars, svc)?;

        let env_plain = |key: &str| -> String {
            resolved_env_vars
                .get(key)
                .and_then(|e| match &e.value {
                    EnvVarValue::Plain(v) => Some(v.clone()),
                    _ => None,
                })
                .unwrap_or_default()
        };
        let project_name = compose_project_name(&svc.id, &svc.spec.name);
        let host = format!("{}-{}-1", project_name, db.yaml_service_name());
        let port = db.default_port();
        let (url, fields) = match db {
            DbKind::Postgres => {
                let (d, u, p) = (env_plain("POSTGRES_DB"), env_plain("POSTGRES_USER"), env_plain("POSTGRES_PASSWORD"));
                (
                    format!("postgresql://{u}:{p}@{host}:{port}/{d}"),
                    vec![("Database".into(), d), ("User".into(), u), ("Password".into(), p)],
                )
            }
            DbKind::MongoDB => {
                let (u, p) = (env_plain("MONGO_INITDB_ROOT_USERNAME"), env_plain("MONGO_INITDB_ROOT_PASSWORD"));
                (
                    format!("mongodb://{u}:{p}@{host}:{port}"),
                    vec![("User".into(), u), ("Password".into(), p)],
                )
            }
            DbKind::MariaDB | DbKind::MySQL => {
                let (d, u, p) = (env_plain("MYSQL_DATABASE"), env_plain("MYSQL_USER"), env_plain("MYSQL_PASSWORD"));
                (
                    format!("mysql://{u}:{p}@{host}:{port}/{d}"),
                    vec![("Database".into(), d), ("User".into(), u), ("Password".into(), p)],
                )
            }
            DbKind::Redis => {
                let p = env_plain("REDIS_PASSWORD");
                let url = if p.is_empty() {
                    format!("redis://{host}:{port}")
                } else {
                    format!("redis://:{p}@{host}:{port}")
                };
                let fields = if p.is_empty() { vec![] } else { vec![("Password".into(), p)] };
                (url, fields)
            }
        };
        Some(ConnInfo {
            db_label: db.label().to_string(),
            host,
            port: port.to_string(),
            url,
            fields,
        })
    }
}

/// Inline key/value editor used by the environment and secret panels.
#[derive(Debug, Clone, Default)]
pub struct KvEditor {
    pub open: bool,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: chrono::DateTime<chrono::Utc>,
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
pub enum ConfirmAction {
    DeleteProject(String),
    DeleteService(String),
}

// ── IPC context / worker events / messages ────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Ctx {
    Projects,
    Services,
    Deployments,
    HomeDeployments,
    DeployEngine,
    Logs,
    BuildLogs,
    Secrets,
    WebhookUrl,
    ServerSettings,
    DaemonStatus,
    CreateProject,
    UpdateProject,
    DeleteProject(String),
    UpdateProjectEnv,
    CreateService,
    UpdateService,
    DeleteService(String),
    Deploy,
    Action(String),
    // Git providers
    GitProviders,
    CreateGitProvider,
    OAuthStart,
    GitRepos,
    GitBranches,
    GitProviderDeleted,
}

#[derive(Debug, Clone)]
pub enum WorkerEvent {
    Ready(UnboundedSender<(Ctx, shared::Command)>),
    Connected,
    Reply(Ctx, shared::Response),
    Event(shared::Event),
    Error(String),
    Disconnected,
}

#[derive(Debug, Clone)]
pub enum Message {
    // connection
    UrlChanged(String),
    TokenChanged(String),
    RememberUrlToggled(bool),
    RememberTokenToggled(bool),
    Connect,
    Disconnect,
    Worker(WorkerEvent),
    Tick,
    // navigation
    Sidebar(SidebarItem),
    OpenProject(String),
    BackToProjects,
    OpenService(String),
    BackToProject,
    ProjectTab(ProjectTab),
    ServiceTab(ServiceTab),
    // new project
    NewProjectOpen,
    NpName(String),
    NpDesc(String),
    NpSubmit,
    NpCancel,
    // project settings / delete
    PsName(String),
    PsDesc(String),
    PsSave,
    AskDelete(ConfirmAction),
    ConfirmYes,
    ConfirmNo,
    // project env
    PEnvOpen,
    PEnvKey(String),
    PEnvVal(String),
    PEnvSubmit,
    PEnvCancel,
    PEnvDelete(usize),
    PEnvTextOpen,
    PEnvTextAction(iced::widget::text_editor::Action),
    PEnvImport,
    PEnvExport,
    // secrets
    SecretOpen,
    SecretName(String),
    SecretVal(String),
    SecretSubmit,
    SecretCancel,
    SecretDelete(String),
    // service actions
    SvcDeploy,
    SvcReload,
    SvcStop,
    SvcRollback,
    // general/git form
    GenField(GenField, String),
    GenSubmodules(bool),
    GenSave,
    ComposeAction(iced::widget::text_editor::Action),
    ComposeSave,
    BuildLogScrollTo(f32),
    LogAction(iced::widget::text_editor::Action),
    BuildLogModal(bool),
    // service env
    SEnvOpen,
    SEnvKey(String),
    SEnvVal(String),
    SEnvSubmit,
    SEnvCancel,
    SEnvDelete(usize),
    SEnvTextOpen,
    SEnvTextAction(iced::widget::text_editor::Action),
    SEnvImport,
    SEnvExport,
    // domains
    DomDomain(String),
    DomHostPort(String),
    DomTls(bool),
    DomSave,
    // healthcheck
    HcKind(String),
    HcField(HcField, String),
    HcSave,
    // advanced
    AdvReplicas(String),
    AdvCommand(String),
    AdvArgAdd,
    AdvArg(usize, String),
    AdvArgDelete(usize),
    AdvSave,
    // deployments
    DeploySelect(usize),
    WebhookRegen,
    // new service wizard
    NewServiceOpen,
    NsCancel,
    NsBack,
    NsPickType(ServiceKind),
    NsPickDb(DbKind),
    NsField(NsField, String),
    NsReplica(bool),
    NsTemplateCat(usize),
    NsTemplateSearch(String),
    NsTemplateSelect(&'static str),
    NsTemplateVar(usize, String),
    NsCreate,
    // server settings
    SsDomain(String),
    SsEmail(String),
    SsSave,
    // git providers (Settings → Git)
    GpName(String),
    GpBaseUrl(String),
    GpMode(GitAuthMode),
    GpClientId(String),
    GpClientSecret(String),
    GpPat(String),
    GpConnect,
    GpDelete(String),
    GpRefresh,
    // provider sub-tab + service Gitea form
    ProviderTabChanged(ProviderTab),
    GiteaProviderPick(ProviderChoice),
    GiteaRepoPick(RepoChoice),
    GiteaBranchPick(String),
    GiteaBuildPath(String),
    GiteaDockerfile(String),
    GiteaSubmodules(bool),
    GiteaPort(String),
    GiteaWatchAdd,
    GiteaWatch(usize, String),
    GiteaWatchDelete(usize),
    GiteaSave,
    DismissNotification,
    /// Copy a value to the system clipboard.
    Copy(String),
    /// No-op used to make read-only text inputs selectable/copyable.
    Ignore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenField {
    RepoUrl,
    Branch,
    Username,
    Credentials,
    BuildPath,
    WatchPaths,
    Port,
    Dockerfile,
    ContextPath,
    BuildStage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HcField {
    HttpPath,
    ExpectedStatus,
    Interval,
    Timeout,
    Retries,
    StartPeriod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsField {
    Name,
    AppName,
    Description,
    DbName,
    DbUser,
    DbPassword,
    DbRootPassword,
    Image,
}

// ── The application state ─────────────────────────────────────────────────────

pub struct Session {
    pub addr: String,
    pub token: Option<String>,
}

pub struct App {
    // connection
    pub url: String,
    pub token: String,
    pub remember_url: bool,
    pub remember_token: bool,
    pub connect_seq: u64,
    pub session: Option<Session>,
    pub worker_tx: Option<UnboundedSender<(Ctx, shared::Command)>>,
    pub connected: bool,
    pub status_msg: String,
    pub error: Option<String>,
    pub notification: Option<Notification>,

    // navigation
    pub view: View,
    pub sidebar: SidebarItem,
    pub project_tab: ProjectTab,
    pub service_tab: ServiceTab,
    pub provider_tab: ProviderTab,

    // data
    pub daemon_status: Option<DaemonStatus>,
    pub projects: Vec<Project>,
    pub active_project_id: Option<String>,
    pub services: Vec<Service>,
    pub active_service_id: Option<String>,
    pub home_deployments: Vec<DeploymentSummary>,
    pub deploy_engine: Option<DeployEngineSummary>,
    pub service_deployments: Vec<Deployment>,
    pub selected_deployment: usize,
    pub conn_info: Option<ConnInfo>,
    pub build_logs: HashMap<String, Vec<LogLine>>,
    pub logs: HashMap<String, Vec<LogLine>>,
    pub log_editor: iced::widget::text_editor::Content,
    pub metrics: HashMap<String, Vec<ContainerMetricsPoint>>,
    pub project_secrets: Vec<String>,
    pub webhook_url: Option<String>,

    // forms
    pub build_log_modal_open: bool,
    pub new_project_open: bool,
    pub np_name: String,
    pub np_desc: String,
    pub ps_name: String,
    pub ps_desc: String,
    pub p_env_editor: KvEditor,
    pub p_env_text_open: bool,
    pub p_env_text_editor: iced::widget::text_editor::Content,
    pub secret_editor: KvEditor,
    pub general: GeneralForm,
    pub compose_editor: iced::widget::text_editor::Content,
    pub s_env_editor: KvEditor,
    pub s_env_text_open: bool,
    pub s_env_text_editor: iced::widget::text_editor::Content,
    pub domains: DomainsForm,
    pub health: HealthForm,
    pub advanced: AdvancedForm,
    pub ns: Option<NsForm>,
    pub confirm: Option<ConfirmAction>,
    pub ss_domain: String,
    pub ss_email: String,
    pub ss_loaded: bool,

    // git providers
    pub git_providers: Vec<GitProvider>,
    pub gp_form: GpForm,
    pub gitea: GiteaForm,
    pub git_repos: Vec<GitRepo>,
    pub git_branches: Vec<GitBranch>,

    // periodic refresh
    pub log_ticks: u32,
    pub engine_ticks: u32,
}

impl App {
    pub fn new(url: String) -> Self {
        Self {
            url,
            token: String::new(),
            remember_url: false,
            remember_token: false,
            connect_seq: 0,
            session: None,
            worker_tx: None,
            connected: false,
            status_msg: "Desconectado".into(),
            error: None,
            notification: None,
            view: View::HomeDeployments,
            sidebar: SidebarItem::HomeDeployments,
            project_tab: ProjectTab::Services,
            service_tab: ServiceTab::General,
            provider_tab: ProviderTab::Git,
            daemon_status: None,
            projects: Vec::new(),
            active_project_id: None,
            services: Vec::new(),
            active_service_id: None,
            home_deployments: Vec::new(),
            deploy_engine: None,
            service_deployments: Vec::new(),
            selected_deployment: 0,
            conn_info: None,
            build_logs: HashMap::new(),
            logs: HashMap::new(),
            log_editor: iced::widget::text_editor::Content::new(),
            metrics: HashMap::new(),
            project_secrets: Vec::new(),
            webhook_url: None,
            build_log_modal_open: false,
            new_project_open: false,
            np_name: String::new(),
            np_desc: String::new(),
            ps_name: String::new(),
            ps_desc: String::new(),
            p_env_editor: KvEditor::default(),
            p_env_text_open: false,
            p_env_text_editor: iced::widget::text_editor::Content::new(),
            secret_editor: KvEditor::default(),
            general: GeneralForm::default(),
            compose_editor: iced::widget::text_editor::Content::new(),
            s_env_editor: KvEditor::default(),
            s_env_text_open: false,
            s_env_text_editor: iced::widget::text_editor::Content::new(),
            domains: DomainsForm::default(),
            health: HealthForm::default(),
            advanced: AdvancedForm::default(),
            ns: None,
            confirm: None,
            ss_domain: String::new(),
            ss_email: String::new(),
            ss_loaded: false,
            git_providers: Vec::new(),
            gp_form: GpForm::default(),
            gitea: GiteaForm::default(),
            git_repos: Vec::new(),
            git_branches: Vec::new(),
            log_ticks: 0,
            engine_ticks: 0,
        }
    }

    /// Builds the app, prefilling the connect screen from saved preferences.
    /// `default_url` is used only when no URL was remembered.
    pub fn with_prefs(default_url: String, prefs: crate::store::RemotePrefs) -> Self {
        let mut app = Self::new(default_url);
        app.remember_url = prefs.remember_url;
        app.remember_token = prefs.remember_token;
        if prefs.remember_url {
            if let Some(url) = prefs.url.filter(|s| !s.is_empty()) {
                app.url = url;
            }
        }
        if prefs.remember_token {
            if let Some(token) = prefs.token {
                app.token = token;
            }
        }
        app
    }

    /// Persists the current "remember" choices and, when enabled, the URL
    /// and token so the next launch can prefill them.
    pub fn persist_prefs(&self) {
        crate::store::RemotePrefs {
            remember_url: self.remember_url,
            remember_token: self.remember_token,
            url: self.remember_url.then(|| self.url.clone()),
            token: self.remember_token.then(|| self.token.clone()),
        }
        .save();
    }

    pub fn send(&self, ctx: Ctx, cmd: shared::Command) {
        if let Some(tx) = &self.worker_tx {
            let _ = tx.send((ctx, cmd));
        }
    }

    pub fn notify(&mut self, msg: impl Into<String>, is_error: bool) {
        self.notification = Some(Notification {
            message: msg.into(),
            is_error,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(4),
        });
    }

    pub fn daemon_status(&self) -> Option<&DaemonStatus> {
        self.daemon_status.as_ref()
    }

    pub fn current_project(&self) -> Option<&Project> {
        let id = self.active_project_id.as_deref()?;
        self.projects.iter().find(|p| p.id == id)
    }

    pub fn current_service(&self) -> Option<&Service> {
        let id = self.active_service_id.as_deref()?;
        self.services.iter().find(|s| s.id == id)
    }

    pub fn project_services(&self) -> Vec<&Service> {
        self.services.iter().collect()
    }
}

pub fn opt(s: &str) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Gera um token URL-safe de `n` caracteres usando apenas std.
/// Usa SystemTime como semente com splitmix64 — adequado para senhas de setup.
pub fn token_urlsafe(n: usize) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(12345);
    let mut state = seed ^ (seed << 13) ^ (seed >> 7) ^ 0x9e3779b97f4a7c15;
    (0..n)
        .map(|_| {
            // splitmix64
            state = state.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^= z >> 31;
            ALPHABET[(z as usize) % ALPHABET.len()] as char
        })
        .collect()
}
