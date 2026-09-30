# Índice: daemon: persistência (db/)

> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## crates/daemon/src/db/

### build_logs.rs — Tabela `build_log`: linhas do log de build de cada deployment.
fn append(db, deployment_id, line, timestamp) -> Result<()>
fn delete_for_deployment(db, deployment_id) -> Result<()>
fn get_for_deployment(db, deployment_id) -> Result<Vec<BuildLogLine>>

### daemon_settings.rs — Tabela chave-valor de configurações do daemon editáveis pela UI (ACME, registry, limpeza do Docker).
fn get(db, key) -> Result<Option<String>>
fn set(db, key, value) -> Result<()>
fn delete(db, key) -> Result<()>
const KEY_WEBHOOK_BASE_URL — DEPRECADA: a base pública passou a ser derivada de `[api] domain`/`port` (ver `AppState::public_base…
const KEY_DOCKER_CLEANUP_CONFIG — JSON de `shared::DockerCleanupConfig` — ver `crate::maintenance`.
const KEY_DOCKER_CLEANUP_LAST_RUN — JSON de `shared::DockerCleanupLastRun`, ausente até a primeira execução.
const KEY_ACME_EMAIL, KEY_REGISTRY_DOMAIN

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
type Row = (String, String, String, String, String, Option<St…
fn row_to_stored(r) -> Result<StoredProvider>
impl StoredProvider
  fn to_public() -> GitProvider — Client-facing projection.
fn insert(db, p) -> Result<()>
fn list(db) -> Result<Vec<StoredProvider>>
fn get(db, id) -> Result<Option<StoredProvider>>
fn set_tokens(db, id, access_token_enc, refresh_token_enc, account_login, account_avatar) -> Result<()> — Records the connected account and its tokens once OAuth completes (or a PAT validates).
fn delete(db, id) -> Result<bool>
const COLS

### job.rs — Tabela `job`: jobs one-shot (Schedules), incluindo quais estão vencidos para o agendador.
type JobRow = (String, String, String, String, String, String, b…
fn row_to_job(row) -> Result<Job>
fn create(db, project_id, trigger_service_id, name, compose, git_source, main_service, env_vars, env_comments, recurrence) -> Result<Job>
fn get(db, id) -> Result<Option<Job>>
fn list(db, project_id) -> Result<Vec<Job>>
fn list_all(db) -> Result<Vec<Job>>
fn list_due(db, now) -> Result<Vec<Job>> — Jobs vencidos: habilitados, com agendamento configurado, e cujo `next_run_at` já passou — usado só p…
fn update(db, id, name, compose, git_source, main_service, env_vars, env_comments, enabled, recurrence) -> Result<Option<Job>>
fn mark_fired(db, id, last_run_at, next_run_at) -> Result<()> — Registra o disparo de um job: `last_run_at` = agora, `next_run_at` já avançado (`None` se o job não …
fn delete(db, id) -> Result<bool> — Apaga o job e o histórico junto (`job_run` + `job_log`) — não há FK com cascade no schema, então a l…
fn count_by_project(db, project_id) -> Result<i64>
fn delete_by_trigger_service(db, service_id) -> Result<u64> — Remove todos os jobs que usam o serviço como gatilho (cascade do `service_delete` — a GUI avisa no c…
const SELECT_COLS
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

### managed_database.rs — Tabela `managed_database`: databases criados em servidores compartilhados.
struct Row { id, server_service_id, project_id, name, password, env_var, connection_limit, statement_timeout_ms, created_at }
type Tuple = (String, String, String, String, String, String, O…
fn row(t) -> Row
fn insert(db, r) -> Result<Row>
fn list(db, server_id) -> Result<Vec<Row>>
fn get(db, id) -> Result<Option<Row>>
fn delete(db, id) -> Result<()>
fn delete_for_server(db, server_id) -> Result<()>
const COLS

### mod.rs — Conexão SQLite (`Db`) e as migrações do schema, feitas à mão com `add_column_if_missing`.
type Db = SqlitePool
fn connect(db_path) -> Result<Db>
fn migrate(pool) -> Result<()> — Cria as tabelas e aplica as migrações incrementais (`ALTER TABLE` idempotente via `add_column_if_mis…
fn connect_existing_for_test(pool) — Reexecuta a migração num banco já aberto (um boot seguinte), para os testes.
fn ensure_unique_index(pool, name, target, why) — Cria um índice `UNIQUE` sem derrubar o boot: se já há valores repetidos (uma colisão anterior ao pla…
fn add_column_if_missing(pool, sql) -> Result<()> — Executa um `ALTER TABLE ...

### projects.rs — Tabela `project`: CRUD de projetos e suas env vars de nível de projeto.
struct ProjectRow { id, name, description, env_vars, env_comments, created_at }
fn row_to_project(row) -> Result<Project>
fn create(db, name, description) -> Result<Project>
fn update_env_vars(db, id, env_vars, env_comments) -> Result<Option<Project>>
fn network_name(db, project_id) -> Result<String> — Nome da rede Docker do projeto — o gravado.
fn network_names(db) — `id do projeto → nome da rede` de todos os projetos (o inventário Docker descobre por aqui de quem é…
fn list(db) -> Result<Vec<Project>>
fn get(db, id) -> Result<Option<Project>>
fn update(db, id, name, description) -> Result<Option<Project>>
fn delete(db, id) -> Result<bool>
(1 testes)

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
fn gc_metadata(db) -> Result<()> — Fase de metadados do GC (`crate::registry::gc`), numa transação só:
fn all_cas_digests(db) -> Result<Vec<String>> — Todos os digests que DEVEM existir no CAS (blobs + manifests) — o conjunto "vivo" que o sweep do GC …
(13 testes)

### registry_tokens.rs — Tokens de acesso do registry OCI embutido (Basic auth — ver `crate::registry::auth`).
const RP_INTERNAL — Nome reservado do token interno usado pelo próprio deploy executor pra puxar imagens do registry emb…
struct TokenInfo { name, scope, created_at, last_used_at }
fn create(db, name, token_sha256, scope) -> Result<()>
fn upsert_internal(db, token_sha256) -> Result<()> — Cria ou atualiza o token interno `rp-internal`, regenerado a cada boot do daemon (ver `crate::regist…
fn list(db) -> Result<Vec<TokenInfo>>
fn revoke(db, name) -> Result<bool>
fn verify_scope(db, token_sha256) -> Result<Option<String>> — Retorna o escopo do token cujo hash bate, se existir.
fn touch_last_used(db, token_sha256) -> Result<()> — Best-effort, chamado em background (`tokio::spawn`) pelo caminho de auth — não deve atrasar a respos…
(7 testes)

### services.rs — Tabela `service`: CRUD do ServiceSpec, status e container live.
type ServiceRow = (String, String, String, String, String, Option<St…
fn row_to_service(row) -> Result<Service>
fn parse_status(s) -> ServiceStatus
fn new_compose_project(db, id, name) -> Result<String> — Nome de stack para um serviço Compose novo: o curto (`rp_<últimos 8 do ID>_<nome>`); se já está em u…
fn check_shared_rules(spec) -> Result<()> — Regras de nome/`shared` (prefixo `rp-shared-` reservado etc.) — ver `shared::connection::validate_sh…
fn create(db, spec) -> Result<Service>
fn list(db, project_id) -> Result<Vec<Service>>
fn get(db, id) -> Result<Option<Service>>
fn update_spec(db, id, spec) -> Result<Option<Service>>
fn clear_pre_deploy_job(db, job_id) -> Result<u64> — Remove o job dado da fila de pré-deploy check (`pre_deploy_job_ids`, e do `pre_deploy_job_id` legado…
fn update_status(db, id, status, container_id) -> Result<()>
fn delete(db, id) -> Result<bool>
fn get_running(db) -> Result<Vec<Service>>
fn count_by_project(db, project_id) -> Result<i64>
fn get_watchable(db) -> Result<Vec<Service>>
fn list_all(db) -> Result<Vec<Service>>
fn compose_service_keys(content) -> std::collections::BTreeSet<String> — Chaves de serviço (`services:`) de um compose, para casar com as de uma stack viva.
fn backfill_compose_projects(db, live) -> Result<()> — Grava `compose_project` nos serviços Compose que ainda não têm, com o nome **que a stack já usa** — …
const SELECT_COLS
(14 testes)

### shared_access.rs — Tabela `shared_access`: quais projetos podem alcançar cada servidor de banco compartilhado.
fn grant(db, server_id, project_id) -> Result<()>
fn revoke(db, server_id, project_id) -> Result<bool>
fn projects_of(db, server_id) -> Result<Vec<String>> — Projetos autorizados num servidor, em ordem de concessão.
fn delete_for_server(db, server_id) -> Result<()>
(1 testes)

### webhook_tokens.rs — Tabela de tokens de webhook de deploy, um por serviço.
fn get(db, service_id) -> Result<Option<String>>
fn upsert(db, service_id, token) -> Result<()>
fn _delete(db, service_id) -> Result<()>
fn generate_token() -> String
