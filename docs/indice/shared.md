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

### audit.rs — Auditoria de valores em claro no estado de infra (plano `docs/plano-backup-estado-em-git.md`, R6).
struct PlaintextFinding { definite, project, scope, what, allowed } — Um valor que está (ou parece estar) em texto puro onde deveria estar cifrado.
struct InfraFile { path, content } — Um arquivo da infra tal como o `state_git` o grava no repositório (resposta de `Command::InfraDownlo…

### data_backup.rs — Backup dos dados dos serviços para um S3.
struct BackupDestination { endpoint, region, bucket, prefix, path_style, access_key, secret_key } — O destino S3 dos backups (um por servidor).
fn path_style_default() -> bool
struct BackupDestinationView { configured, endpoint, region, bucket, prefix, path_style, access_key, secret_set, recovery_configured } — O destino como o daemon o **devolve**: sem a chave secreta, só se ela existe.
struct BackupEntry { key, last_modified, size_bytes } — Um backup de um serviço, como está no S3.
struct BackupRun { op, state, started_at, finished_at, size_bytes, key, error, checks } — A última operação de backup ou restauração de um serviço (em andamento ou terminada).
struct BackupPolicy { enabled, recurrence, keep, stop } — A política de backup automático de **um serviço** (vive no YAML do serviço e vai para o git).
fn backup_keep_default() -> u32
fn backup_stop_default() -> bool
impl BackupPolicy
  fn validate() -> Result<(), String> — Confere os limites; devolve a causa em linguagem de gente.
struct ServiceBackups { entries, last_run, blocked, list_error, policy, next_run_at } — Resposta de `ServiceBackupList`.
struct RecoverService { service_id, name, project, kind, latest_backup, step, error } — Um serviço na tela de recuperação.
struct ServerRecoverInfo { phase, projects, services, destination_configured, destination_error, message, recovery_public_key } — Resposta dos comandos `ServerRecover*`: em que ponto a recuperação deste servidor está.
impl std::fmt::Debug for BackupDestination
(5 testes)

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
enum DeployBatchStatus { Queued, Skipped, Blocked, Failed } — O que aconteceu com um serviço num deploy em lote (`DeployWithDependencies`, `ProjectDeploy`).
struct DeployBatchItem { service_id, service_name, wave, status, deployment_id, message } — Um serviço do plano de um deploy em lote, na ordem em que entra na fila: as dependências vêm antes d…
enum DeployState { Pending, PreDeployCheck, ResolvingDeps, PullingImage, CloningRepo, BuildingImage, Staging, HealthcheckPolling, SwappingIn, Draining, Promoting, Live, Stopped, RollingBack, Failed, Pruning, ComposingUp }
impl DeployState
  fn is_terminal() -> bool
  fn label() -> &'static str
struct StateTransition { from, to, at, message }
struct DeploymentSummary { deployment, service_name, project_name }
struct DaemonStatus { version, uptime_secs, services_running, services_total }
enum QueueItemKind { Deploy, Job, Warm } — O que um item da fila global é: um deploy, a execução de um job avulso (Agendamentos) ou o aquecimen…
struct ActiveDeployInfo { deployment_id, service_id, service_name, project_name, state, started_at, elapsed_secs, current_state_secs, states, kind }
struct DeployEngineSummary { version, uptime_secs, active, recent, total_24h, successful_24h, failed_24h, queued, paused }
(8 testes)

### docker_inventory.rs
struct DockerImageInfo { id, tags, size_bytes, created, containers, project, service } — One Docker image (from `docker system df`), independent of whether it's currently used by any servic…
struct DockerVolumeInfo { name, driver, mountpoint, in_use, ref_count, size_bytes } — One Docker volume, independent of rustploy ownership (rustploy itself only ever bind-mounts host pat…
struct DockerContainerInfo { id, name, image, state, status, managed, project, service, removed_in_git }
struct DockerNetworkInfo { id, name, driver, scope, in_use, container_count, project } — One Docker network.
fn default_warm_after_cleanup() -> bool
fn default_cleanup_retention_hours() -> u32
struct DockerCleanupConfig { enabled, recurrence, containers, images, images_all, volumes, volumes_all, networks, build_cache, build_cache_older_than_hours, images_older_than_hours, warm_after_cleanup, last_run_at, next_run_at } — Configuração de limpeza automática de recursos Docker não usados (containers parados, imagens, volum…
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
(9 testes)

### metrics.rs
struct ContainerMetricsPoint { service_id, container_id, cpu_percent, mem_used_bytes, mem_limit_bytes, net_rx_bytes, net_tx_bytes, timestamp }
struct SystemMetricsPoint { cpu_percent, mem_used_bytes, mem_total_bytes, disk_used_bytes, disk_total_bytes, load_avg_1, load_avg_5, load_avg_15, timestamp }

### mod.rs — Modelos de domínio: projeto, serviço e suas fontes, deployment, jobs, bancos, registry e inventário …

### project.rs
struct Project { id, name, description, env_vars, env_comments, created_at }

### registry.rs
struct RegistryRepoInfo { name, tag_count, size_bytes, created_at } — Um repositório do registry embutido, para a lista da sub-aba Registry.
struct RegistryTagInfo { tag, digest, media_type, size_bytes, updated_at } — Uma tag de um repositório, com o manifest que ela aponta.
struct RegistryRetentionConfig { enabled, keep_last } — Retenção de tags do registry embutido: com ela ligada, antes de cada GC o daemon apaga as tags mais …
impl RegistryRetentionConfig
  fn effective_keep_last() -> u32 — `keep_last` zero apagaria tudo o que não está em uso: vale no mínimo 1.
struct RegistryStatusInfo { enabled, port, domain, repo_count, blob_count, storage_bytes, retention } — Status do registry embutido: config + agregados, para o cabeçalho da sub-aba Registry.
struct RegistryTokenInfo { name, scope, created_at, last_used_at } — Um token de acesso do registry (Basic auth) — o segredo em texto plano só aparece uma vez, em `Regis…
impl Default for RegistryRetentionConfig
(3 testes)

### service.rs
struct ServiceSpec { name, project_id, source, port, host_port, domain, tls_enabled, env_vars, env_comments, volumes, healthcheck, replicas, resources, run_command, run_args, product, domains, pre_deploy_job_id, pre_deploy_job_ids, shared, depends_on, autostart, registry_auto_deploy }
fn default_autostart() -> bool
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
struct ServiceWarmStatus { service_id, warm, warming, reason, warmed_at } — Se a imagem de um serviço com fonte Git ou zip já tem o cache de build quente no host, de modo que o…
impl Default for Healthcheck
impl std::fmt::Display for ServiceStatus
(3 testes)

### service_bundle.rs
struct PlanVar { key, is_secret, suggested } — Uma variável listada na tela de checkboxes do export.
struct ServiceExportPlan { service_name, project_name, service_env, project_env, blocked } — Resposta de `ServiceExportPlan`: o que dá para levar, para a tela marcar.
enum ProjectEnvChoice { Keep, Overwrite, ServiceOnly, Ignore } — O que fazer, no destino, com UMA variável de projeto trazida no pacote.
struct ServiceImportReq { yaml, project_id, name, drop_domains, git_provider_id, vars, project_vars, secrets, project_env, deploy, dry_run } — Pedido de `ServiceImport`.
struct ImportWarning { code, message }
enum ProjectEnvState { New, Same, Conflict }
struct ProjectEnvStatus { key, state, is_secret, choice }
struct ServiceImportReport { dry_run, service_id, service_name, project_name, name_conflict, warnings, missing_service_vars, missing_project_vars, missing_secrets, missing_git_provider, project_env, deployed } — Resposta de `ServiceImport` (também no `dry_run`).

### service_data.rs — Dados de um serviço (volumes) que podem ser exportados e importados.
struct DataPart { name, kind, size_bytes } — Uma parte do pacote de dados: hoje, um volume nomeado do serviço.
struct ServiceDataPlan { service_name, parts, running, blocked, notes, encrypt_default } — Resposta de `ServiceDataPlan`: o que um export levaria, sem mudar nada.

### source.rs
fn looks_like_git_url(url) -> bool — Heuristic used by the clients' General tab to decide whether a "Repository URL / Image" value denote…
enum ServiceSource { Registry, Git, Archive, Compose }
struct ArchiveSource { archive_id, original_filename, dockerfile_path, build_context }
struct ComposeSource { content, ingress_service }
struct GitSource { url, branch, root_path, watch_paths, submodules, dockerfile_path, build_context, build_stage, credentials, username, provider_id }
impl Default for ArchiveSource
impl Default for GitSource

### state_git.rs — Sincronização do estado de infra com um repositório git (plano `docs/plano-backup-estado-em-git.md`,…
struct StateGitCommit { sha, at, subject } — Um commit do repositório de estado.
struct StateGitRemoved { id, kind, name, project, sha, at } — Algo que sumiu do estado por um commit externo.
struct StateGitRejected { sha, reason } — Um commit do remoto que o daemon **recusou** aplicar (e por quê).
struct StateGitStatus { mode, remote, branch, provider_id, git_available, recovery_configured, last_commit, last_push_ok, last_push_at, last_error, blocked_by_plaintext, needs_push, last_fetch_at, ahead, behind, paused_reason, incoming_rejected, removed_in_git, pending_deploy } — Situação da sincronização, para a tela de Settings.

### update.rs — Verificação de versão: o que o daemon responde a `Command::CheckUpdate`.
struct UpdateInfo { current_version, latest_version, is_update_available, release_url, published_at, checked_at } — Resultado de comparar a versão do daemon com a última release publicada.
enum UpdateStage { Idle, Downloading, Installing, RollingBack, Succeeded, Failed, RolledBack } — Em que ponto está a atualização do daemon pedida por `Command::ApplyUpdate`.
struct UpdateProgress { stage, target_version, message, is_helper_available, finished_at } — Resposta de `Command::UpdateStatus`.

### wizard.rs
struct WizardCreateReq { kind, id, project_id, name, app_name, db_name, user, password, root_password, image, use_replica, template_values, expose_external } — Campos coletados pelo wizard no cliente; o daemon (`WizardCreate`) resolve o `kind`/`id` e monta o `…

## rustploy-shared/src/protocol/

### command.rs — `Command`: tudo o que o cliente pede ao daemon.
enum StateGitResolveChoice { UseRemote, UseLocal } — Como resolver um histórico divergido (`Command::StateGitResolve`).
enum SecretMoveOp { EnvToSecret, SecretToEnv, ServiceSecretToProject, ProjectSecretToService, EnvServiceToProject, EnvProjectToService, EnvJobToProject, JobSecretToProject } — As operações de [`Command::SecretMove`].
enum Command { ProjectCreate, ProjectDelete, ProjectUpdate, ProjectList, ProjectServicesReorder, ProjectEnvSet, ServiceCreate, ServiceUpdate, ServiceDelete, ServiceList, ServiceGet, DeployStart, DeployWithDependencies, ProjectDeploy, DeployWithDependenciesPlan, ProjectDeployPlan, DeployAbort, DeployRollback, DeployHistory, DeployDelete, ServiceStop, ServiceReload, RecentDeployments, GetBuildLogs, LogsGet, LogsHistory, LogsExecutions, LogsExecutionDelete, LogsSubscribe, LogsUnsubscribe, MetricsSubscribe, MetricsUnsubscribe, ServiceConnectionInfo, SharedAccessList, SharedAccessGrant, SharedAccessRevoke, ManagedDatabaseList, ManagedDatabaseCreate, ManagedDatabaseDelete, ManagedDatabaseListAll, MigrationStart, MigrationGet, MigrationList, MigrationRollback, MigrationDiscard, GetWebhookUrl, RegenerateWebhookToken, GetDaemonSettings, SetDaemonSettings, SecretSet, SecretDelete, SecretList, SecretReveal, InfraAudit, InfraDownload, StateGitStatus, StateGitConfigure, StateGitSyncNow, StateGitResume, StateGitResolve, StateGitDismissRemoval, InfraAuditAllow, StopContainer, SecretMove, EnvKeyRename, EnvValueSet, SecretRename, ServiceExportPlan, ServiceExport, ServiceDataPlan, BackupDestinationGet, BackupDestinationSet, BackupDestinationClear, BackupDestinationTest, ServiceBackupList, ServiceBackupNow, ServiceBackupRestore, ServiceBackupDelete, ServiceBackupPolicySet, ServerRecoverStart, ServerRecoverStatus, ServerRecoverRun, ServerRecoverCancel, ServiceImport, JobCreate, JobUpdate, JobDelete, JobList, JobListAll, JobRunNow, JobRunCancel, JobRunHistory, GetJobLogs, JobWarmStatus, JobWarm, ServiceWarmStatusList, ServiceWarm, PruneContainers, PruneVolumes, PruneImages, PruneBuildCache, PruneNetworks, DockerCleanupConfigGet, DockerCleanupConfigSet, DockerCleanupRunNow, DockerImages, DockerVolumes, DockerNetworks, DockerContainers, RemoveContainer, RemoveImage, RemoveVolume, RemoveNetwork, StopAllManaged, IngressRoutes, IngressReconcile, EnvBackupList, EnvBackupRestore, Ping, DaemonStatus, DeployEngineStatus, CheckUpdate, ApplyUpdate, UpdateStatus, UpdateRollback, GitProviderList, GitProviderCreate, GitProviderDelete, GitOAuthStart, GitRepoList, GitBranchList, WizardCatalog, WizardCreate, Snapshot, RegistryStatus, RegistryRepoList, RegistryTagList, RegistryTagDelete, RegistryRepoDelete, RegistryGc, RegistryRetentionSet, RegistryTokenCreate, RegistryTokenList, RegistryTokenRevoke, DeployQueuePromote, DeployQueueReorder, DeployQueuePause }
fn backup_stop_default() -> bool
(13 testes)

### event.rs — `Event`: o que o SSE entrega ao cliente.
enum Event { DeployStateChanged, DeployProgress, BuildLog, LogLine, ContainerMetrics, SystemMetrics, ServiceStatusChanged, DaemonReady, Error, JobLogLine, JobRunStateChanged, DeployQueueChanged, DockerCleanupCompleted, JobWarmChanged, ServiceWarmChanged, RegistryPush }
impl Event
  fn matches(service_id) -> bool

### logs.rs — Linhas de log: stream de origem e entradas de container/build.
enum LogStream { Stdout, Stderr }
struct LogEntry { stream, line, timestamp }
struct LogHistoryEntry { timestamp, container_id, stream, line } — Uma linha do log de runtime **persistido** (`LogsHistory`): além do texto, de qual container veio, p…
struct LogExecution { container_ids, deployment_id, image, started_at, ended_at, lines, bytes, current, partial, outcome } — Uma execução (um container) com log guardado: resposta de `LogsExecutions`.
struct BuildLogLine { stream, line, timestamp }

### mod.rs — Protocolo da API: `Command` (o que o cliente pede), `Response` e `Event` (o que o SSE entrega).

### response.rs — `Response`: a resposta a cada `Command`.
enum Response { Ok, Project, Projects, Service, Services, Deployment, Deployments, DeployBatch, Logs, LogsHistory, LogsExecutions, LogsExecutionsDeleted, BuildLogs, DeploymentSummaries, DaemonStatus, UpdateInfo, UpdateProgress, DeployEngineStatus, Pong, WebhookUrl, SharedAccessList, Migration, Migrations, ManagedDatabases, ConnectionInfo, DaemonSettings, SecretNames, StateGit, InfraAudit, InfraFiles, SecretValue, ServiceExportPlan, ServiceBundleYaml, ServiceDataPlan, BackupDestination, ServiceBackups, ServerRecover, ServiceImportReport, GitProviders, GitProviderInfo, OAuthUrl, GitRepos, GitBranches, PruneResult, DockerCleanupConfig, EnvBackupSnapshots, DockerImages, DockerVolumes, DockerNetworks, DockerContainers, StopAllResult, IngressRoutes, WizardCatalog, Snapshot, Job, Jobs, JobSummaries, JobRun, JobRuns, JobLogs, JobWarmStatus, JobWarmRequested, ServiceWarmStatuses, ServiceWarmRequested, RegistryStatus, RegistryRepos, RegistryTags, RegistryGcResult, RegistryTokenCreated, RegistryTokens, Err }
impl Response
  fn err(code, message) -> Self

## rustploy-shared/update-core/src/

### dpkg_version.rs — Comparação de versões de **pacote Debian** (`1:0.1.10-1`), pelo algoritmo da política Debian (o mesm…
struct DpkgVersionError { input }
struct Parts { epoch, upstream, revision }
fn split(version) -> Result<Parts<'_>, DpkgVersionError>
fn order(byte) -> i32 — Peso de um caractere que não é dígito: `~` vem antes de tudo (até do fim da string), letras vêm ante…
fn compare_part(left, right) -> Ordering
fn compare(left, right) -> Result<Ordering, DpkgVersionError> — Compara duas versões de pacote Debian (época, depois upstream, depois revisão).
fn is_newer(candidate, installed) -> Result<bool, DpkgVersionError> — `true` se `candidate` é **estritamente** mais nova que `installed`.
impl fmt::Display, std::error::Error for DpkgVersionError
(5 testes)

### installer.rs — A instalação de um pacote assinado, em duas fases:
struct Paths { allowed_dir, state_dir, managed_binaries } — Onde o updater lê e escreve.
impl Paths
  fn staging_dir() -> PathBuf
  fn backup_dir() -> PathBuf
  fn outcome_file() -> PathBuf
  fn lock_file() -> PathBuf
struct Settings { package, service, trusted_keys, max_deb_bytes, poll_interval, stable_polls, max_polls }
struct Updater { paths, settings, system }
struct RollbackPlan { _lock, previous_version } — Um rollback já aprovado.
struct Prepared { _lock, staged_deb, staged_signature, facts } — Um pacote já aprovado.
impl Updater<'_, S>
  fn prepare(deb, signature_path) -> Result<Prepared, String>
  fn acquire_lock() -> Result<fs::File, String>
  fn verify_and_check(deb, signature_path, known_version)
  fn installed_version() -> Result<Option<String>, String>
  fn execute(prepared) -> UpdateOutcome
  fn prepare_rollback() -> Result<RollbackPlan, String> — Fase 1 do rollback: só **recusa** (trava e backup existente), sem mexer em nada.
  fn execute_rollback(plan) -> UpdateOutcome — Fase 2: volta os binários para os do último backup (o único caminho que aceita uma versão menor), re…
  fn rollback() -> Result<UpdateOutcome, String> — As duas fases juntas (nos testes não há cliente esperando uma resposta no meio).
  fn finish(prepared, version, kind, message) -> UpdateOutcome
  fn record(version, kind, message) — Grava o resultado.
  fn run_checked(program, args, envs) -> Result<CommandOutput, String>
  fn backup_binaries() -> Result<(), String>
  fn restore_and_restart() -> Result<(), String>
  fn service_name() -> &str
  fn wait_until_stable() -> bool — `true` se o serviço ficou `active` por `stable_polls` checagens seguidas.
fn backup_file(backup_dir, binary) -> Option<PathBuf>
fn replace_atomically(source, target) -> io::Result<()> — Troca `target` por uma cópia de `source` sem nunca deixá-lo pela metade: copia para um nome ao lado …
fn restore_note(restored) -> String
fn tail(output) -> String — Últimas linhas do que o comando disse, para a mensagem que a pessoa vai ler.
const MAX_SIGNATURE_BYTES
(12 testes)

### lib.rs — Lógica de atualização comum ao daemon e ao app: um só lugar decide se uma versão é mais nova que out…

### outcome.rs — O resultado da última atualização, gravado pelo updater (root) e lido pelo daemon depois de reinicia…
enum OutcomeKind { InProgress, Installed, Failed, RolledBack }
struct UpdateOutcome { package, version, kind, message, at_unix }
impl UpdateOutcome
  fn new(package, version, kind, message) -> Self
fn write_outcome(path, outcome) -> io::Result<()> — Grava de forma atômica (arquivo temporário + `rename`) e legível por todos: o daemon (usuário `rustp…
fn read_outcome(path) -> io::Result<Option<UpdateOutcome>> — `None` se ainda não houve nenhuma atualização; erro se o arquivo existir e estiver ilegível.
(3 testes)

### package.rs — A decisão "este pacote assinado pode ser instalado aqui?", como função pura: cruza o que a **assinat…
struct PackageFacts { package, version, architecture } — Campos lidos do `.deb` com `dpkg-deb -f <arquivo> Package Version Architecture`.
struct Expectations { package, machine_architecture, installed_version } — O que este updater aceita instalar.
enum PackageError { WrongPackage, ClaimsMismatch, WrongArchitecture, NotNewer, BadVersion, UnreadableFields }
fn parse_dpkg_fields(output) -> Result<PackageFacts, PackageError> — Lê a saída de `dpkg-deb -f <arquivo> Package Version Architecture` (linhas `Campo: valor`, em qualqu…
fn check_package(claims, facts, expectations) -> Result<(), PackageError> — Recusa se qualquer regra falhar.
impl fmt::Display, std::error::Error for PackageError
(7 testes)

### release.rs — Os arquivos de uma release do GitHub e a escolha do pacote certo para esta máquina.
struct ReleaseAsset { name, url } — Um arquivo anexado à release (`rustployd_0.1.10-1_amd64.deb`, o `.minisig`…).
struct ReleaseInfo { tag, html_url, assets } — O que interessa do JSON de uma release.
struct ReleasePayload { tag_name, html_url, assets }
struct AssetPayload { name, browser_download_url }
fn parse_release(json) -> Result<ReleaseInfo, String> — Lê o JSON de `GET /repos/{repo}/releases/latest`.
fn debian_architecture() -> Result<&'static str, String> — Arquitetura no vocabulário do Debian, a que aparece no nome do `.deb`.
fn debian_architecture_of(rust_architecture) -> Result<&'static str, String>
fn select_package_assets(assets, repo, package, architecture, tag) — O `.deb` de `package` para `architecture` e a `.minisig` dele, **dentro do repositório dado** (uma U…
struct PackageVersion { full, upstream } — A versão de um pacote, como a release a anuncia no nome do arquivo.
fn package_version(assets, repo, package, architecture) -> Option<PackageVersion> — A versão do `.deb` de `package` para `architecture` que a release traz, lida do **nome do arquivo** …
(10 testes)

### signature.rs — Verificação da assinatura minisign de um pacote e leitura do comentário confiável.
const TRUSTED_PUBLIC_KEYS — Chaves públicas em que os updaters confiam; qualquer uma delas basta.
struct SignedClaims { package, version, arch } — O que a assinatura afirma sobre o arquivo assinado.
enum SignatureError { NoTrustedKeys, BadTrustedKey, MalformedSignature, NotTrusted, MissingClaims }
fn verify(data, signature_text, public_keys) -> Result<SignedClaims, SignatureError> — Confere `data` contra `signature_text` (o conteúdo do `.minisig`) usando as chaves dadas e devolve o…
fn parse_claims(trusted_comment) -> Result<SignedClaims, SignatureError>
impl fmt::Display, std::error::Error for SignatureError
(10 testes)

### staging.rs — Cópia segura do pacote para um diretório que só o root controla.
enum StagingError { OutsideAllowedDir, BadFileName, NotRegularFile, TooLarge, Io }
fn is_plain_file_name(name) -> bool
fn stage_file(source, allowed_dir, staging_dir, max_bytes) -> Result<PathBuf, StagingError> — Copia `source` para `staging_dir` (criado `0700`, cópia `0600`) e devolve o caminho da cópia.
impl fmt::Display, std::error::Error, From<io::Error> for StagingError
(9 testes)

### system.rs — Tudo que o updater pede ao sistema (rodar comando, esperar) passa por aqui, para os testes trocarem …
struct CommandOutput { success, stdout, stderr }
trait System
  fn run(program, args, envs) -> io::Result<CommandOutput> — Roda `program` com `args`, num ambiente limpo (só `PATH`, `LANG` e `envs`).
  fn sleep(duration)
struct RealSystem
impl System for RealSystem: run, sleep

### version.rs — Comparação de versões de release (semver), tolerante ao `v` das tags.
struct VersionError { input } — A entrada não é uma versão semver (depois de tirar espaços e o `v`/`V` inicial).
fn parse_version(text) -> Result<Version, VersionError> — Lê `0.1.15`, `v0.1.15` ou `V0.1.15` (com ou sem espaços ao redor).
fn is_newer(candidate, installed) -> Result<bool, VersionError> — `true` se `candidate` é **estritamente** mais nova que `installed`.
impl fmt::Display, std::error::Error for VersionError
(7 testes)
