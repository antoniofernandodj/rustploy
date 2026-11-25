# Índice: daemon: domínio, protocolo e nomes derivados

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-daemon/crates/daemon/src/domain/

### audit.rs — Auditoria de valores em claro no estado de infra (plano `docs/arquivo/plano-backup-estado-em-git.md`…
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
struct DockerVolumeInfo { name, driver, mountpoint, in_use, ref_count, size_bytes, service_id, service, project, orphan_of, has_backup_policy } — One Docker volume, independent of rustploy ownership (rustploy itself only ever bind-mounts host pat…
struct VolumeMountUse { service_id, service, project, container_path, borrowed } — Quem monta um volume e onde (tela Volumes).
struct VolumeInfo { name, size_bytes, in_use, service_id, service, project, orphan_of, has_backup_policy, anonymous, mounts } — Um volume na tela **Volumes**: o inventário com dono, órfão, backup e quem monta.
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

### environment.rs — **Ambiente**: o mesmo projeto, outra instância (homologação, preview…).
struct EnvironmentOf { origin_project_id, name, service_origins } — O vínculo de um projeto-ambiente com o projeto de onde ele foi copiado.
struct EnvironmentServicePair { service_name, origin_service_id, environment_service_id, origin_release_number, environment_release_number, origin_release_id, environment_release_id, same_image, can_promote_to_origin, can_bring_from_origin, note, changes } — Um par de serviços irmãos na comparação de um ambiente com a origem.
struct EnvironmentComparison { origin_project_id, origin_project_name, environment_project_id, environment_name, services, project_env_changes, only_in_origin, only_in_environment } — Resposta de `EnvironmentCompare`.
struct EnvironmentSummary { project_id, project_name, name } — Um ambiente de um projeto, na lista da origem.
fn environment_project_name(origin_name, environment_name) -> String — O nome do projeto-ambiente: `loja (homologação)`.
fn is_valid_environment_name(name) -> bool — Nome de ambiente aceito: 1–40 caracteres, sem parênteses (que entram no nome do projeto) nem quebra …
const ENVIRONMENT_FIELDS_IGNORED — Campos do spec que são diferentes por definição entre ambientes e não entram na comparação: domínios…
fn environment_relevant(changes) -> Vec<ReleaseChange> — Filtra das mudanças de um diff de spec os campos que diferem por definição entre ambientes.
(2 testes)

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
(6 testes)

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

### project.rs
struct Project { id, name, description, env_vars, env_comments, environment, created_at }

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

### release.rs — **Release**: o que rodou num deploy que chegou a Live — a imagem fixada e a configuração congelada d…
const PROTECTED_RELEASES_PER_SERVICE — Quantas releases mais recentes de cada serviço têm a imagem protegida da limpeza do Docker.
struct Release { id, service_id, deployment_id, number, image, image_ref, spec, project_env, restored_from_number, promoted_from, created_at, is_current, image_status }
enum ReleaseImageStatus { Local, Pullable, Missing, NotApplicable } — Se a imagem de uma release ainda está disponível para um "voltar".
struct ReleaseChange { scope, field, before, after } — Uma diferença entre duas configurações (duas releases, ou uma release e o que está declarado agora).
struct ReleaseDiff { from_label, to_label, changes } — Resposta de `ReleaseDiff`: o que muda indo de `from` para `to`.
struct ReleaseSide { image_ref, spec, project_env } — Uma configuração comparável: a imagem, o spec e as variáveis do projeto.
fn env_display(value) -> String — O valor de uma variável como a tela pode mostrar: secret vira `secret:NOME`.
fn env_map(vars) -> BTreeMap<String, String>
fn flatten(prefix, value, out) — Achata um valor JSON em `caminho → texto`.
const SPEC_FIELDS_COMPARED_APART — Campos do spec que não entram no diff: as variáveis (comparadas uma a uma, à parte), os comentários …
fn spec_fields(spec) -> BTreeMap<String, String>
fn diff_maps(scope, before, after, out)
fn diff_releases(from, to) -> Vec<ReleaseChange> — O que muda indo de `from` para `to`: imagem, campos do spec, variáveis do serviço e do projeto.
fn release_skips_build(source) -> bool — A fonte da release dá para voltar só trocando a imagem (pulando o build)? Compose não: refaz o `comp…
fn split_repository_tag(image) -> Option<(String, String)> — Separa `repo:tag` (aceitando registry com porta, `host:5000/app:v1`) para o `docker tag`.
fn pick_image_ref(from_registry, image, image_id, repo_digests) -> Option<String> — A referência fixa de uma imagem, a partir da inspeção dela.
fn image_ref_is_pullable(image_ref) -> bool — A referência é um digest de registry (`repo@sha256:…`), que dá para baixar de novo?
(6 testes)

### route.rs — **Rota**: o que o proxy faz com um domínio, fora do serviço.
struct Route { id, project_id, domain, tls, action, basic_auth } — Uma rota, guardada no projeto (`ProjectInfra.routes`).
enum RouteAction { Forward, Redirect }
struct PathRule { path_prefix, service_id, port, strip_prefix }
struct RouteBasicAuth { user, password }
fn normalize_route_domain(domain) -> String — O host normalizado (minúsculas, sem espaço, sem ponto final).
fn is_valid_route_domain(domain) -> bool — Host aceito: letras, dígitos, `-` e `.`, com pelo menos um ponto ou `localhost`/`.test`.
fn prefix_matches(prefix, path) -> bool — O prefixo casa com o caminho? `/api` casa com `/api` e `/api/x`, não com `/apix`; `/` casa com tudo.
fn match_rule(rules, path) -> Option<&'a PathRule> — A regra que atende o caminho: a de prefixo mais longo que casa.
fn strip_path(prefix, path_and_query) -> String — O caminho que o serviço recebe com `strip_prefix` (sempre começa com `/`).
fn redirect_location(to, path_and_query) -> String — O `Location` de um redirect: o destino sem a barra final + o caminho pedido.
fn validate_route_shape(route) -> Result<(), String> — Valida a forma de uma rota (o que não depende do resto do estado).
(3 testes)

### service.rs
struct ServiceSpec { name, description, project_id, source, port, host_port, tcp_ports, domain, tls_enabled, env_vars, env_comments, volumes, healthcheck, replicas, resources, run_command, run_args, product, domains, pre_deploy_job_id, pre_deploy_job_ids, shared, depends_on, autostart, registry_auto_deploy, links }
fn default_autostart() -> bool
const SERVICE_DESCRIPTION_MAX_CHARS — Tamanho máximo da descrição de um serviço, em caracteres.
fn normalize_description(description) -> Result<Option<String>, String> — A descrição como ela é gravada: sem espaços nas pontas, vazia = `None`.
fn description_summary(description, max_chars) -> (String, bool) — O resumo da descrição para listagens: a primeira linha não vazia, até `max_chars` (com `…` se cortou…
struct ServiceLink { service_id, env_prefix, resource, env_names } — "Este serviço usa aquele": ver [`ServiceSpec::links`].
const LINK_ENV_SUFFIXES — Os sufixos das variáveis que um vínculo pode entregar, na ordem em que aparecem na tela.
impl ServiceLink
  fn env_name(default_prefix, suffix) -> Option<String> — O nome com que a variável `suffix` entra no container, ou `None` se não deve ser entregue.
struct LinkResource { name, public_read } — O recurso de um vínculo: o nome dele no provedor.
enum ResourceKind { Bucket, Database } — Que tipo de recurso um provedor oferece, pelo `product` dele.
fn resource_kind(product) -> Option<ResourceKind>
fn is_valid_resource_name(kind, name) -> bool — O nome do recurso é válido para o tipo? É a única barreira antes de ele entrar numa URL S3 ou num SQ…
fn link_env_prefix(provider_name) -> String — O prefixo padrão das variáveis de um vínculo: o nome do provedor em maiúsculas, com tudo que não for…
fn is_valid_env_prefix(prefix) -> bool — Um prefixo escrito pela pessoa é aceito? Só `A-Z`, `0-9` e `_`, sem começar por dígito.
struct SharedServerConfig { accept_links } — Configuração de um servidor de banco compartilhado.
fn default_accept_links() -> bool
struct DomainRoute { domain, port, tls } — Uma rota HTTP de domínio de um serviço: qual domínio, para qual porta do container e com ou sem TLS.
impl DomainRoute
  fn container_port(default) -> u16 — Porta de container efetiva (a própria, ou a `port` padrão do serviço).
struct PortSuggestion { port, label, sources, internal } — Uma porta do container que o serviço provavelmente tem, sugerida ao configurar as portas TCP.
const MAX_TCP_PORTS — Máximo de portas TCP por serviço: uma lista longa por engano abriria meia faixa de portas no firewal…
struct TcpPortRoute { host_port, container_port, label } — Uma porta TCP do host ligada a uma porta do container.
impl TcpPortRoute
  fn container_port(default) -> u16 — Porta de container efetiva (a própria, ou a `port` padrão do serviço).
impl Service
  fn compose_project_name() -> String — Nome da stack Compose: o gravado; na falta (banco ainda não migrado, ou serviço que virou Compose de…
impl ServiceSpec
  fn safe_name() -> String
  fn domain_routes() -> Vec<DomainRoute> — Rotas HTTP efetivas do serviço: a lista `domains` nova, ou — para specs antigos que só têm o campo `…
  fn tcp_routes() -> Vec<TcpPortRoute> — Portas TCP efetivas do serviço: a lista `tcp_ports`, ou — para specs antigos que só têm o `host_port…
  fn materialize_tcp_ports() — Move o `host_port` legado para `tcp_ports` e **espelha** de volta no `host_port` a rota que vai para…
  fn ingress_container_ports() -> Vec<u16> — Portas de container que o ingress precisa alcançar: a de cada domínio e a de cada porta TCP do host.
  fn materialize_domains() — Move o domínio legado (`domain`/`tls_enabled`) para a lista `domains` e zera os campos legados, para…
  fn pre_deploy_checks() -> Vec<String> — Fila efetiva de checks de pré-deploy: `pre_deploy_job_ids` quando não vazia, ou — para specs antigos…
  fn materialize_pre_deploy_checks() — Move o `pre_deploy_job_id` legado para `pre_deploy_job_ids` e zera o campo legado — mesmo idioma de …
struct Service { id, spec, compose_project, status, live_container_id, created_at, updated_at }
enum ServiceStatus { Stopped, Stopping, Deploying, Running, Degraded, Error, Queued }
struct VolumeMount { host_path, container_path, read_only }
fn is_named_volume(host_path) -> bool — `host_path` de um [`VolumeMount`] é o nome de um **volume nomeado** do motor (e não uma pasta do ser…
fn validate_volume_mount(mount) -> Result<(), String> — Valida uma montagem: o caminho no container é absoluto (e não `/`), e a origem é um volume nomeado o…
struct Healthcheck { kind, interval_secs, timeout_secs, retries, start_period_secs }
enum HealthcheckKind { None, Http, Tcp, DockerNative }
struct ResourceLimits { cpu_shares, mem_limit_bytes }
struct ServiceWarmStatus { service_id, warm, warming, reason, warmed_at } — Se a imagem de um serviço com fonte Git ou zip já tem o cache de build quente no host, de modo que o…
impl Default for Healthcheck
impl std::fmt::Display for ServiceStatus
impl Default for SharedServerConfig
(11 testes)

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

### state_git.rs — Sincronização do estado de infra com um repositório git (plano `docs/arquivo/plano-backup-estado-em-…
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
(1 testes)

## rustploy-daemon/crates/daemon/src/protocol/

### command.rs — `Command`: tudo o que o cliente pede ao daemon.
enum StateGitResolveChoice { UseRemote, UseLocal } — Como resolver um histórico divergido (`Command::StateGitResolve`).
enum SecretMoveOp { EnvToSecret, SecretToEnv, ServiceSecretToProject, ProjectSecretToService, EnvServiceToProject, EnvProjectToService, EnvJobToProject, JobSecretToProject } — As operações de [`Command::SecretMove`].
enum SecretKind { Password, Hex, Base64Url } — O formato do valor que [`Command::SecretGenerate`] sorteia.
enum SecretOnExists { Fail, Keep, Replace } — O que [`Command::SecretGenerate`] faz quando o secret já existe no escopo.
enum Command { ProjectCreate, ProjectDelete, ProjectUpdate, ProjectList, ProjectServicesReorder, EnvironmentCreate, EnvironmentList, EnvironmentCompare, EnvironmentPromote, EnvironmentDestroy, ProjectEnvSet, ServiceCreate, ServiceUpdate, ServiceDescriptionSet, ServiceDelete, ServiceVolumeAttach, ServiceVolumeDetach, ServiceList, ServiceGet, DeployStart, DeployWithDependencies, ProjectDeploy, DeployWithDependenciesPlan, ProjectDeployPlan, DeployAbort, DeployRollback, ReleaseList, ReleaseDiff, ReleaseRollback, DeployHistory, DeployDelete, ServiceStop, ServiceReload, RecentDeployments, GetBuildLogs, LogsGet, LogsHistory, LogsExecutions, LogsExecutionDelete, LogsSubscribe, LogsUnsubscribe, MetricsSubscribe, MetricsUnsubscribe, ServiceConnectionInfo, SharedAccessList, SharedAccessGrant, SharedAccessRevoke, ManagedDatabaseList, ManagedDatabaseCreate, ManagedDatabaseDelete, ManagedDatabaseListAll, MigrationStart, MigrationGet, MigrationList, MigrationRollback, MigrationDiscard, GetWebhookUrl, RegenerateWebhookToken, GetDaemonSettings, SetDaemonSettings, SecretSet, SecretGenerate, SecretDelete, SecretList, SecretReveal, InfraAudit, InfraDownload, StateGitStatus, StateGitConfigure, StateGitSyncNow, StateGitResume, StateGitResolve, StateGitDismissRemoval, InfraAuditAllow, StopContainer, SecretMove, EnvKeyRename, EnvValueSet, SecretRename, ServiceExportPlan, ServiceExport, ServiceDataPlan, ServicePortSuggestions, BackupDestinationGet, BackupDestinationSet, BackupDestinationClear, BackupDestinationTest, ServiceBackupList, ServiceBackupNow, ServiceBackupRestore, ServiceBackupDelete, ServiceBackupPolicySet, ServerRecoverStart, ServerRecoverStatus, ServerRecoverRun, ServerRecoverCancel, RecoveryKeyStatus, RecoveryKeyGenerate, ServiceImport, JobCreate, JobUpdate, JobDelete, JobList, JobListAll, JobRunNow, JobRunCancel, JobRunHistory, GetJobLogs, JobWarmStatus, JobWarm, ServiceWarmStatusList, ServiceWarm, PruneContainers, PruneVolumes, PruneImages, PruneBuildCache, PruneNetworks, DockerCleanupConfigGet, DockerCleanupConfigSet, DockerCleanupRunNow, DockerImages, DockerVolumes, DockerNetworks, DockerContainers, RemoveContainer, RemoveImage, RemoveVolume, VolumeList, ComposeInsertExternalVolume, RemoveNetwork, StopAllManaged, IngressRoutes, RouteList, RouteSave, RouteDelete, IngressReconcile, EnvBackupList, EnvBackupRestore, Ping, DaemonStatus, DeployEngineStatus, CheckUpdate, ApplyUpdate, UpdateStatus, UpdateRollback, ApplyAppUpdate, GitProviderList, GitProviderCreate, GitProviderDelete, GitOAuthStart, GitRepoList, GitBranchList, WizardCatalog, WizardCreate, Snapshot, RegistryStatus, RegistryRepoList, RegistryTagList, RegistryTagDelete, RegistryRepoDelete, RegistryGc, RegistryRetentionSet, RegistryTokenCreate, RegistryTokenList, RegistryTokenRevoke, DeployQueuePromote, DeployQueueReorder, DeployQueuePause }
fn backup_stop_default() -> bool
fn null_as_empty(d) -> Result<String, D::Error> — `null` vira `""` num campo `String` cujo vazio já significa "nenhum".
(17 testes)

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
enum Response { Ok, Project, Projects, Service, Services, Deployment, Deployments, Volumes, ComposeWithVolume, Routes, Route, Environments, EnvironmentComparison, Releases, ReleaseDiff, DeployBatch, Logs, LogsHistory, LogsExecutions, LogsExecutionsDeleted, BuildLogs, DeploymentSummaries, DaemonStatus, UpdateInfo, UpdateProgress, AppUpdated, DeployEngineStatus, Pong, WebhookUrl, SharedAccessList, Migration, Migrations, ManagedDatabases, ConnectionInfo, DaemonSettings, SecretNames, SecretGenerated, StateGit, InfraAudit, InfraFiles, SecretValue, ServiceExportPlan, ServiceBundleYaml, ServiceDataPlan, ServicePortSuggestions, BackupDestination, ServiceBackups, ServerRecover, ServiceImportReport, GitProviders, GitProviderInfo, OAuthUrl, GitRepos, GitBranches, PruneResult, DockerCleanupConfig, EnvBackupSnapshots, DockerImages, DockerVolumes, DockerNetworks, DockerContainers, StopAllResult, IngressRoutes, WizardCatalog, Snapshot, Job, Jobs, JobSummaries, JobRun, JobRuns, JobLogs, JobWarmStatus, JobWarmRequested, ServiceWarmStatuses, ServiceWarmRequested, RegistryStatus, RegistryRepos, RegistryTags, RegistryGcResult, RegistryTokenCreated, RegistryTokens, RecoveryKey, RecoveryKeyGenerated, Err }
impl Response
  fn err(code, message) -> Self
