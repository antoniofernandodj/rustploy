# Índice: rustploy-shared — domínio, protocolo e nomes

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-shared/src/

### lib.rs — Tipos compartilhados entre o daemon e os clientes: domínio, protocolo e nomes derivados.
mod models — Alias de [`domain`], o nome antigo do módulo de modelos.

### naming.rs — Nomes derivados de um projeto ou serviço: containers, redes, stacks Compose e aliases DNS.
fn normalize_name(name) -> String
fn compose_project_name(svc_id, svc_name) -> String — Nome de stack Compose no formato **legado**, derivado a cada uso: primeiros 8 caracteres do ID (time…
fn new_compose_project_name(svc_id, svc_name) -> String — Nome de stack Compose de um serviço **novo**: `rp_<últimos 8 chars do ID>_<nome>` (a parte aleatória…
fn new_compose_project_name_long(svc_id, svc_name) -> String — Variante com o ID inteiro, para quando o índice `UNIQUE` recusa a curta.
fn new_project_network_name(project_id) -> String — Nome de rede Docker de um projeto **novo**: `rp_net_<ID inteiro, minúsculo>`.
fn compose_safe(name) -> String — Nome de serviço reduzido ao que o Docker Compose aceita num nome de projeto (ASCII minúsculo, dígito…
fn app_container_base(svc_id, svc_name) -> String — Base do nome de container de um serviço Application: `rp_<id8>_<safe>`.
fn app_network_alias(svc_name) -> String — Hostname de um serviço Application **dentro da rede do projeto**: `rp_<safe>`.
fn shared_alias(compose_project) -> String — Alias DNS **global** de um servidor de banco compartilhado, o único nome dele que atravessa a rede d…
const SHARED_ALIAS_PREFIX — Prefixo reservado: nenhum serviço/chave de Compose pode começar com ele.
fn legacy_project_net_for(project_id) -> String — Nome de rede no formato **legado** (`rp_net_` + 8 primeiros chars do ID), derivado a cada uso e sem …
(8 testes)

## rustploy-shared/src/domain/

### database.rs
struct ManagedDatabase { id, server_service_id, project_id, name, env_var, connection_limit, statement_timeout_ms, connection_url, created_at } — Database (e usuário dedicado) criado para um projeto dentro de um servidor compartilhado.
struct ManagedDatabaseCreateReq { server_service_id, project_id, name, env_var, overwrite_env, skip_env, connection_limit, statement_timeout_ms } — Pedido de novo database gerenciado (`Command::ManagedDatabaseCreate`).
struct MigrationStep { name, state, detail } — Um passo do assistente de migração.
struct Migration { id, project_id, source_service_id, source_database, dest_database_id, env_var, status, steps, log, created_at } — Migração de um banco antigo (serviço Compose do projeto) para um database gerenciado de um servidor …
struct MigrationStartReq { source_service_id, source_database, dest_database_id, env_var } — Pedido de migração (`Command::MigrationStart`).
struct SharedAccess { server_service_id, project_id, network, alias } — Projeto autorizado a alcançar um servidor compartilhado.
struct EnvChange { service_id, key, previous } — Onde a chave de conexão estava e o que valia (`None` = não existia ali); guardado para desfazer a tr…

### deployment.rs
struct Deployment { id, service_id, image, state, states_log, started_at, finished_at }
enum DeployState { Pending, PreDeployCheck, ResolvingDeps, PullingImage, CloningRepo, BuildingImage, Staging, HealthcheckPolling, SwappingIn, Draining, Promoting, Live, Stopped, RollingBack, Failed, Pruning, ComposingUp }
impl DeployState
  fn is_terminal() -> bool
  fn label() -> &'static str
struct StateTransition { from, to, at, message }
struct DeploymentSummary { deployment, service_name, project_name }
struct DaemonStatus { version, uptime_secs, services_running, services_total }
struct ActiveDeployInfo { deployment_id, service_id, service_name, project_name, state, started_at, elapsed_secs, current_state_secs, states }
struct DeployEngineSummary { version, uptime_secs, active, recent, total_24h, successful_24h, failed_24h, queued, paused }
(6 testes)

### docker_inventory.rs
struct DockerImageInfo { id, tags, size_bytes, created, containers, project, service } — One Docker image (from `docker system df`), independent of whether it's currently used by any servic…
struct DockerVolumeInfo { name, driver, mountpoint, in_use, ref_count, size_bytes } — One Docker volume, independent of rustploy ownership (rustploy itself only ever bind-mounts host pat…
struct DockerContainerInfo { id, name, image, state, status, managed, project, service }
struct DockerNetworkInfo { id, name, driver, scope, in_use, container_count, project } — One Docker network.
fn default_cleanup_retention_hours() -> u32
struct DockerCleanupConfig { enabled, recurrence, containers, images, images_all, volumes, volumes_all, networks, build_cache, build_cache_older_than_hours, images_older_than_hours, last_run_at, next_run_at } — Configuração de limpeza automática de recursos Docker não usados (containers parados, imagens, volum…
impl DockerCleanupConfig
  fn any_resource_enabled() -> bool
  fn recompute_next_run(now) — Recalcula `next_run_at` a partir de `last_run_at` (ou de `now` quando nunca rodou) — chamado ao salv…
struct DockerCleanupResourceResult { resource, count, reclaimed_bytes, error } — Resultado da limpeza de UM recurso, dentro de uma execução — usado tanto no resumo persistido (`Dock…
struct DockerCleanupLastRun { at, results } — Resumo persistido da última execução (chave `KEY_DOCKER_CLEANUP_LAST_RUN` em `daemon_settings`), pra…
const DEFAULT_CLEANUP_RETENTION_HOURS
impl Default for DockerCleanupConfig

### env.rs
fn resolve_env_vars(project, service) — Resolve as variáveis de ambiente para um serviço, combinando as do projeto e as do serviço.
struct EnvVar { key, value }
struct EnvComment { text, before_key } — Uma linha de comentário do editor `.env`, ancorada à var que a segue (por `key`, não por posição) — …
enum EnvVarValue { Plain, Secret }

### git.rs
enum GitProviderKind { Gitea, Github } — Which hosted Git service a provider connects to.
impl GitProviderKind
  fn as_str() -> &'static str
  fn from_str(s) -> Option<Self>
enum GitAuthMode { OAuth, Pat } — How a provider authenticates: full OAuth2 authorization-code flow, or a pasted Personal Access Token…
impl GitAuthMode
  fn as_str() -> &'static str
  fn from_str(s) -> Option<Self>
struct GitAccount { login, avatar_url } — The connected account, populated once OAuth completes (or the PAT validates).
struct GitProvider { id, kind, name, base_url, auth_mode, oauth_client_id, account, created_at } — A connected Git provider, as exposed to clients.
struct GitRepo { full_name, clone_url, default_branch, private } — A repository listed from a provider's API.
struct GitBranch { name } — A branch listed from a provider's API.
(4 testes)

### ingress.rs
struct IngressDomainRoute { domain, backends, service_id } — One Docker container on the host (running or stopped), for the Docker tab's Containers sub-tab.
struct IngressPortRoute { host_port, backends } — Uma rota de porta TCP viva (`host_port` → backends).
struct IngressSnapshot { domains, ports } — Foto da tabela de rotas do ingress (resposta de `Command::IngressRoutes`).
(5 testes)

### job.rs
struct Job { id, project_id, trigger_service_id, name, compose, git_source, main_service, env_vars, env_comments, enabled, recurrence, last_run_at, next_run_at, created_at } — Uma tarefa one-shot: sobe um stack docker-compose próprio (efêmero — roda até terminar e é removido,…
struct JobGitSource { provider_id, url, branch, username, credentials, compose_path } — Fonte git de um `Job` — irmã mais enxuta de `GitSource` (sem `dockerfile_path`/`build_context`/`buil…
enum Recurrence { IntervalHours, Daily, Weekly } — Recorrência estruturada (sem expressão cron — ver decisão em docs/planejamento; `chrono` já é depend…
impl Recurrence
  fn next_after(from) -> DateTime<Utc> — Próximo disparo estritamente depois de `from`.
  fn at_time(from, hour, minute) -> DateTime<Utc> — `from`'s calendar date at the given UTC hour:minute (seconds zeroed).
struct JobRun { id, job_id, started_at, finished_at, exit_code, success } — Uma execução de um `Job`.
struct JobSummary { job, project_name, trigger_service_name, last_run, warm } — `Job` + nomes resolvidos, pra listagem cross-project (sidebar Schedules) — mesmo espírito de `Deploy…
struct JobWarmStatus { job_id, warm, warming, reason, warmed_at } — Se as imagens e o cache de build de um job já estão no host, de modo que a próxima execução não baix…
impl Default for JobGitSource
(8 testes)

### manifest_report.rs
struct ApplyReport { actions, deployed } — Resultado de um `apply`: o que foi criado/atualizado/removido em cada recurso, mais a lista de servi…
struct ResourceAction { kind, name, action }
enum ResourceActionKind { Project, Service }
enum ActionVerb { Created, Updated, Unchanged, Deleted }
impl std::fmt::Display for ActionVerb
impl std::fmt::Display for ResourceActionKind

### metrics.rs
struct ContainerMetricsPoint { service_id, container_id, cpu_percent, mem_used_bytes, mem_limit_bytes, net_rx_bytes, net_tx_bytes, timestamp }
struct SystemMetricsPoint { cpu_percent, mem_used_bytes, mem_total_bytes, disk_used_bytes, disk_total_bytes, load_avg_1, load_avg_5, load_avg_15, timestamp }

### mod.rs — Modelos de domínio: projeto, serviço e suas fontes, deployment, jobs, bancos, registry e inventário …

### project.rs
struct Project { id, name, description, env_vars, env_comments, created_at }

### registry.rs
struct RegistryRepoInfo { name, tag_count, size_bytes, created_at } — Um repositório do registry embutido, para a lista da sub-aba Registry.
struct RegistryTagInfo { tag, digest, media_type, size_bytes, updated_at } — Uma tag de um repositório, com o manifest que ela aponta.
struct RegistryStatusInfo { enabled, port, domain, repo_count, blob_count, storage_bytes } — Status do registry embutido: config + agregados, para o cabeçalho da sub-aba Registry.
struct RegistryTokenInfo { name, scope, created_at, last_used_at } — Um token de acesso do registry (Basic auth) — o segredo em texto plano só aparece uma vez, em `Regis…

### service.rs
struct ServiceSpec { name, project_id, source, port, host_port, domain, tls_enabled, env_vars, env_comments, volumes, healthcheck, replicas, resources, run_command, run_args, db_kind, domains, pre_deploy_job_id, pre_deploy_job_ids, shared }
struct SharedServerConfig {  } — Configuração de um servidor de banco compartilhado.
struct DomainRoute { domain, port, tls } — Uma rota HTTP de domínio de um serviço: qual domínio, para qual porta do container e com ou sem TLS.
impl DomainRoute
  fn container_port(default) -> u16 — Porta de container efetiva (a própria, ou a `port` padrão do serviço).
impl Service
  fn compose_project_name() -> String — Nome da stack Compose: o gravado; na falta (banco ainda não migrado, ou serviço que virou Compose de…
impl ServiceSpec
  fn safe_name() -> String
  fn domain_routes() -> Vec<DomainRoute> — Rotas HTTP efetivas do serviço: a lista `domains` nova, ou — para specs antigos que só têm o campo `…
  fn ingress_container_ports() -> Vec<u16> — Portas de container que o ingress precisa alcançar: a de cada domínio, mais a `port` do serviço quan…
  fn materialize_domains() — Move o domínio legado (`domain`/`tls_enabled`) para a lista `domains` e zera os campos legados, para…
  fn pre_deploy_checks() -> Vec<String> — Fila efetiva de checks de pré-deploy: `pre_deploy_job_ids` quando não vazia, ou — para specs antigos…
  fn materialize_pre_deploy_checks() — Move o `pre_deploy_job_id` legado para `pre_deploy_job_ids` e zera o campo legado — mesmo idioma de …
struct Service { id, spec, compose_project, status, live_container_id, created_at, updated_at }
enum ServiceStatus { Stopped, Stopping, Deploying, Running, Degraded, Error, Queued }
struct VolumeMount { host_path, container_path, read_only }
struct Healthcheck { kind, interval_secs, timeout_secs, retries, start_period_secs }
enum HealthcheckKind { None, Http, Tcp, DockerNative }
struct ResourceLimits { cpu_shares, mem_limit_bytes }
impl Default for Healthcheck
impl std::fmt::Display for ServiceStatus

### service_bundle.rs
struct PlanVar { key, is_secret, suggested } — Uma variável listada na tela de checkboxes do export.
struct ServiceExportPlan { service_name, project_name, service_env, project_env, blocked } — Resposta de `ServiceExportPlan`: o que dá para levar, para a tela marcar.
enum ProjectEnvChoice { Keep, Overwrite, ServiceOnly, Ignore } — O que fazer, no destino, com UMA variável de projeto trazida no pacote.
struct ServiceImportReq { yaml, project_id, name, drop_domains, git_provider_id, vars, project_vars, secrets, project_env, deploy, dry_run } — Pedido de `ServiceImport`.
struct ImportWarning { code, message }
enum ProjectEnvState { New, Same, Conflict }
struct ProjectEnvStatus { key, state, is_secret, choice }
struct ServiceImportReport { dry_run, service_id, service_name, project_name, name_conflict, warnings, missing_service_vars, missing_project_vars, missing_secrets, missing_git_provider, project_env, deployed } — Resposta de `ServiceImport` (também no `dry_run`).

### source.rs
fn looks_like_git_url(url) -> bool — Heuristic used by the clients' General tab to decide whether a "Repository URL / Image" value denote…
enum ServiceSource { Registry, Git, Archive, Compose }
struct ArchiveSource { archive_id, original_filename, dockerfile_path, build_context }
struct ComposeSource { content, ingress_service }
struct GitSource { url, branch, root_path, watch_paths, submodules, dockerfile_path, build_context, build_stage, credentials, username, provider_id }
impl Default for ArchiveSource
impl Default for GitSource

### wizard.rs
struct WizardCreateReq { kind, id, project_id, name, app_name, db_name, user, password, root_password, image, use_replica, template_values, expose_external } — Campos coletados pelo wizard no cliente; o daemon (`WizardCreate`) resolve o `kind`/`id` e monta o `…

## rustploy-shared/src/protocol/

### command.rs — `Command`: tudo o que o cliente pede ao daemon.
enum Command { ProjectCreate, ProjectDelete, ProjectUpdate, ProjectList, ProjectEnvSet, ServiceCreate, ServiceUpdate, ServiceDelete, ServiceList, ServiceGet, DeployStart, DeployAbort, DeployRollback, DeployHistory, DeployDelete, ServiceStop, ServiceReload, RecentDeployments, GetBuildLogs, LogsGet, LogsSubscribe, LogsUnsubscribe, MetricsSubscribe, MetricsUnsubscribe, ServiceConnectionInfo, SharedAccessList, SharedAccessGrant, SharedAccessRevoke, ManagedDatabaseList, ManagedDatabaseCreate, ManagedDatabaseDelete, ManagedDatabaseListAll, MigrationStart, MigrationGet, MigrationList, MigrationRollback, MigrationDiscard, GetWebhookUrl, RegenerateWebhookToken, GetDaemonSettings, SetDaemonSettings, SecretSet, SecretDelete, SecretList, ManifestApply, ManifestExport, ManifestExportAll, ManifestImport, ServiceExportPlan, ServiceExport, ServiceImport, JobCreate, JobUpdate, JobDelete, JobList, JobListAll, JobRunNow, JobRunCancel, JobRunHistory, GetJobLogs, JobWarmStatus, JobWarm, PruneContainers, PruneVolumes, PruneImages, PruneBuildCache, PruneNetworks, DockerCleanupConfigGet, DockerCleanupConfigSet, DockerCleanupRunNow, DockerImages, DockerVolumes, DockerNetworks, DockerContainers, RemoveContainer, RemoveImage, RemoveVolume, RemoveNetwork, StopAllManaged, IngressRoutes, IngressReconcile, EnvBackupList, EnvBackupRestore, Ping, DaemonStatus, DeployEngineStatus, GitProviderList, GitProviderCreate, GitProviderDelete, GitOAuthStart, GitRepoList, GitBranchList, WizardCatalog, WizardCreate, Snapshot, RegistryStatus, RegistryRepoList, RegistryTagList, RegistryTagDelete, RegistryRepoDelete, RegistryGc, RegistryTokenCreate, RegistryTokenList, RegistryTokenRevoke, DeployQueuePromote, DeployQueueReorder, DeployQueuePause }
(2 testes)

### event.rs — `Event`: o que o SSE entrega ao cliente.
enum Event { DeployStateChanged, DeployProgress, BuildLog, LogLine, ContainerMetrics, SystemMetrics, ServiceStatusChanged, DaemonReady, Error, JobLogLine, JobRunStateChanged, DeployQueueChanged, DockerCleanupCompleted, JobWarmChanged }
impl Event
  fn matches(service_id) -> bool

### logs.rs — Linhas de log: stream de origem e entradas de container/build.
enum LogStream { Stdout, Stderr }
struct LogEntry { stream, line, timestamp }
struct BuildLogLine { stream, line, timestamp }

### mod.rs — Protocolo da API: `Command` (o que o cliente pede), `Response` e `Event` (o que o SSE entrega).

### response.rs — `Response`: a resposta a cada `Command`.
enum Response { Ok, Project, Projects, Service, Services, Deployment, Deployments, Logs, BuildLogs, DeploymentSummaries, DaemonStatus, DeployEngineStatus, Pong, WebhookUrl, SharedAccessList, Migration, Migrations, ManagedDatabases, ConnectionInfo, DaemonSettings, SecretNames, ManifestReport, Manifest, ManifestBundle, ServiceExportPlan, ServiceBundleYaml, ServiceImportReport, MissingEnvVars, GitProviders, GitProviderInfo, OAuthUrl, GitRepos, GitBranches, PruneResult, DockerCleanupConfig, EnvBackupSnapshots, DockerImages, DockerVolumes, DockerNetworks, DockerContainers, StopAllResult, IngressRoutes, WizardCatalog, Snapshot, Job, Jobs, JobSummaries, JobRun, JobRuns, JobLogs, JobWarmStatus, JobWarmRequested, RegistryStatus, RegistryRepos, RegistryTags, RegistryGcResult, RegistryTokenCreated, RegistryTokens, Err }
impl Response
  fn err(code, message) -> Self
