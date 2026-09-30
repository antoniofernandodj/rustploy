# Índice: crates/shared — modelos, protocolo, manifest, templates

> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## crates/shared/

### build.rs — Gera, em tempo de compilação, o catálogo estático de templates a partir dos blueprints escritos à mã…
struct RawTemplate { variables, config }
struct RawConfig { env, domains, mounts }
struct RawDomain { service_name, port, host, path }
fn main()
fn read_meta(dir, id) -> (String, String) — `meta-entry.json` (só alguns blueprints têm) traz description/logo melhores.
fn json_string_field(s, field) -> Option<String>
fn find_logo(dir) -> Option<String>
fn prettify(id) -> String — `gitea_mysql` → "Gitea Mysql"; `open-webui` → "Open Webui".
fn normalize_env(env) -> Vec<(String, String)> — Normaliza `config.env` (array `"K=V"` ou tabela) para pares `(K, V)`.
fn lit(s) -> String — Literal Rust `&str` válido (escapado) para `s`.

## crates/shared/src/

### config.rs — Configuração do daemon (`config.toml`): structs de cada seção com defaults e o singleton `CONFIG`.
const CONFIG — Process-wide configuration singleton.
struct EnvBackupConfig { dir, interval_secs }
fn default_env_backup_interval() -> u64
struct RustployConfig { daemon, ingress, docker, deploy, metrics, secrets, api, env_backup, external_ports, registry }
struct ExternalPortsConfig { range_start, range_end } — Faixa de portas de host que o rustploy pode alocar automaticamente para exposição externa de serviço…
fn default_external_range_start() -> u16
fn default_external_range_end() -> u16
impl ExternalPortsConfig
  fn contains(port) -> bool
struct ApiConfig { enabled, bind_address, port, token, domain, max_connections } — Configuration for the HTTP/JSON + SSE control API — o canal administrativo remoto.
fn default_api_enabled() -> bool
fn default_api_bind() -> String
fn default_api_port() -> u16
fn default_api_max_connections() -> usize
impl ApiConfig
  fn is_public_bind() -> bool — True when the configured bind address is not a loopback address.
  fn public_base_url(fallback_host) -> String — URL pública desta API, sem barra final — a base das URLs de webhook (`{base}/webhook/{service_id}/{t…
struct RegistryConfig { enabled, port, domain, storage_dir } — Registry Docker OCI (Distribution API v2) embutido no daemon.
fn default_registry_port() -> u16
struct DaemonConfig { db_path, log_level }
struct IngressConfig { http_port, https_port, bind_address, acme }
struct AcmeConfig { enabled, email, directory }
struct DockerConfig { socket_path }
struct DeployConfig { drain_secs, image_cache }
struct MetricsConfig { interval_secs, history_points }
struct SecretsConfig { master_key_path }
impl RustployConfig
  fn global() -> &'static RustployConfig — Returns the process-wide config, loading it on first call.
  fn load() -> Self
  fn apply_env_overrides(cfg) -> Self
fn user_home() -> Option<String> — The single place the process reads `$HOME`.
fn fallback_data_dir() -> PathBuf — `~/.local/share/rustploy`, falling back to `/tmp` when `$HOME` is unset.
fn dirs_config_path() -> Option<PathBuf>
impl Default for ApiConfig
impl Default for EnvBackupConfig
impl Default for ExternalPortsConfig
impl Default for RegistryConfig
impl Default for RustployConfig
(2 testes)

### connection.rs — Connection string de banco/broker, montada num lugar só.
fn pct_encode(s) -> String — Percent-encoding de um componente de userinfo (`@`, `:`, `/`, `?`, `#`…).
fn scheme(db_kind) -> Option<&'static str> — Esquema de URI por `db_kind`; `None` = sem esquema (Kafka, serviço comum).
struct ConnTarget { host, port, database, user, password, auth_source } — Destino de uma conexão.
fn non_empty(s) -> Option<&str>
fn connection_url(db_kind, t) -> String — URI padrão (`scheme://user:senha@host:porta/database`) que os drivers aceitam como `DATABASE_URL`.
fn credentials(db_kind, get) — `(database, usuário, senha, authSource)` de um banco/broker, lidos das env vars nas convenções que o…
fn compose_services(content) -> Vec<(String, Option<String>)> — Serviços declarados num YAML de Compose: `(chave, imagem)`, na ordem.
fn compose_host(content, ingress_service, db_kind) -> Option<String> — Host (chave do serviço no YAML) que recebe a conexão numa stack Compose: `ingress_service` se preenc…
fn is_sql_or_mongo(db_kind) -> bool — Bancos que podem ser servidor compartilhado / ter databases gerenciados.
fn validate_shared_rules(name, source, db_kind, shared) -> Result<(), String> — Regras do spec ligadas ao banco compartilhado, checadas onde o spec é gravado: (1) nenhum nome de se…
(6 testes)

### lib.rs — Tipos compartilhados entre daemon e GUI (modelos, protocolo, config, manifest, templates) e os nomes…
fn compose_project_name(svc_id, svc_name) -> String — Nome de stack Compose no formato **legado**, derivado a cada uso: primeiros 8 caracteres do ID (time…
fn new_compose_project_name(svc_id, svc_name) -> String — Nome de stack Compose de um serviço **novo**: `rp_<últimos 8 chars do ID>_<nome>` (a parte aleatória…
fn new_compose_project_name_long(svc_id, svc_name) -> String — Variante com o ID inteiro, para quando o índice `UNIQUE` recusa a curta.
fn new_project_network_name(project_id) -> String — Nome de rede Docker de um projeto **novo**: `rp_net_<ID inteiro, minúsculo>`.
fn compose_safe(name) -> String — Nome de serviço reduzido ao que o Docker Compose aceita num nome de projeto (ASCII minúsculo, dígito…
fn app_container_base(svc_id, svc_name) -> String — Base do nome de container de um serviço Application: `rp_<id8>_<safe>`.
fn app_network_alias(svc_name) -> String — Hostname de um serviço Application **dentro da rede do projeto**: `rp_<safe>`.
fn shared_alias(compose_project) -> String — Alias DNS **global** de um servidor de banco compartilhado, o único nome dele que atravessa a rede d…
const SHARED_ALIAS_PREFIX — Prefixo reservado: nenhum serviço/chave de Compose pode começar com ele.
(8 testes)

### manifest.rs — Infra-as-Code: structs do manifesto declarativo (`rustploy.yml`).
struct ServerManifest { api_version, projects } — Manifesto raiz (agregador): vários projetos, inline ou via `include:`.
impl ServerManifest
  fn from_existing_redacted(items, providers) -> (Self, EnvDoc) — Constrói um manifesto raiz redigido a partir de TODOS os projetos do banco (para `Command::ManifestE…
struct EnvDoc { project, git_provider } — Documento TOML que acompanha o manifesto no export/import de Infra-as-Code: variáveis de ambiente **…
struct GitProviderDoc { kind, base_url, auth_mode, oauth_client_id } — Dados não-secretos de um Git provider conectado (ver [`EnvDoc`]).
struct ProjectEnvDoc { env, service }
struct ServiceEnvDoc { env }
enum ProjectEntry { Include, Inline } — Uma entrada do manifesto raiz: projeto inline OU referência a um arquivo.
struct ProjectManifest { api_version, project, services } — Manifesto de um único projeto (`project:` + `services:`).
struct ProjectMeta { name, description, env }
struct ServiceManifest { name, source, port, host_port, domain, tls, env, volumes, healthcheck, replicas, resources, command, args, db, shared }
struct SourceManifest { registry, git, compose, compose_ingress } — Origem do serviço: exatamente uma das três chaves deve estar presente.
struct GitManifest { url, branch, root_path, dockerfile, context, build_stage, submodules, watch_paths, username, credentials, provider }
struct HealthcheckManifest { kind, path, status, interval, timeout, retries, start_period }
struct ResourcesManifest { cpu_shares, mem }
struct ApplyReport { actions, deployed } — Resultado de um `apply`: o que foi criado/atualizado/removido em cada recurso, mais a lista de servi…
struct ResourceAction { kind, name, action }
enum ResourceActionKind { Project, Service }
enum ActionVerb { Created, Updated, Unchanged, Deleted }
impl ProjectManifest
  fn project_fields() -> (String, Option<String>, Vec<EnvVar>) — Campos do projeto: `(name, description, env_vars)`.
  fn service_specs(project_id, provider_ids) -> Vec<ServiceSpec> — Converte cada serviço do manifesto numa [`ServiceSpec`] já vinculada ao projeto.
  fn interpolate(env) -> Vec<String> — Substitui `${VAR}` em todos os valores de env (projeto + serviços) resolvendo contra o [`EnvDoc`] **…
  fn from_existing(project, services, providers) -> Self — Reconstrói um manifesto a partir do estado atual no banco (para `export`).
  fn from_existing_redacted(project, services, providers, doc) -> Self — Como [`from_existing`](Self::from_existing), mas redige todo valor de env var `Plain` para `${KEY}` …
impl ServiceManifest
  fn to_spec(project_id, provider_ids) -> ServiceSpec
  fn from_spec(svc, providers) -> Self
impl SourceManifest
  fn to_source(provider_ids) -> ServiceSource
  fn from_source(src, providers) -> Self
impl HealthcheckManifest
  fn to_healthcheck() -> Healthcheck
  fn from_healthcheck(hc) -> Option<Self>
impl ResourcesManifest
  fn to_limits() -> ResourceLimits
  fn from_limits(limits) -> Option<Self>
impl GitProviderDoc
  fn from_provider(p) -> Self
fn env_map_to_vars(map) -> Vec<EnvVar>
fn env_vars_to_map(vars) -> BTreeMap<String, String>
fn redact_env_map(map, dotenv_out) — Redige, in-place, todo valor `Plain` (isto é, que não começa com `secret:`) de `map` para `${KEY}`, …
fn format_env_doc(doc) -> String — Serializa um [`EnvDoc`] como texto TOML (o arquivo de variáveis que acompanha o manifesto no export …
fn parse_env_doc(text) -> Result<EnvDoc, String> — Faz o parse do texto TOML do arquivo de variáveis num [`EnvDoc`].
fn format_dotenv(map) -> String — Serializa um mapa `KEY -> VALUE` como texto `.env` (uma linha por var, ordenado por chave).
fn parse_dotenv(text) -> BTreeMap<String, String> — Parser simples de texto `.env`: linhas `KEY=VALUE`, ignora vazias e `# comentário`.
fn interpolate_map_scoped(map, lookup, project, service, missing) — Interpola um mapa de env de um escopo (projeto ou serviço), rotulando as vars não resolvidas com o e…
fn interpolate_str(input, lookup, missing) -> String — Substitui ocorrências de `${VAR}` em `input`.
fn parse_volume(s) -> Option<VolumeMount> — `host:container` ou `host:container:ro`.
fn format_volume(v) -> String
fn parse_mem(s) -> Option<u64> — Aceita `1024`, `256k`, `256m`, `2g` (case-insensitive).
fn humanize_mem(bytes) -> String
fn default_hc_type() -> String
fn one() -> u32
fn is_one(n) -> bool
fn is_false(b) -> bool
fn is_zero_u64(n) -> bool
const API_VERSION, SECRET_PREFIX
impl std::fmt::Display for ActionVerb
impl std::fmt::Display for ResourceActionKind
(11 testes)

### models.rs — Modelos de domínio: projeto, `ServiceSpec` e suas fontes, deployment e estados, jobs, healthcheck, m…
struct Project { id, name, description, env_vars, env_comments, created_at }
struct ServiceSpec { name, project_id, source, port, host_port, domain, tls_enabled, env_vars, env_comments, volumes, healthcheck, replicas, resources, run_command, run_args, db_kind, domains, pre_deploy_job_id, pre_deploy_job_ids, shared }
struct SharedServerConfig {  } — Configuração de um servidor de banco compartilhado.
struct ManagedDatabase { id, server_service_id, project_id, name, env_var, connection_limit, statement_timeout_ms, connection_url, created_at } — Database (e usuário dedicado) criado para um projeto dentro de um servidor compartilhado.
struct ManagedDatabaseCreateReq { server_service_id, project_id, name, env_var, overwrite_env, skip_env, connection_limit, statement_timeout_ms } — Pedido de novo database gerenciado (`Command::ManagedDatabaseCreate`).
struct SharedAccess { server_service_id, project_id, network, alias } — Projeto autorizado a alcançar um servidor compartilhado.
struct DomainRoute { domain, port, tls } — Uma rota HTTP de domínio de um serviço: qual domínio, para qual porta do container e com ou sem TLS.
impl DomainRoute
  fn container_port(default) -> u16 — Porta de container efetiva (a própria, ou a `port` padrão do serviço).
fn normalize_name(name) -> String
impl Service
  fn compose_project_name() -> String — Nome da stack Compose: o gravado; na falta (banco ainda não migrado, ou serviço que virou Compose de…
impl ServiceSpec
  fn safe_name() -> String
  fn domain_routes() -> Vec<DomainRoute> — Rotas HTTP efetivas do serviço: a lista `domains` nova, ou — para specs antigos que só têm o campo `…
  fn ingress_container_ports() -> Vec<u16> — Portas de container que o ingress precisa alcançar: a de cada domínio, mais a `port` do serviço quan…
  fn materialize_domains() — Move o domínio legado (`domain`/`tls_enabled`) para a lista `domains` e zera os campos legados, para…
  fn pre_deploy_checks() -> Vec<String> — Fila efetiva de checks de pré-deploy: `pre_deploy_job_ids` quando não vazia, ou — para specs antigos…
  fn materialize_pre_deploy_checks() — Move o `pre_deploy_job_id` legado para `pre_deploy_job_ids` e zera o campo legado — mesmo idioma de …
fn resolve_env_vars(project, service) — Resolve as variáveis de ambiente para um serviço, combinando as do projeto e as do serviço.
fn looks_like_git_url(url) -> bool — Heuristic used by the clients' General tab to decide whether a "Repository URL / Image" value denote…
enum ServiceSource { Registry, Git, Archive, Compose }
struct ArchiveSource { archive_id, original_filename, dockerfile_path, build_context }
struct ComposeSource { content, ingress_service }
struct GitSource { url, branch, root_path, watch_paths, submodules, dockerfile_path, build_context, build_stage, credentials, username, provider_id }
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
struct Service { id, spec, compose_project, status, live_container_id, created_at, updated_at }
enum ServiceStatus { Stopped, Stopping, Deploying, Running, Degraded, Error, Queued }
struct Deployment { id, service_id, image, state, states_log, started_at, finished_at }
enum DeployState { Pending, PreDeployCheck, ResolvingDeps, PullingImage, CloningRepo, BuildingImage, Staging, HealthcheckPolling, SwappingIn, Draining, Promoting, Live, Stopped, RollingBack, Failed, Pruning, ComposingUp }
impl DeployState
  fn is_terminal() -> bool
  fn to_percent() -> u8
  fn label() -> &'static str
struct StateTransition { from, to, at, message }
struct EnvVar { key, value }
struct EnvComment { text, before_key } — Uma linha de comentário do editor `.env`, ancorada à var que a segue (por `key`, não por posição) — …
enum EnvVarValue { Plain, Secret }
struct VolumeMount { host_path, container_path, read_only }
struct Healthcheck { kind, interval_secs, timeout_secs, retries, start_period_secs }
enum HealthcheckKind { None, Http, Tcp, DockerNative }
struct DeploymentSummary { deployment, service_name, project_name }
struct ResourceLimits { cpu_shares, mem_limit_bytes }
struct DaemonStatus { version, uptime_secs, services_running, services_total }
struct ActiveDeployInfo { deployment_id, service_id, service_name, project_name, state, percent, started_at, elapsed_secs, current_state_secs }
struct DeployEngineSummary { version, uptime_secs, active, recent, total_24h, successful_24h, failed_24h, queued, paused }
struct ContainerMetricsPoint { service_id, container_id, cpu_percent, mem_used_bytes, mem_limit_bytes, net_rx_bytes, net_tx_bytes, timestamp }
struct SystemMetricsPoint { cpu_percent, mem_used_bytes, mem_total_bytes, disk_used_bytes, disk_total_bytes, load_avg_1, load_avg_5, load_avg_15, timestamp }
struct DockerImageInfo { id, tags, size_bytes, created, containers, project, service } — One Docker image (from `docker system df`), independent of whether it's currently used by any servic…
struct DockerVolumeInfo { name, driver, mountpoint, in_use, ref_count, size_bytes } — One Docker volume, independent of rustploy ownership (rustploy itself only ever bind-mounts host pat…
struct IngressDomainRoute { domain, backends, service_id } — One Docker container on the host (running or stopped), for the Docker tab's Containers sub-tab.
struct IngressPortRoute { host_port, backends } — Uma rota de porta TCP viva (`host_port` → backends).
struct IngressSnapshot { domains, ports } — Foto da tabela de rotas do ingress (resposta de `Command::IngressRoutes`).
struct DockerContainerInfo { id, name, image, state, status, managed, project, service }
struct DockerNetworkInfo { id, name, driver, scope, in_use, container_count, project } — One Docker network.
struct RegistryRepoInfo { name, tag_count, size_bytes, created_at } — Um repositório do registry embutido, para a lista da sub-aba Registry.
struct RegistryTagInfo { tag, digest, media_type, size_bytes, updated_at } — Uma tag de um repositório, com o manifest que ela aponta.
struct RegistryStatusInfo { enabled, port, domain, repo_count, blob_count, storage_bytes } — Status do registry embutido: config + agregados, para o cabeçalho da sub-aba Registry.
struct RegistryTokenInfo { name, scope, created_at, last_used_at } — Um token de acesso do registry (Basic auth) — o segredo em texto plano só aparece uma vez, em `Regis…
struct Job { id, project_id, trigger_service_id, name, compose, git_source, main_service, env_vars, env_comments, enabled, recurrence, last_run_at, next_run_at, created_at } — Uma tarefa one-shot: sobe um stack docker-compose próprio (efêmero — roda até terminar e é removido,…
struct JobGitSource { provider_id, url, branch, username, credentials, compose_path } — Fonte git de um `Job` — irmã mais enxuta de `GitSource` (sem `dockerfile_path`/`build_context`/`buil…
enum Recurrence { IntervalHours, Daily, Weekly } — Recorrência estruturada (sem expressão cron — ver decisão em docs/planejamento; `chrono` já é depend…
impl Recurrence
  fn next_after(from) -> DateTime<Utc> — Próximo disparo estritamente depois de `from`.
  fn at_time(from, hour, minute) -> DateTime<Utc> — `from`'s calendar date at the given UTC hour:minute (seconds zeroed).
struct JobRun { id, job_id, started_at, finished_at, exit_code, success } — Uma execução de um `Job`.
struct JobSummary { job, project_name, trigger_service_name, last_run } — `Job` + nomes resolvidos, pra listagem cross-project (sidebar Schedules) — mesmo espírito de `Deploy…
struct DockerCleanupConfig { enabled, recurrence, containers, images, images_all, volumes, volumes_all, networks, build_cache, last_run_at, next_run_at } — Configuração de limpeza automática de recursos Docker não usados (containers parados, imagens, volum…
impl DockerCleanupConfig
  fn any_resource_enabled() -> bool
  fn recompute_next_run(now) — Recalcula `next_run_at` a partir de `last_run_at` (ou de `now` quando nunca rodou) — chamado ao salv…
struct DockerCleanupResourceResult { resource, count, reclaimed_bytes, error } — Resultado da limpeza de UM recurso, dentro de uma execução — usado tanto no resumo persistido (`Dock…
struct DockerCleanupLastRun { at, results } — Resumo persistido da última execução (chave `KEY_DOCKER_CLEANUP_LAST_RUN` em `daemon_settings`), pra…
impl Default for ArchiveSource
impl Default for DockerCleanupConfig
impl Default for GitSource
impl Default for Healthcheck
impl Default for JobGitSource
impl std::fmt::Display for ServiceStatus
(21 testes)

### protocol.rs — Protocolo da API: `Command` (o que o cliente pede), `Response` e `Event` (o que o SSE entrega).
enum Command { ProjectCreate, ProjectDelete, ProjectUpdate, ProjectList, ProjectEnvSet, ServiceCreate, ServiceUpdate, ServiceDelete, ServiceList, ServiceGet, DeployStart, DeployAbort, DeployRollback, DeployHistory, DeployDelete, ServiceStop, ServiceReload, RecentDeployments, GetBuildLogs, LogsGet, LogsSubscribe, LogsUnsubscribe, MetricsSubscribe, MetricsUnsubscribe, ServiceConnectionInfo, SharedAccessList, SharedAccessGrant, SharedAccessRevoke, ManagedDatabaseList, ManagedDatabaseCreate, ManagedDatabaseDelete, GetWebhookUrl, RegenerateWebhookToken, GetDaemonSettings, SetDaemonSettings, SecretSet, SecretDelete, SecretList, ManifestApply, ManifestExport, ManifestExportAll, ManifestImport, JobCreate, JobUpdate, JobDelete, JobList, JobListAll, JobRunNow, JobRunCancel, JobRunHistory, GetJobLogs, PruneContainers, PruneVolumes, PruneImages, PruneBuildCache, PruneNetworks, DockerCleanupConfigGet, DockerCleanupConfigSet, DockerCleanupRunNow, DockerImages, DockerVolumes, DockerNetworks, DockerContainers, RemoveContainer, RemoveImage, RemoveVolume, RemoveNetwork, StopAllManaged, IngressRoutes, IngressReconcile, EnvBackupList, EnvBackupRestore, Ping, DaemonStatus, DeployEngineStatus, GitProviderList, GitProviderCreate, GitProviderDelete, GitOAuthStart, GitRepoList, GitBranchList, WizardCatalog, WizardCreate, Snapshot, RegistryStatus, RegistryRepoList, RegistryTagList, RegistryTagDelete, RegistryRepoDelete, RegistryGc, RegistryTokenCreate, RegistryTokenList, RegistryTokenRevoke, DeployQueuePromote, DeployQueueReorder, DeployQueuePause }
enum Event { DeployStateChanged, DeployProgress, BuildLog, LogLine, ContainerMetrics, SystemMetrics, ServiceStatusChanged, DaemonReady, Error, JobLogLine, JobRunStateChanged, DeployQueueChanged, DockerCleanupCompleted }
impl Event
  fn matches(service_id) -> bool
enum LogStream { Stdout, Stderr }
struct LogEntry { stream, line, timestamp }
struct BuildLogLine { stream, line, timestamp }
enum Response { Ok, Project, Projects, Service, Services, Deployment, Deployments, Logs, BuildLogs, DeploymentSummaries, DaemonStatus, DeployEngineStatus, Pong, WebhookUrl, SharedAccessList, ManagedDatabases, ConnectionInfo, DaemonSettings, SecretNames, ManifestReport, Manifest, ManifestBundle, MissingEnvVars, GitProviders, GitProviderInfo, OAuthUrl, GitRepos, GitBranches, PruneResult, DockerCleanupConfig, EnvBackupSnapshots, DockerImages, DockerVolumes, DockerNetworks, DockerContainers, StopAllResult, IngressRoutes, WizardCatalog, Snapshot, Job, Jobs, JobSummaries, JobRun, JobRuns, JobLogs, RegistryStatus, RegistryRepos, RegistryTags, RegistryGcResult, RegistryTokenCreated, RegistryTokens, Err }
impl Response
  fn err(code, message) -> Self

## crates/shared/src/templates/

### mod.rs — Catálogo de templates de aplicações (formato Dokploy), lido dos blueprints em `templates/blueprints/…
struct Template { id, name, description, logo, default_port, compose, variables, env, domains, mounts }
struct Var { key, raw }
struct Kv { key, raw }
struct Domain { service_name, port, host, path }
struct Mount { file_path, content }
fn all() -> &'static [Template]
fn find(id) -> Option<&'static Template>
fn filtered(search) -> Vec<&'static Template> — Templates cujo nome/descrição/id batem com o termo de busca, ordenados por nome.
fn editable_vars(t) -> Vec<&'static Var> — Variáveis que o usuário edita no wizard: as que são um domínio (`${domain}`).
struct Rendered { compose, env, domain, port, mounts }
fn render(t, user) -> Rendered — Resolve as variáveis (usando `user` para as editáveis) e devolve o compose + env + domínio prontos.
fn resolve_vars(t, user, rng) -> BTreeMap<String, String> — Resolve o mapa `variable -> valor`, semeando com os valores do usuário.
fn try_resolve(raw, resolved, rng) -> Option<String> — Tenta resolver um valor bruto; devolve `None` se depender de uma referência ainda não resolvida (par…
fn single_token(s) -> Option<&str> — `${x}` (e só isso, sem texto ao redor) → `Some("x")`.
fn all_refs_available(s, resolved) -> bool — Toda referência `${k}` de `s` a uma variável (não-gerador) já está resolvida? Geradores que apontam …
enum Unknown { Drop, Keep } — O que fazer com um `${...}` que não é variável resolvida nem gerador.
fn substitute(s, resolved, rng) -> String — Substitui todos os `${...}` de `s`: variável resolvida → valor; gerador → valor gerado; desconhecido…
fn substitute_mode(s, resolved, rng, unknown) -> String
fn tokens(s) -> impl Iterator<Item = &str> — Itera os miolos de todos os `${...}` em `s`.
enum Gen { Domain, Password, Base64, Jwt, Hash, Email, Username, Uuid, Timestamp, Timestamps, Timezone, JwtSigned }
fn parse_gen(inner) -> Option<Gen<'_>> — Interpreta um miolo de token (`"password:32"`, `"domain"`, …) como gerador.
impl Gen<'_>
  fn refs_available(resolved) -> bool — As variáveis referenciadas pelo gerador já estão resolvidas? Só `${jwt:<segredo>:<payload>}` referen…
  fn generate(rng, resolved) -> String
fn jwt_hs256(secret, payload_json) -> String — JWS compacto `header.payload.assinatura` em HS256.
struct Rng { state }
impl Rng
  fn new() -> Self
  fn next_u64() -> u64
  fn bytes(n) -> Vec<u8>
  fn hex(n) -> String
  fn uuid_v4() -> String — UUID v4 (bits de versão/variante fixados) — `POOLER_TENANT_ID` e afins.
  fn password(n) -> String
fn base64(data) -> String — Base64 padrão (com padding), sem dependência externa.
fn base64_url(data) -> String — Base64url **sem** padding (RFC 4648 §5) — o alfabeto que o JWS exige.
fn base64_with(data, table, pad) -> String
impl std::fmt::Debug for Template
(6 testes)

## crates/shared/src/

### wizard.rs — Lógica do wizard "Novo serviço" (Application / Database / Broker / Compose /Template): catálogos, ge…
enum DbKind { MongoDb, Postgres, MariaDb, MySql, Redis }
impl DbKind
  fn label() -> &'static str
  fn default_image() -> &'static str
  fn default_port() -> u16
  fn kind_id() -> &'static str
  fn from_str(s) -> Option<Self>
  fn default_user() -> &'static str
  fn has_db_name() -> bool
  fn has_user() -> bool
  fn has_root_password() -> bool
  fn has_replica_sets() -> bool
enum BrokerKind { Kafka, RabbitMq, Nats }
impl BrokerKind
  fn label() -> &'static str
  fn default_image() -> &'static str
  fn default_port() -> u16
  fn kind_id() -> &'static str
  fn from_str(s) -> Option<Self>
  fn has_user() -> bool
  fn default_user() -> &'static str
fn token_urlsafe(n) -> String — Senha aleatória URL-safe (splitmix64 semeado no relógio) — suficiente para credenciais iniciais de u…
fn db_rows_json() -> String — Passo "escolha o banco": inclui os flags de formulário (has_*) e o user default para o cliente pré-p…
fn broker_rows_json() -> String — Passo "escolha o broker".
fn templates_catalog_json(search) -> String — Templates filtrados por `search`, com as variáveis editáveis embutidas em cada linha (`vars: [{idx,l…
fn template_logo(t) -> (String, &'static str)
fn template_slug(t) -> String
struct DbFormInput { db_name, user, password, root_password, image, use_replica_sets } — Campos do formulário de banco/broker.
fn base_spec(name, project_id, source, port, env_vars, db_kind) -> ServiceSpec
fn app_spec(name, project_id) -> ServiceSpec
fn compose_spec(name, project_id) -> ServiceSpec
fn db_spec(db, name, project_id, f) -> ServiceSpec
fn db_env_vars(db, f) -> Vec<EnvVar>
fn db_compose(db, svc, img, f) -> String
fn broker_spec(broker, name, project_id, f) -> ServiceSpec
fn broker_env_vars(broker, f) -> Vec<EnvVar>
fn broker_compose(broker, svc, img, _f) -> String
fn template_spec(t, name, project_id, values) -> ServiceSpec
struct WizardCreateReq { kind, id, project_id, name, app_name, db_name, user, password, root_password, image, use_replica, template_values, expose_external } — Campos coletados pelo wizard no cliente; o daemon (`WizardCreate`) resolve o `kind`/`id` e monta o `…
impl WizardCreateReq
  fn db_form() -> DbFormInput
  fn effective_name() -> String — Nome efetivo: `app_name` tem precedência sobre `name` (ambos trimados).
fn build_spec(req) -> Result<ServiceSpec, String> — Monta o `ServiceSpec` a partir da requisição do wizard.
fn build_spec_inner(req) -> Result<ServiceSpec, String>
