use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    /// Variáveis de ambiente herdadas por todos os serviços deste projeto no deploy.
    #[serde(default)]
    pub env_vars: Vec<EnvVar>,
    /// Comentários (`# ...`) do editor `.env` do projeto, ancorados por `key`
    /// (mesmo esquema do `ServiceSpec.env_comments`). Não participam da
    /// construção do ambiente — só a lista/edição via UI olha pra isso.
    #[serde(default)]
    pub env_comments: Vec<EnvComment>,
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
    #[serde(default)]
    pub tls_enabled: bool,
    pub env_vars: Vec<EnvVar>,
    /// Comentários (`# ...`) do editor `.env`, ancorados por `key` (posição
    /// preservada mesmo que as vars sejam reordenadas). Não participa da
    /// construção do ambiente do container — só a lista/edição via UI olha
    /// pra isso.
    #[serde(default)]
    pub env_comments: Vec<EnvComment>,
    pub volumes: Vec<VolumeMount>,
    pub healthcheck: Healthcheck,
    pub replicas: u32,
    pub resources: ResourceLimits,
    #[serde(default)]
    pub run_command: Option<String>,
    #[serde(default)]
    pub run_args: Vec<String>,
    /// Tipo de serviço gerenciado — banco (postgres | mongodb | mariadb |
    /// mysql | redis) ou broker (kafka | rabbitmq | nats). Controla a aba
    /// Connection no painel e a geração da internal connection URL (scheme por
    /// tipo: postgres://, amqp://, nats://, Kafka sem scheme).
    #[serde(default)]
    pub db_kind: Option<String>,
    /// Rotas HTTP de domínio. Cada domínio pode apontar para uma porta de
    /// container diferente e ligar TLS de forma independente — permite um
    /// serviço que responde em várias portas expor vários subdomínios.
    /// Retrocompat: quando vazio, cai no `domain`/`tls_enabled` legado (roteado
    /// para `port`). Ver [`ServiceSpec::domain_routes`].
    #[serde(default)]
    pub domains: Vec<DomainRoute>,
}

/// Uma rota HTTP de domínio de um serviço: qual domínio, para qual porta do
/// container e com ou sem TLS.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainRoute {
    pub domain: String,
    /// Porta do container que este domínio atende. `None` = a `port` padrão do
    /// serviço.
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub tls: bool,
}

impl DomainRoute {
    /// Porta de container efetiva (a própria, ou a `port` padrão do serviço).
    pub fn container_port(&self, default: u16) -> u16 {
        self.port.unwrap_or(default)
    }
}

pub fn normalize_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_was_dash = true; // Prevent leading dashes

    for c in name.to_lowercase().chars() {
        if c.is_alphanumeric() {
            out.push(c);
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('_');
            last_was_dash = true;
        }
    }

    out.trim_matches('_').to_string()
}

impl ServiceSpec {
    pub fn safe_name(&self) -> String {
        normalize_name(&self.name)
    }

    /// Rotas HTTP efetivas do serviço: a lista `domains` nova, ou — para specs
    /// antigos que só têm o campo `domain` — uma rota única sintetizada a partir
    /// do domínio legado (roteada para `port`, TLS = `tls_enabled`).
    pub fn domain_routes(&self) -> Vec<DomainRoute> {
        if !self.domains.is_empty() {
            self.domains.clone()
        } else if let Some(d) = &self.domain {
            vec![DomainRoute { domain: d.clone(), port: None, tls: self.tls_enabled }]
        } else {
            Vec::new()
        }
    }

    /// Move o domínio legado (`domain`/`tls_enabled`) para a lista `domains` e
    /// zera os campos legados, para que edições subsequentes operem numa única
    /// fonte de verdade (a lista). Idempotente.
    pub fn materialize_domains(&mut self) {
        if self.domains.is_empty() {
            self.domains = self.domain_routes();
        }
        self.domain = None;
        self.tls_enabled = false;
    }
}

/// Resolve as variáveis de ambiente para um serviço, combinando as do projeto e as do serviço.
/// As variáveis do serviço têm precedência sobre as do projeto.
pub fn resolve_env_vars(project: &Project, service: &Service) -> std::collections::HashMap<String, EnvVar> {
    let mut resolved = std::collections::HashMap::new();

    // Adiciona as variáveis de ambiente do projeto
    for env_var in &project.env_vars {
        resolved.insert(env_var.key.clone(), env_var.clone());
    }

    // Adiciona/sobrescreve com as variáveis de ambiente do serviço
    for env_var in &service.spec.env_vars {
        resolved.insert(env_var.key.clone(), env_var.clone());
    }

    resolved
}

/// Heuristic used by the clients' General tab to decide whether a
/// "Repository URL / Image" value denotes a Git source to clone+build, or a
/// Docker image to pull. Recognizes the common git URL schemes, including
/// local `file://` repositories (which the daemon clones via `git clone`).
pub fn looks_like_git_url(url: &str) -> bool {
    let u = url.trim();
    u.starts_with("https://")
        || u.starts_with("http://")
        || u.starts_with("git@")
        || u.starts_with("ssh://")
        || u.starts_with("file://")
        || u.ends_with(".git")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceSource {
    Registry { image: String },
    Git(GitSource),
    Compose(ComposeSource),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComposeSource {
    #[serde(alias = "compose_file")]
    pub content: String,
}

impl Default for ComposeSource {
    fn default() -> Self {
        Self {
            content: String::new(),
        }
    }
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
    pub username: Option<String>,
    /// When set, deploys resolve the clone token from the connected Git
    /// provider (Gitea OAuth/PAT) instead of (or in addition to) `credentials`.
    #[serde(default)]
    pub provider_id: Option<String>,
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
            username: None,
            provider_id: None,
        }
    }
}

// ── Git providers (Gitea OAuth2 / PAT) ────────────────────────────────────────

/// Which hosted Git service a provider connects to. Only Gitea is implemented
/// today; the enum exists so GitHub/GitLab can be added without a wire break.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GitProviderKind {
    Gitea,
}

impl GitProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gitea => "gitea",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "gitea" => Some(Self::Gitea),
            _ => None,
        }
    }
}

/// How a provider authenticates: full OAuth2 authorization-code flow, or a
/// pasted Personal Access Token.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GitAuthMode {
    OAuth,
    Pat,
}

impl GitAuthMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OAuth => "oauth",
            Self::Pat => "pat",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "oauth" => Some(Self::OAuth),
            "pat" => Some(Self::Pat),
            _ => None,
        }
    }
}

/// The connected account, populated once OAuth completes (or the PAT validates).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitAccount {
    pub login: String,
    pub avatar_url: Option<String>,
}

/// A connected Git provider, as exposed to clients. Secrets (client secret,
/// access/refresh tokens, PAT) are kept daemon-side and never serialized here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitProvider {
    pub id: String,
    pub kind: GitProviderKind,
    pub name: String,
    pub base_url: String,
    pub auth_mode: GitAuthMode,
    pub oauth_client_id: Option<String>,
    /// `Some` once the connection is established.
    pub account: Option<GitAccount>,
    pub created_at: DateTime<Utc>,
}

/// A repository listed from a provider's API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitRepo {
    pub full_name: String,
    pub clone_url: String,
    pub default_branch: String,
    pub private: bool,
}

/// A branch listed from a provider's API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GitBranch {
    pub name: String,
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
    ComposingUp,
}

impl DeployState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Live | Self::Stopped | Self::Failed | Self::Pruning
        )
    }

    pub fn to_percent(&self) -> u8 {
        match self {
            Self::Pending => 5,
            Self::ResolvingDeps => 10,
            Self::PullingImage => 30,
            Self::CloningRepo => 20,
            Self::BuildingImage => 50,
            Self::ComposingUp => 60,
            Self::Staging => 65,
            Self::HealthcheckPolling => 75,
            Self::SwappingIn => 85,
            Self::Draining => 90,
            Self::Promoting => 95,
            Self::Live | Self::Stopped | Self::Pruning => 100,
            Self::RollingBack | Self::Failed => 0,
        }
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
            Self::ComposingUp => "ComposingUp",
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

/// Uma linha de comentário do editor `.env`, ancorada à var que a segue (por
/// `key`, não por posição) — assim ela sobrevive a uma reordenação de vars.
/// `before_key: None` é um comentário solto/final, sem var associada.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EnvComment {
    /// Linha completa, incluindo o `#`.
    pub text: String,
    pub before_key: Option<String>,
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
    None,
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
pub struct ActiveDeployInfo {
    pub deployment_id: String,
    pub service_id: String,
    pub service_name: String,
    pub project_name: String,
    pub state: DeployState,
    pub percent: u8,
    pub started_at: DateTime<Utc>,
    pub elapsed_secs: u64,
    pub current_state_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployEngineSummary {
    pub version: String,
    pub uptime_secs: u64,
    pub active: Vec<ActiveDeployInfo>,
    pub recent: Vec<ActiveDeployInfo>,
    pub total_24h: u64,
    pub successful_24h: u64,
    pub failed_24h: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsPoint {
    pub cpu_percent: f64,
    pub mem_used_bytes: u64,
    pub mem_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub load_avg_1: f64,
    pub load_avg_5: f64,
    pub load_avg_15: f64,
    pub timestamp: DateTime<Utc>,
}

/// One Docker image (from `docker system df`), independent of whether it's
/// currently used by any service the daemon manages — the Docker tab lists
/// every image on the host, not just rustploy's own.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerImageInfo {
    pub id: String,
    /// Repo:tag references; empty when the image is untagged (`<none>`, a
    /// dangling layer left behind by a superseded build).
    pub tags: Vec<String>,
    pub size_bytes: u64,
    pub created: DateTime<Utc>,
    /// Containers (running or stopped) referencing this image. `docker
    /// system df` always computes this (unlike the plain image-list
    /// endpoint, which leaves it `-1` unless asked).
    pub containers: i64,
    /// Best-effort project/service this image belongs to, inferred from its
    /// tag (`rp_<safe_name>:...` for Git builds, exact string match for
    /// registry images). `None` when nothing in the daemon's DB matches —
    /// e.g. a manually-pulled image, or one left over from a deleted service.
    pub project: Option<String>,
    pub service: Option<String>,
}

/// One Docker volume, independent of rustploy ownership (rustploy itself
/// only ever bind-mounts host paths — see `docker/containers.rs` — so named
/// volumes here are either created by the user directly or implicitly by an
/// image's own `VOLUME` directive).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerVolumeInfo {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    /// `false` when no container currently references the volume — the
    /// "unused" set `docker volume prune` would remove.
    pub in_use: bool,
    /// Containers referencing this volume, when Docker reports it (`-1`
    /// when not computed/available for this driver).
    pub ref_count: i64,
    /// Disk usage in bytes, when available (`-1` otherwise — e.g. non-`local`
    /// drivers never report a size).
    pub size_bytes: i64,
}

/// One Docker container on the host (running or stopped), for the Docker tab's
/// Containers sub-tab. Host-wide (not just rustploy-managed) — `managed`/
/// `project`/`service` are best-effort attribution (label `rustploy.managed` +
/// the `rp_<safe_name>_...` container-name convention).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerContainerInfo {
    /// Full container id (for removal).
    pub id: String,
    /// Primary name, sem a barra inicial que o Docker prefixa.
    pub name: String,
    pub image: String,
    /// Estado bruto do Docker: "running", "exited", "created", "paused", …
    pub state: String,
    /// Linha de status legível do Docker (ex.: "Exited (0) 2 hours ago").
    pub status: String,
    /// `true` quando tem o label `rustploy.managed=true`.
    pub managed: bool,
    pub project: Option<String>,
    pub service: Option<String>,
}

/// One Docker network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerNetworkInfo {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    /// `false` when no container is attached — the "unused" set `docker
    /// network prune` would remove (rustploy's own per-project networks
    /// included, once their last service is gone).
    pub in_use: bool,
    pub container_count: usize,
    /// Project this network belongs to, inferred from the
    /// `rp_net_<project_id_short>` naming convention (see
    /// `docker/networks.rs::project_network_name`). `None` for non-rustploy
    /// networks (the built-in `bridge`/`host`/`none`, or manually created ones).
    pub project: Option<String>,
}

// ── Jobs (tarefas one-shot via docker-compose, agendadas ou manuais) ──────────

/// Uma tarefa one-shot: sobe um stack docker-compose próprio (efêmero — roda
/// até terminar e é removido, nunca fica de pé como um `Service`), anexado à
/// rede do projeto do `trigger_service_id` (que também empresta as env vars de
/// base). Disparado por agendamento (`recurrence`) e/ou manualmente
/// (`Command::JobRunNow`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Job {
    pub id: String,
    pub project_id: String,
    /// Serviço do projeto que empresta rede Docker + env vars de base ao job
    /// (o job não roda dentro do container dele — sobe um stack novo).
    pub trigger_service_id: String,
    pub name: String,
    /// Conteúdo de um `docker-compose.yml` — mesma UX/formato de
    /// `ComposeSource.content`.
    pub compose: String,
    /// Nome do serviço, dentro de `compose`, cujo exit code decide
    /// sucesso/falha do job (`docker compose up --exit-code-from`).
    pub main_service: String,
    pub enabled: bool,
    /// `None` = sem agendamento automático, só `Command::JobRunNow`.
    pub recurrence: Option<Recurrence>,
    pub last_run_at: Option<DateTime<Utc>>,
    /// `None` quando `recurrence` é `None` (nada a agendar).
    pub next_run_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Recorrência estruturada (sem expressão cron — ver decisão em
/// docs/planejamento; `chrono` já é dependência, sem precisar de crate nova).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Recurrence {
    IntervalHours(u32),
    Daily { hour: u8, minute: u8 },
    /// `weekday`: 0=segunda .. 6=domingo (`chrono::Weekday::num_days_from_monday`).
    Weekly { weekday: u8, hour: u8, minute: u8 },
}

impl Recurrence {
    /// Próximo disparo estritamente depois de `from`.
    pub fn next_after(&self, from: DateTime<Utc>) -> DateTime<Utc> {
        match self {
            Recurrence::IntervalHours(hours) => from + chrono::Duration::hours((*hours).max(1) as i64),
            Recurrence::Daily { hour, minute } => {
                let mut candidate = Self::at_time(from, *hour, *minute);
                if candidate <= from {
                    candidate += chrono::Duration::days(1);
                }
                candidate
            }
            Recurrence::Weekly { weekday, hour, minute } => {
                let mut candidate = Self::at_time(from, *hour, *minute);
                let target = (*weekday % 7) as i64;
                let current = from.weekday().num_days_from_monday() as i64;
                let mut delta_days = target - current;
                if delta_days < 0 {
                    delta_days += 7;
                }
                candidate += chrono::Duration::days(delta_days);
                if candidate <= from {
                    candidate += chrono::Duration::days(7);
                }
                candidate
            }
        }
    }

    /// `from`'s calendar date at the given UTC hour:minute (seconds zeroed).
    fn at_time(from: DateTime<Utc>, hour: u8, minute: u8) -> DateTime<Utc> {
        from.date_naive()
            .and_hms_opt(hour.min(23) as u32, minute.min(59) as u32, 0)
            .unwrap_or_else(|| from.date_naive().and_hms_opt(0, 0, 0).unwrap())
            .and_utc()
    }
}

/// Uma execução de um `Job`. `success: None` enquanto ainda está rodando.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobRun {
    pub id: String,
    pub job_id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub success: Option<bool>,
}

/// `Job` + nomes resolvidos, pra listagem cross-project (sidebar Schedules) —
/// mesmo espírito de `DeploymentSummary`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub job: Job,
    pub project_name: String,
    pub trigger_service_name: String,
    pub last_run: Option<JobRun>,
}

#[cfg(test)]
mod job_tests {
    use super::*;

    #[test]
    fn interval_advances_by_hours() {
        let from = "2026-01-01T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let next = Recurrence::IntervalHours(6).next_after(from);
        assert_eq!(next, from + chrono::Duration::hours(6));
    }

    #[test]
    fn daily_before_time_today_fires_today() {
        let from = "2026-01-01T02:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let next = Recurrence::Daily { hour: 3, minute: 0 }.next_after(from);
        assert_eq!(next, "2026-01-01T03:00:00Z".parse::<DateTime<Utc>>().unwrap());
    }

    #[test]
    fn daily_after_time_today_fires_tomorrow() {
        let from = "2026-01-01T04:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let next = Recurrence::Daily { hour: 3, minute: 0 }.next_after(from);
        assert_eq!(next, "2026-01-02T03:00:00Z".parse::<DateTime<Utc>>().unwrap());
    }

    #[test]
    fn weekly_picks_next_matching_weekday() {
        // 2026-01-01 é uma quinta-feira (weekday 3, 0=segunda).
        let from = "2026-01-01T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        assert_eq!(from.weekday().num_days_from_monday(), 3);
        // Próxima segunda (weekday 0) às 03:00 -> 2026-01-05.
        let next = Recurrence::Weekly { weekday: 0, hour: 3, minute: 0 }.next_after(from);
        assert_eq!(next, "2026-01-05T03:00:00Z".parse::<DateTime<Utc>>().unwrap());
    }

    #[test]
    fn weekly_same_weekday_but_time_passed_rolls_to_next_week() {
        // 2026-01-01 é quinta (weekday 3); pede quinta às 03:00, mas já são 10:00 -> semana que vem.
        let from = "2026-01-01T10:00:00Z".parse::<DateTime<Utc>>().unwrap();
        let next = Recurrence::Weekly { weekday: 3, hour: 3, minute: 0 }.next_after(from);
        assert_eq!(next, "2026-01-08T03:00:00Z".parse::<DateTime<Utc>>().unwrap());
    }

    #[test]
    fn recurrence_json_round_trip() {
        for r in [
            Recurrence::IntervalHours(6),
            Recurrence::Daily { hour: 3, minute: 30 },
            Recurrence::Weekly { weekday: 6, hour: 0, minute: 0 },
        ] {
            let json = serde_json::to_string(&r).unwrap();
            let back: Recurrence = serde_json::from_str(&json).unwrap();
            assert_eq!(r, back);
        }
    }
}

#[cfg(test)]
mod git_provider_tests {
    use super::*;
    use crate::protocol::{Command, Response};

    #[test]
    fn git_source_json_back_compat_without_provider_id() {
        // Specs persisted before the feature lack `provider_id`; serde default
        // must fill it as None.
        let json = r#"{
            "url":"https://gitea.test/u/r.git","branch":"main","root_path":".",
            "watch_paths":[],"submodules":false,"dockerfile_path":"Dockerfile",
            "build_context":".","build_stage":null,"credentials":null,"username":null
        }"#;
        let g: GitSource = serde_json::from_str(json).unwrap();
        assert_eq!(g.provider_id, None);
        assert_eq!(g.branch, "main");
    }

    #[test]
    fn command_postcard_round_trip() {
        let cmd = Command::GitProviderCreate {
            kind: GitProviderKind::Gitea,
            name: "Gitea".into(),
            base_url: "https://gitea.test".into(),
            auth_mode: GitAuthMode::OAuth,
            oauth_client_id: Some("cid".into()),
            oauth_client_secret: Some("secret".into()),
            pat: None,
        };
        let bytes = postcard::to_allocvec(&cmd).unwrap();
        let back: Command = postcard::from_bytes(&bytes).unwrap();
        assert!(matches!(back, Command::GitProviderCreate { kind: GitProviderKind::Gitea, .. }));
    }

    #[test]
    fn response_postcard_round_trip() {
        let resp = Response::GitProviders(vec![GitProvider {
            id: "1".into(),
            kind: GitProviderKind::Gitea,
            name: "Gitea".into(),
            base_url: "https://gitea.test".into(),
            auth_mode: GitAuthMode::Pat,
            oauth_client_id: None,
            account: Some(GitAccount { login: "alice".into(), avatar_url: None }),
            created_at: Utc::now(),
        }]);
        let bytes = postcard::to_allocvec(&resp).unwrap();
        let back: Response = postcard::from_bytes(&bytes).unwrap();
        match back {
            Response::GitProviders(v) => assert_eq!(v[0].account.as_ref().unwrap().login, "alice"),
            _ => panic!("variante errada"),
        }
    }
}
