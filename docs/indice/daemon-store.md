# Índice: daemon: persistência (store/)

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-daemon/crates/daemon/src/store/

### build_logs.rs — Tabela `build_log`: linhas do log de build de cada deployment.
fn append(db, deployment_id, line, timestamp) -> Result<()>
fn delete_for_deployment(db, deployment_id) -> Result<()>
fn get_for_deployment(db, deployment_id) -> Result<Vec<BuildLogLine>>

### daemon_settings.rs — Tabela chave-valor de configurações do daemon editáveis pela UI (ACME, registry, limpeza do Docker).
fn is_infra_key(key) -> bool — As três chaves de **configuração** moram no YAML de infra (`settings.yml`); as demais (`*_last_run`,…
fn get(db, key) -> Result<Option<String>>
fn set(db, key, value) -> Result<()>
fn delete(db, key) -> Result<()>
fn sql_get(db, key) -> Result<Option<String>> — Leitura **direta da tabela**, sem desviar para o YAML.
const KEY_WEBHOOK_BASE_URL — DEPRECADA: a base pública passou a ser derivada de `[api] domain`/`port` (ver `AppState::public_base…
const KEY_DOCKER_CLEANUP_CONFIG — JSON de `shared::DockerCleanupConfig` — ver `crate::features::maintenance`.
const KEY_DOCKER_CLEANUP_LAST_RUN — JSON de `shared::DockerCleanupLastRun`, ausente até a primeira execução.
const KEY_BUILD_CACHE_LAST_PRUNE — JSON de `BuildCachePrune`: quando o cache de build foi limpo pela última vez e com qual corte de ida…
const KEY_JOB_WARM_PREFIX — Prefixo das chaves `job_warm:<job_id>` (JSON de `jobs::warm::WarmRecord`).
const KEY_SERVICE_WARM_PREFIX — Prefixo das chaves `service_warm:<service_id>` (JSON de `maintenance::service_warm::WarmRecord`).
const KEY_ACME_EMAIL, KEY_REGISTRY_DOMAIN
(1 testes)

### deployments.rs — Tabela `deployment`: criar, transicionar de estado, histórico por serviço e estatísticas de 24h.
type DeploymentRow = (String, String, String, String, String, DateTime<…
fn row_to_deployment(row) -> Deployment
fn parse_state(s) -> DeployState
fn create(db, service_id, image) -> Result<Deployment>
fn get(db, id) -> Result<Option<Deployment>>
fn list_for_service(db, service_id, limit) -> Result<Vec<Deployment>>
fn _latest_for_service(db, service_id) -> Result<Option<Deployment>>
fn transition(db, id, from, to, message) -> Result<Deployment>
fn list_recent(db, limit) -> Result<Vec<Deployment>>
fn get_non_terminal(db) -> Result<Vec<Deployment>>
fn list_terminal_last_24h(db, limit) -> Result<Vec<Deployment>>
fn stats_last_24h(db) -> Result<(u64, u64, u64)>
fn delete(db, id) -> Result<()>
const SELECT_COLS

### git_providers.rs — Persistence for connected Git providers (Gitea OAuth2 / PAT).
struct StoredProvider { id, kind, name, base_url, auth_mode, oauth_client_id, oauth_client_secret_enc, access_token_enc, refresh_token_enc, account_login, account_avatar, created_at } — Full row as stored, including the encrypted secret columns.
impl StoredProvider
  fn to_public() -> GitProvider — Client-facing projection.
type Tokens = (Option<String>, Option<String>)
fn all_tokens(db) -> Result<BTreeMap<String, Tokens>>
fn stored(p, tokens) -> StoredProvider
fn put_tokens(db, id, access, refresh) -> Result<()>
fn insert(db, p) -> Result<()>
fn list(db) -> Result<Vec<StoredProvider>>
fn get(db, id) -> Result<Option<StoredProvider>>
fn set_tokens(db, id, access_token_enc, refresh_token_enc, account_login, account_avatar) -> Result<()> — Records the connected account and its tokens once OAuth completes (or a PAT validates).
fn delete(db, id) -> Result<bool>
(1 testes)

### job.rs — Tabela `job`: jobs one-shot (Schedules), incluindo quais estão vencidos para o agendador.
type Runtime = (Option<DateTime<Utc>>, Option<DateTime<Utc>>)
type RuntimeRow = (String, Option<DateTime<Utc>>, Option<DateTime<Ut…
fn runtimes(db) -> Result<HashMap<String, Runtime>>
fn put_runtime(db, id, last, next) -> Result<()>
fn assemble(project_id, j, rt) -> Job
fn all_jobs(db) -> Result<Vec<Job>>
fn create(db, project_id, trigger_service_id, name, compose, git_source, main_service, env_vars, env_comments, recurrence) -> Result<Job>
fn get(db, id) -> Result<Option<Job>>
fn list(db, project_id) -> Result<Vec<Job>>
fn list_all(db) -> Result<Vec<Job>>
fn list_due(db, now) -> Result<Vec<Job>> — Jobs vencidos: habilitados, com agendamento configurado, e cujo `next_run_at` já passou — usado só p…
fn update(db, id, name, compose, git_source, main_service, env_vars, env_comments, enabled, recurrence) -> Result<Option<Job>>
fn mark_fired(db, id, last_run_at, next_run_at) -> Result<()> — Registra o disparo de um job: `last_run_at` = agora, `next_run_at` já avançado (`None` se o job não …
fn delete(db, id) -> Result<bool> — Apaga o job e o histórico junto (`job_run` + `job_log` + runtime) — não há FK com cascade no schema,…
fn count_by_project(db, project_id) -> Result<i64>
fn delete_by_trigger_service(db, service_id) -> Result<u64> — Remove todos os jobs que usam o serviço como gatilho (cascade do `service_delete` — a GUI avisa no c…
(5 testes)

### job_log.rs — Tabela `job_log`: linhas de log (stdout/stderr) de cada execução de job.
fn stream_to_str(s) -> &'static str
fn str_to_stream(s) -> LogStream
fn append(db, job_run_id, stream, line, timestamp) -> Result<()>
fn get_for_run(db, job_run_id) -> Result<Vec<BuildLogLine>>
(1 testes)

### job_run.rs — Tabela `job_run`: cada execução de um job e seu exit code.
type JobRunRow = (String, String, DateTime<Utc>, Option<DateTime<Ut…
fn row_to_job_run(row) -> JobRun
fn create(db, job_id) -> Result<JobRun>
fn finish(db, id, exit_code) -> Result<Option<JobRun>> — Fecha uma execução com o exit code do processo `docker compose`.
fn has_unfinished(db, job_id) -> Result<bool> — Há execução deste job **sem fim** — esperando a vez na fila ou rodando? O agendador usa para não emp…
fn list_unfinished(db) -> Result<Vec<JobRun>> — Todas as execuções sem fim, de qualquer job.
fn get(db, id) -> Result<Option<JobRun>>
fn list_for_job(db, job_id, limit) -> Result<Vec<JobRun>>
fn latest_for_job(db, job_id) -> Result<Option<JobRun>>
const SELECT_COLS
(3 testes)

### log_sink.rs — Grava as linhas de log de build/job nas tabelas `build_log` e `job_log`.
impl LogSink for Db: append_build_line, append_job_line

### managed_database.rs — Tabela `managed_database`: databases criados em servidores compartilhados.
struct Row { id, server_service_id, project_id, name, password, env_var, connection_limit, statement_timeout_ms, created_at }
fn to_row(project_id, m) -> Row
fn all_rows(db) -> Vec<Row>
fn insert(db, r) -> Result<Row>
fn list(db, server_id) -> Result<Vec<Row>>
fn get(db, id) -> Result<Option<Row>>
fn delete(db, id) -> Result<()>
fn delete_for_server(db, server_id) -> Result<()>
(1 testes)

### migration.rs — Tabela `migration`: estado do assistente de migração (passos, log, o que foi parado e as env vars tr…
struct Record { m, env_changes, stopped_services } — Linha completa: a `Migration` pública + o que só o daemon precisa.
type Tuple = (String, String, String, String, String, String, S…
fn record(t) -> Result<Record>
fn save(db, r) -> Result<()>
fn get(db, id) -> Result<Option<Record>>
fn list(db, project_id) -> Result<Vec<Record>>
fn active_for_source(db, source_service_id) -> Result<bool> — Há migração `Running`/`Completed` (ainda não revertida/descartada) deste banco?
fn fail_orphans(db) -> Result<u64> — Migrações deixadas `Running` por um daemon que caiu no meio: viram `Failed` (o dump não retoma; o us…
const COLS

### mod.rs — Conexão SQLite (`Db`) e as migrações do schema, feitas à mão com `add_column_if_missing`.
struct Db { pool, infra } — O armazenamento do daemon: o **SQLite** (runtime: deploys, logs, status…) e o **repositório de infra…
impl Db
  fn pool() -> &SqlitePool
  fn infra() -> &infra::repo::Repo — O repositório de infra (leituras: `snapshot()`; escritas: `update()`).
  fn seed_project_for_test(id) — Cria (se ainda não existe) um projeto com este id — para testes que precisam de um dono para os serv…
  fn seed_service_for_test(project_id, service_id) — Cria (se ainda não existe) um serviço com este id dentro do projeto (que já deve existir).
  fn lazy_for_test() -> Db — Banco que nunca chega a conectar (só para quem precisa de um `Db` de enfeite).
  fn memory_for_test() -> Db — Banco em memória + repositório em pasta temporária, para os testes.
fn connect(db_path) -> Result<Db>
fn migrate(pool) -> Result<()> — Cria as tabelas e aplica as migrações incrementais (`ALTER TABLE` idempotente via `add_column_if_mis…
fn connect_existing_for_test(db) — Reexecuta a migração num banco já aberto (um boot seguinte), para os testes.
fn ensure_unique_index(pool, name, target, why) — Cria um índice `UNIQUE` sem derrubar o boot: se já há valores repetidos (uma colisão anterior ao pla…
fn add_column_if_missing(pool, sql) -> Result<()> — Executa um `ALTER TABLE ...
impl std::ops::Deref for Db

### projects.rs — Tabela `project`: CRUD de projetos e suas env vars de nível de projeto.
fn to_project(p) -> Project
fn create(db, name, description) -> Result<Project>
fn update_env_vars(db, id, env_vars, env_comments) -> Result<Option<Project>>
fn network_name(db, project_id) -> Result<String> — Nome da rede Docker do projeto — o gravado.
fn network_names(db) — `id do projeto → nome da rede` de todos os projetos (o inventário Docker descobre por aqui de quem é…
fn list(db) -> Result<Vec<Project>>
fn get(db, id) -> Result<Option<Project>>
fn update(db, id, name, description) -> Result<Option<Project>>
fn delete(db, id) -> Result<bool>
(2 testes)

### registry.rs — Wrappers SQL do registry OCI embutido (metadados; os bytes de blob/manifest vivem no CAS em disco, v…
struct Repo { id }
struct ManifestRow { media_type, size }
struct RepoSummary { name, tag_count, size_bytes, created_at } — Linha da lista de repositórios (sub-aba Registry).
struct TagDetail { tag, digest, media_type, size_bytes, updated_at } — Linha da lista de tags de um repositório (sub-aba Registry, detalhe).
struct RegistrySummary { repo_count, blob_count, storage_bytes } — Agregados globais do registry, para o cabeçalho da sub-aba Registry.
fn get_or_create_repo(db, name) -> Result<Repo> — Busca o repo por nome; cria (`rrepo_<ulid>`) se ainda não existir.
fn get_repo_by_name(db, name) -> Result<Option<Repo>> — Só leitura — usada nas rotas GET/HEAD que não devem criar repo implicitamente (manifests/tags/blobs …
fn list_repo_names(db) -> Result<Vec<String>> — `GET /v2/_catalog` — nomes ordenados.
fn list_repos(db) -> Result<Vec<RepoSummary>> — Lista de repositórios com contagem de tags + tamanho agregado (soma dos manifests do repo — aproxima…
fn list_tags_detailed(db, repo_id) -> Result<Vec<TagDetail>> — Tags de um repositório com o manifest que cada uma aponta (digest, media_type, tamanho).
fn summary(db) -> Result<RegistrySummary> — Agregados globais (repos/blobs/tamanho total) para o cabeçalho da sub-aba Registry.
fn delete_repo(db, repo_id) -> Result<bool> — Remove o repositório inteiro (todos os manifests/tags/refs) — só metadados, não mexe no CAS em disco…
fn insert_blob(db, digest, size) -> Result<()> — Idempotente — uploads concorrentes do mesmo blob finalizando quase ao mesmo tempo não colidem.
fn blob_exists(db, digest) -> Result<bool>
fn insert_manifest(db, digest, repo_id, media_type, size, refs) -> Result<()> — Grava o manifest e substitui suas refs numa transação — idempotente: republicar a mesma tag/digest n…
fn get_manifest(db, repo_id, digest) -> Result<Option<ManifestRow>> — Confere que o digest pertence ao repo (multi-tenant seguro: um repo não pode ler manifest de outro s…
fn ref_blob_or_manifest_exists(db, digest) -> Result<bool> — Checagem GLOBAL no CAS (não por repo) — decisão deliberada: exigir refs presentes globalmente simpli…
fn upsert_tag(db, repo_id, tag, manifest_digest) -> Result<()>
fn get_tag_digest(db, repo_id, tag) -> Result<Option<String>>
fn list_tags(db, repo_id) -> Result<Vec<String>>
fn delete_manifest(db, repo_id, digest) -> Result<bool> — Remove o manifest deste repo e as tags dele que apontavam para ele — só metadados (blobs órfãos são …
fn gc_metadata(db) -> Result<()> — Fase de metadados do GC (`crate::features::registry::gc`), numa transação só:
fn all_cas_digests(db) -> Result<Vec<String>> — Todos os digests que DEVEM existir no CAS (blobs + manifests) — o conjunto "vivo" que o sweep do GC …
(13 testes)

### registry_tokens.rs — Tokens de acesso do registry OCI embutido (Basic auth — ver `crate::features::registry::auth`).
const RP_INTERNAL — Nome reservado do token interno usado pelo próprio deploy executor pra puxar imagens do registry emb…
struct TokenInfo { name, scope, created_at, last_used_at }
fn create(db, name, token_sha256, scope) -> Result<()>
fn upsert_internal(db, token_sha256) -> Result<()> — Cria ou atualiza o token interno `rp-internal`, regenerado a cada boot do daemon (ver `crate::featur…
fn list(db) -> Result<Vec<TokenInfo>>
fn revoke(db, name) -> Result<bool>
fn verify_scope(db, token_sha256) -> Result<Option<String>> — Retorna o escopo do token cujo hash bate, se existir.
fn touch_last_used(db, token_sha256) -> Result<()> — Best-effort, chamado em background (`tokio::spawn`) pelo caminho de auth — não deve atrasar a respos…
(7 testes)

### services.rs — Serviços: o `ServiceSpec` e o nome da stack Compose moram no **YAML de infra** (`store::infra`); o *…
type Runtime = (String, Option<String>, String, DateTime<Utc>) — Linha de `service_runtime`: `(status, live_container_id, desired_state, updated_at)`.
type RuntimeRow = (String, String, Option<String>, String, DateTime<…
fn runtimes(db) -> Result<HashMap<String, Runtime>>
fn parse_status(s) -> ServiceStatus
fn assemble(s, rt) -> Service — Junta a parte declarada (YAML) com o runtime (SQL).
fn all_services(db) -> Result<Vec<(Service, Option<Runtime>)>> — Todos os serviços do YAML, em ordem de criação, com o runtime junto.
fn filtered(db, keep) -> Result<Vec<Service>>
fn new_compose_project(st, id, name) -> String — Nome de stack para um serviço Compose novo: o curto (`rp_<últimos 8 do ID>_<nome>`); se já está em u…
fn check_shared_rules(spec) -> Result<()> — Regras de nome/`shared` (prefixo `rp-shared-` reservado etc.) — ver `crate::platform::connection::va…
fn create(db, spec) -> Result<Service>
fn list(db, project_id) -> Result<Vec<Service>>
fn get(db, id) -> Result<Option<Service>>
fn update_spec(db, id, spec) -> Result<Option<Service>>
fn clear_pre_deploy_job(db, job_id) -> Result<u64> — Remove o job dado da fila de pré-deploy check (`pre_deploy_job_ids`, e do `pre_deploy_job_id` legado…
fn update_status(db, id, status, container_id) -> Result<()>
fn delete(db, id) -> Result<bool>
enum DesiredState { Running, Stopped } — Intenção do usuário para um serviço, **distinta do `status` observado**: `Running` = "deve estar no …
impl DesiredState
  fn as_str() -> &'static str
fn set_desired_state(db, id, state) -> Result<()> — Grava a intenção.
fn desired_state(db, id) -> Result<DesiredState>
fn get_desired_running(db) -> Result<Vec<Service>> — Serviços que o usuário quer no ar (`desired_state = Running`), qualquer que seja o `status` observad…
fn get_running(db) -> Result<Vec<Service>>
fn count_by_project(db, project_id) -> Result<i64>
fn get_watchable(db) -> Result<Vec<Service>>
fn list_all(db) -> Result<Vec<Service>>
fn test_compose_spec(name, project_id) -> ServiceSpec
fn compose_service_keys(content) -> std::collections::BTreeSet<String> — Chaves de serviço (`services:`) de um compose, para casar com as de uma stack viva.
fn backfill_compose_projects(db, live) -> Result<()> — Grava `compose_project` nos serviços Compose que ainda não têm, com o nome **que a stack já usa** — …
(22 testes)

### shared_access.rs — Tabela `shared_access`: quais projetos podem alcançar cada servidor de banco compartilhado.
fn grant(db, server_id, project_id) -> Result<()>
fn revoke(db, server_id, project_id) -> Result<bool>
fn rows(db) — `(servidor, projeto, concedido em)` de todos os acessos, em ordem de concessão.
fn projects_of(db, server_id) -> Result<Vec<String>> — Projetos autorizados num servidor, em ordem de concessão.
fn delete_for_server(db, server_id) -> Result<()>
fn all(db) -> Result<Vec<(String, String)>> — Todos os acessos concedidos: `(server_service_id, project_id)`.
(1 testes)

### state_git_marks.rs — Marcas de runtime da sincronização com o git (Fase 7): o que o `pull` deixou **pendente de decisão h…
type RemovedRow = (String, String, String, String, String, String, O… — Linha de `state_git_removed`: `(id, kind, name, project, sha, at, compose_project)`.
struct Removed { id, kind, name, project, sha, at, compose_project }
fn add_removed(db, r) -> Result<()>
fn list_removed(db) -> Result<Vec<Removed>>
fn clear_returned(db, service_ids, project_ids) -> Result<usize> — Tira as marcas de tudo o que **voltou** ao estado (um rollback no git que devolve o serviço): o cont…
fn dismiss_removed(db, id) -> Result<bool>
fn set_pending(db, service_id, sha) -> Result<()>
fn clear_pending(db, service_id) -> Result<()>
fn list_pending(db) -> Result<Vec<String>>

### webhook_tokens.rs — Tabela de tokens de webhook de deploy, um por serviço.
fn service_mut(st, service_id)
fn get(db, secrets, service_id) -> Result<Option<String>>
fn upsert(db, secrets, service_id, token) -> Result<()>
fn _delete(db, service_id) -> Result<()>
fn generate_token() -> String

## rustploy-daemon/crates/daemon/src/store/infra/

### audit.rs — **Auditoria de valores em claro** no estado de infra (plano `docs/plano-backup-estado-em-git.md`, R5…
enum Certainty { Definite, Suspect }
struct Finding { certainty, project, scope, what }
const SENSITIVE_NAME_PARTS — Fragmentos de nome que denunciam um segredo (comparados em maiúsculas).
fn name_looks_sensitive(key) -> bool — O nome da variável denuncia um segredo? (heurística — ver o aviso no topo do módulo)
fn url_with_credentials(value) -> bool — `scheme://usuario:senha@host` — credencial embutida numa URL.
fn is_reference(value) -> bool — Valor que é só uma referência a ser resolvida depois (`${VAR}`), não um segredo literal.
fn check_env(out, project, scope, vars)
fn compose_suspect_lines(content) -> Vec<String> — Linhas de compose do tipo `POSTGRES_PASSWORD: valor` / `- API_TOKEN=valor` com valor literal.
fn audit_project(out, p)
fn plaintext_findings(st) -> Vec<Finding> — Tudo o que está em claro (ou parece estar), em ordem estável.
(6 testes)

### migrate.rs — A migração **única** SQL → YAML (Fase 4 do plano, seção R4).
const MARK_KEY — Chave em `daemon_settings` que marca a migração como concluída.
enum Outcome { AlreadyDone, Migrated }
type Dt = DateTime<Utc>
fn read_sql(db) -> Result<(InfraState, Vec<String>)> — Lê as tabelas de infra e monta o [`InfraState`] + avisos de órfãos.
fn backup_sqlite(db, db_path) -> Result<PathBuf> — Cópia consistente do SQLite (`VACUUM INTO`, que é segura com o banco em uso).
fn restrict(p)
fn restrict(_)
fn undo(tree) -> Result<()> — Apaga tudo o que a migração gravou (volta ao "sem infra/").
fn run(db, tree, db_path) -> Result<Outcome> — Roda a migração.
fn first_difference(a, b) -> String — Mensagem útil quando a ida e volta diverge: diz **onde**, sem despejar o estado inteiro (que pode te…
(7 testes)

### mod.rs — Camada de repositório da **infraestrutura** declarada (projetos, serviços, jobs…), que passa a morar…

### model.rs — O **estado de infra tipado** (Fase 4 do plano `docs/plano-backup-estado-em-git.md`, R2/R3) e como el…
const SCHEMA — Versão do formato dos arquivos.
struct InfraState { settings, providers, projects }
struct SettingsInfra { acme_email, registry_domain, docker_cleanup_config } — As chaves de configuração do `daemon_settings` (R2.1).
struct ProviderInfra { id, kind, name, base_url, auth_mode, oauth_client_id, oauth_client_secret_enc, account_login, account_avatar, created_at } — Config de um git provider.
struct ProjectInfra { id, name, description, env_vars, env_comments, network_name, created_at, services, jobs, secrets, managed_databases, shared_access }
struct ServiceInfra { id, spec, compose_project, created_at, webhook_token, secrets }
struct JobInfra { id, trigger_service_id, name, compose, git_source, main_service, env_vars, env_comments, enabled, recurrence, created_at }
struct SecretInfra { id, key, value } — Hoje o `value` já vem cifrado pelo `SecretsManager` (passphrase).
struct ManagedDatabaseInfra { id, server_service_id, name, password, env_var, connection_limit, statement_timeout_ms, created_at } — **Atenção:** `password` está em claro (é assim no SQL hoje).
struct SharedAccessInfra { server_service_id, created_at }
struct ProvidersFile { providers }
struct ManifestFile { schema }
fn disk_schema(tree) -> Result<Option<u32>> — Versão do formato gravada na pasta (`None` = sem `MANIFEST.json`).
fn is_state_file(rel) -> bool — O caminho é um arquivo do estado (e não um `README`, `.gitignore`…)?
fn project_path(id) -> String
impl InfraState
  fn normalize() — Põe tudo em ordem estável.
  fn validate() -> Result<()> — Valida as referências entre entidades — o "valida tudo" do passo 2 da transação (R3).
  fn to_files() -> Result<BTreeMap<String, Vec<u8>>> — O estado como mapa `caminho → bytes`, na ordem estável.
  fn read(tree) -> Result<Option<InfraState>> — Lê o estado de uma árvore de arquivos.
  fn from_files(files) -> Result<Option<InfraState>> — Monta o estado a partir de `caminho → bytes` (de um disco **ou de uma árvore do git**, sem checkout)…
  fn write(tree) -> Result<()> — Grava o estado inteiro numa transação: escreve o que mudou e **apaga** os projetos que deixaram de e…
fn upgrade_project_v1(bytes) -> Result<ProjectInfra> — Lê um `project.yml` do schema 1: renomeia `db_kind` → `product` em cada `services[].spec` antes de t…
const MANIFEST, SETTINGS, PROVIDERS
(6 testes)

### repo.rs — O **repositório** do estado de infra: o `InfraState` em memória (as leituras vêm daqui) mais a pasta…
struct Repo { tree, changes, state, gate, _tmp }
impl Repo
  fn open(dir) -> Result<Self> — Abre (e recupera de queda, se for o caso) a pasta e carrega o estado.
  fn temp() -> Self — Repositório numa pasta temporária que some junto com ele.
  fn tree() -> &FileTree
  fn subscribe() -> tokio::sync::watch::Receiver<u64> — Acorda quando o estado muda (o valor é só um contador crescente).
  fn exclusive(f) -> R — Roda `f` com o **lock de escrita tomado**: nenhuma transação está no meio e nenhuma começa até `f` t…
  fn is_initialized() -> Result<bool> — Já existe estado gravado no disco (`MANIFEST.json`)?
  fn snapshot() -> Arc<InfraState> — O estado atual.
  fn apply_files(files, check, finish) — Aplica uma **árvore inteira vinda de fora** (o `pull` do git) como UMA transação: lê com [`InfraStat…
  fn reload_checked() — Relê a pasta do disco, **valida** e só então troca o estado em memória.
  fn reload() -> Result<()> — Substitui o estado em memória pelo que está no disco (depois da migração).
  fn update(f) -> Result<R> — Aplica `f` ao estado e grava.
(4 testes)

### txn.rs — Transação de "tudo ou nada" sobre um conjunto de arquivos num diretório.
struct Tx { writes, deletes } — Conjunto de mudanças aplicado atomicamente por [`FileTree::commit`].
impl Tx
  fn new() -> Self
  fn write(rel, bytes) -> &mut Self — Cria ou sobrescreve `rel` (caminho relativo, com `/`).
  fn delete(rel) -> &mut Self — Apaga `rel` (sem erro se não existir).
  fn is_empty() -> bool
struct Plan { writes, deletes }
struct FileTree { root, _lock, commit_gate, crash_at } — Diretório de arquivos com escrita transacional.
enum Step { AfterStagingFiles, AfterPlan, AfterCommitMarker, AfterApply, BeforeCleanup } — Pontos em que os testes simulam uma queda do processo (sem nenhuma limpeza).
struct InjectedCrash(Step)
impl FileTree
  fn open(root) -> Result<Self> — Abre (criando se preciso) o diretório, toma o lock e **recupera** qualquer transação interrompida po…
  fn root() -> &Path
  fn read(rel) -> Result<Option<Vec<u8>>> — Lê um arquivo; `None` se não existir.
  fn list() -> Result<Vec<String>> — Todos os arquivos (caminhos relativos, ordenados), ignorando tudo que começa com `.` (`.lock`, `.sta…
  fn commit(tx) -> Result<()> — Aplica a transação inteira ou nada.
  fn stage_and_apply(tx, txdir) -> Result<()>
  fn apply(txdir, plan) -> Result<()> — Aplica o plano de uma transação já commitada.
  fn recover() -> Result<()> — Resolve transações interrompidas: com `COMMIT` → termina; sem → descarta.
  fn inject_crash(step)
  fn crash(step) -> Result<()>
  fn crash(_step) -> Result<()>
type Step_ = Step
enum Step_ { AfterStagingFiles, AfterPlan, AfterCommitMarker, AfterApply, BeforeCleanup } — Em produção `Step` não existe; este alias vazio mantém as chamadas `self.crash(...)` iguais.
fn check_rel(rel) -> Result<()> — `rel` precisa ser relativo, sem `..`, sem componente vazio e sem componente começando por `.` (reser…
fn write_synced(path, bytes) -> Result<()>
fn sync_dir(path) -> Result<()>
fn walk(dir, prefix, out) -> Result<()>
const STAGING, LOCK, COMMIT, PLAN, FILES
impl std::fmt::Display, std::error::Error for InjectedCrash
(7 testes)

### yaml.rs — Serialização YAML **determinística** do estado de infra.
fn to_yaml(value) -> Result<Vec<u8>>
fn from_yaml(bytes) -> Result<T>
(2 testes)
