use crate::manifest::ApplyReport;
use crate::models::*;
use serde::{Deserialize, Serialize};

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
        env_comments: Vec<EnvComment>,
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
    DeployDelete {
        deployment_id: String,
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
        acme_email: Option<String>,
        registry_domain: Option<String>,
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
    /// Os manifestos trafegam como YAML (e não como structs) para que o daemon
    /// receba exatamente o texto do arquivo que o usuário edita; o parse fica
    /// num lugar só, com `serde_yaml`. Ver `docs/infra-as-code.md`.
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
    /// Exporta TODOS os projetos+serviços num único manifesto raiz
    /// (`ServerManifest`). Todo valor de env var `Plain` é redigido para
    /// `${KEY}` (nunca o valor real no YAML); `Secret` continua como
    /// `secret:NOME`. Resposta: `ManifestBundle` (YAML + `.env` complementar
    /// com os valores reais das vars `Plain`).
    ManifestExportAll,
    /// Importa um manifesto raiz (`projects:`) ou de projeto único
    /// (`project:`) junto com o texto `.env` que resolve os `${VAR}` nele
    /// usados. A interpolação roda no daemon (reaproveita
    /// `ProjectManifest::interpolate`); se sobrar alguma `${VAR}` sem valor em
    /// qualquer projeto, nada é aplicado e a resposta é `MissingEnvVars`. Sem
    /// faltantes, reconcilia exatamente como `ManifestApply` (aditivo por
    /// padrão; `prune`/`deploy` com a mesma semântica).
    ManifestImport {
        yaml: String,
        dotenv: String,
        prune: bool,
        deploy: bool,
    },

    // Jobs (tarefas one-shot via docker-compose, agendadas ou manuais)
    JobCreate {
        project_id: String,
        trigger_service_id: String,
        name: String,
        compose: String,
        main_service: String,
        recurrence: Option<Recurrence>,
    },
    JobUpdate {
        id: String,
        name: String,
        compose: String,
        main_service: String,
        enabled: bool,
        recurrence: Option<Recurrence>,
    },
    JobDelete {
        id: String,
    },
    /// Jobs de um único projeto (aba "Jobs" do projeto).
    JobList {
        project_id: String,
    },
    /// Todos os jobs, de todos os projetos (tela global da sidebar "Schedules").
    JobListAll,
    /// Dispara o job imediatamente, fora do agendamento.
    JobRunNow {
        id: String,
    },
    JobRunHistory {
        job_id: String,
        limit: usize,
    },
    GetJobLogs {
        job_run_id: String,
    },

    // Docker cleanup
    PruneContainers,
    /// `all=true` remove volumes mesmo que não sejam anônimos (equivalente ao
    /// `docker volume prune --all`); `false` é o padrão do Docker.
    PruneVolumes {
        all: bool,
    },
    /// `all=true` remove toda imagem sem uso, não só as dangling/untagged
    /// (equivalente ao `docker image prune -a`); `false` é o padrão do Docker.
    PruneImages {
        all: bool,
    },
    PruneBuildCache,
    PruneNetworks,

    // Docker inventory (every image/volume/network on the host, not just
    // rustploy-managed ones — see `shared::DockerImageInfo` etc.)
    DockerImages,
    DockerVolumes,
    DockerNetworks,
    /// Todo container do host (rodando + parado), para a sub-aba Containers.
    DockerContainers,

    // Remoção INDIVIDUAL de um recurso Docker (o par por-item dos `Prune*`).
    // O Docker recusa remover recursos em uso (sem force) — o erro é propagado.
    RemoveContainer { id: String },
    RemoveImage { id: String },
    RemoveVolume { name: String },
    RemoveNetwork { id: String },
    /// Stops every container labeled `rustploy.managed=true`, regardless of
    /// what the DB's service status currently says (more robust than
    /// looping over `Service` rows one `ServiceStop` at a time — see
    /// `Command::ServiceStop`). Scoped to rustploy's own containers; never
    /// touches unrelated containers on the same Docker host.
    StopAllManaged,

    // Env var backup / restore
    /// Lista os snapshots disponíveis (retorna Vec<String> com nomes de ficheiro).
    EnvBackupList,
    /// Restaura o snapshot com o nome dado (caminho relativo ao backup_dir).
    EnvBackupRestore {
        snapshot: String,
    },

    // Infrastructure
    Ping,
    DaemonStatus,
    DeployEngineStatus,

    // Git providers (Gitea OAuth2 / PAT)
    GitProviderList,
    GitProviderCreate {
        kind: GitProviderKind,
        name: String,
        base_url: String,
        auth_mode: GitAuthMode,
        oauth_client_id: Option<String>,
        oauth_client_secret: Option<String>,
        /// Personal Access Token, when `auth_mode == Pat`.
        pat: Option<String>,
    },
    GitProviderDelete {
        id: String,
    },
    /// Returns the Gitea authorization URL for the client to open in a browser.
    GitOAuthStart {
        provider_id: String,
    },
    GitRepoList {
        provider_id: String,
    },
    GitBranchList {
        provider_id: String,
        repo_full_name: String,
    },

    // Wizard "Novo serviço" (catálogos + criação server-side). O cliente Luau
    // só dirige a UI; o daemon monta o ServiceSpec via `shared::wizard` (que tem
    // acesso aos blueprints de `templates`).
    WizardCatalog {
        search: String,
    },
    WizardCreate(crate::wizard::WizardCreateReq),
    /// Snapshot completo do dashboard como JSON (o mesmo que o SSE `/api/events`
    /// empurra a cada 2s). O cliente usa após uma mutação para refletir a
    /// mudança na hora, sem esperar o próximo tick. Resposta: `Snapshot(String)`.
    Snapshot,

    // Registry OCI embutido — sub-aba Docker > Registry (somente leitura +
    // delete; criar conteúdo só acontece via `docker push` externo).
    RegistryStatus,
    RegistryRepoList,
    RegistryTagList { repo: String },
    /// Remove a tag; se outra tag apontar pro mesmo digest (mesmo manifest),
    /// ela também é removida — mesma semântica do DELETE da OCI spec (por
    /// digest, não por tag).
    RegistryTagDelete { repo: String, tag: String },
    RegistryRepoDelete { repo: String },
    /// Garbage collection do registry: remove manifests pendurados (sem tag e
    /// não referenciados por nenhum index), blobs órfãos (metadados + arquivos
    /// do CAS) e uploads abandonados há mais de 24 h. Resposta:
    /// `RegistryGcResult`.
    RegistryGc,
    /// Cria um token de acesso (Basic auth) — `scope` é `"pull"` ou `"push"`.
    /// Resposta traz o segredo em texto plano UMA ÚNICA VEZ.
    RegistryTokenCreate { name: String, scope: String },
    RegistryTokenList,
    RegistryTokenRevoke { name: String },
    /// Move um deploy enfileirado para o início da fila ("furar fila"). Sem
    /// efeito se o id não estiver na fila (ex.: já rodando/terminado).
    /// Resposta: `Ok`.
    DeployQueuePromote { deployment_id: String },
    /// Reordena a fila para exatamente a ordem dada (ids de deployment). Ids
    /// ausentes/desconhecidos são ignorados; enfileirados omitidos ficam ao fim
    /// preservando a ordem relativa. Usado pelo drag-and-drop da GUI.
    /// Resposta: `Ok`.
    DeployQueueReorder { order: Vec<String> },
    /// Pausa (`true`) ou retoma (`false`) a fila global. Pausada, o worker não
    /// puxa o próximo deploy; o que já estiver rodando segue até o fim.
    /// Resposta: `Ok`.
    DeployQueuePause { paused: bool },
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
    SystemMetrics(SystemMetricsPoint),
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
    /// Uma linha de stdout/stderr do `docker compose` de um `JobRun` (mesma
    /// forma de `Event::BuildLog`).
    JobLogLine {
        job_run_id: String,
        job_id: String,
        line: String,
        timestamp: chrono::DateTime<chrono::Utc>,
        stream: LogStream,
    },
    /// Início/fim de uma execução de job.
    JobRunStateChanged {
        job_id: String,
        job_run_id: String,
        running: bool,
        success: Option<bool>,
    },
    /// A fila global de deploys mudou (enfileirou, começou, terminou, reordenou,
    /// cancelou ou pausou/retomou). Sinal leve: o cliente refaz o
    /// `DeployEngineStatus` ao recebê-lo. Anexado no fim do enum de propósito.
    DeployQueueChanged,
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
            Event::DaemonReady { .. }
            | Event::Error { .. }
            | Event::SystemMetrics(_)
            | Event::JobLogLine { .. }
            | Event::JobRunStateChanged { .. }
            | Event::DeployQueueChanged => true,
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
    DaemonSettings {
        /// URL pública da API, **derivada** de `[api] domain`/`port` — base das
        /// URLs de webhook e do callback OAuth. Só-leitura: não existe setter,
        /// muda-se editando a config do daemon.
        public_base_url: String,
        acme_email: Option<String>,
        registry_domain: Option<String>,
    },
    SecretNames(Vec<String>),
    ManifestReport(ApplyReport),
    /// Manifesto YAML serializado (resposta de `ManifestExport`).
    Manifest(String),
    /// YAML + `.env` complementar (resposta de `ManifestExportAll`).
    ManifestBundle { yaml: String, dotenv: String },
    /// `${VAR}` sem valor correspondente no `.env` (resposta de
    /// `ManifestImport` quando faltam variáveis) — nada foi aplicado.
    MissingEnvVars(Vec<String>),

    // Git providers
    GitProviders(Vec<GitProvider>),
    GitProviderInfo(GitProvider),
    /// Authorization URL the client should open (resposta de `GitOAuthStart`).
    OAuthUrl(String),
    GitRepos(Vec<GitRepo>),
    GitBranches(Vec<GitBranch>),

    PruneResult { count: u32, reclaimed_bytes: u64 },
    EnvBackupSnapshots(Vec<String>),

    // Docker inventory
    DockerImages(Vec<DockerImageInfo>),
    DockerVolumes(Vec<DockerVolumeInfo>),
    DockerNetworks(Vec<DockerNetworkInfo>),
    /// Todo container do host (rodando + parado), resposta de `DockerContainers`.
    DockerContainers(Vec<DockerContainerInfo>),
    /// Count of rustploy-managed containers stopped (resposta de `StopAllManaged`).
    StopAllResult { count: u32 },

    /// Catálogos do wizard, prontos como JSON para o contexto (`ns_dbs`,
    /// `ns_brokers`, `ns_templates`). Resposta de `WizardCatalog`.
    WizardCatalog { dbs: String, brokers: String, templates: String },

    /// Snapshot do dashboard como JSON (resposta de `Snapshot`).
    Snapshot(String),

    // Jobs
    Job(Job),
    Jobs(Vec<Job>),
    JobSummaries(Vec<JobSummary>),
    JobRun(JobRun),
    JobRuns(Vec<JobRun>),
    /// Log de uma execução (resposta de `GetJobLogs`) — reaproveita
    /// `BuildLogLine`, mesma forma de linha (stream + texto + timestamp).
    JobLogs(Vec<BuildLogLine>),

    // Registry OCI embutido
    RegistryStatus(RegistryStatusInfo),
    RegistryRepos(Vec<RegistryRepoInfo>),
    RegistryTags(Vec<RegistryTagInfo>),
    /// Resultado do `RegistryGc`: arquivos removidos do CAS/uploads e bytes
    /// liberados no disco.
    RegistryGcResult { blobs_removed: u64, bytes_freed: u64 },
    /// Resposta de `RegistryTokenCreate` — `secret` só aparece aqui, uma vez.
    RegistryTokenCreated { name: String, secret: String },
    RegistryTokens(Vec<RegistryTokenInfo>),

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
