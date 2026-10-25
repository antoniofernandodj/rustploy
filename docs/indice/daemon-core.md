# Índice: daemon: main, bootstrap, platform (config, eventos, segredos) e o resto

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-daemon/crates/daemon/

### build.rs — Gera, em tempo de compilação, os assets estáticos da web UI/PWA (`crates/daemon/webui/`, ver `docs/`…
fn main()
fn collect_files(root, dir, out) — Percorre `dir` recursivamente, coletando caminhos relativos a `root` (sempre com `/`, mesmo no Windo…
fn route_for(rel) -> String
fn content_type_for(rel) -> &'static str
fn minify(src, is_html) -> String — Minificação conservadora — SÓ remove comentários `<!-- -->` do HTML (delimitadores inequívocos, o pa…
fn gzip(data) -> Vec<u8>
fn fnv1a(h, data) -> u64
const FNV_OFFSET, FNV_PRIME

## rustploy-daemon/crates/daemon/build/

### templates_catalog.rs — Gera, em tempo de compilação, o catálogo estático de templates a partir dos blueprints escritos à mã…
struct RawTemplate { variables, config }
struct RawConfig { env, domains, mounts }
struct RawDomain { service_name, port, host, path }
fn generate()
fn read_meta(dir, id) -> (String, String) — `meta-entry.json` (só alguns blueprints têm) traz description/logo melhores.
fn logo_from_list(id) -> Option<String> — Os arquivos de logo NÃO vivem neste crate (pesavam 14 MB e estouravam o limite do crates.io): morara…
fn json_string_field(s, field) -> Option<String>
fn find_logo(dir) -> Option<String>
fn prettify(id) -> String — `gitea_mysql` → "Gitea Mysql"; `open-webui` → "Open Webui".
fn normalize_env(env) -> Vec<(String, String)> — Normaliza `config.env` (array `"K=V"` ou tabela) para pares `(K, V)`.
fn lit(s) -> String — Literal Rust `&str` válido (escapado) para `s`.

## rustploy-daemon/crates/daemon/src/

### main.rs — Ponto de entrada do `rustployd`: delega tudo à montagem do sistema em `bootstrap`.
fn main() -> anyhow::Result<()>
const GLOBAL

## rustploy-daemon/crates/daemon/src/bin/

### rustployd-fw.rs — `rustployd-fw` — helper privilegiado de firewall do rustploy.
struct Request { op, port }
struct Response { ok, backend, error }
impl Response
  fn ok(backend) -> Self
  fn err(msg) -> Self
struct PortRange { range_start, range_end }
fn default_range_start() -> u16
fn default_range_end() -> u16
fn load_range() -> PortRange — Lê só a seção `[external_ports]` da config do rustploy — o resto do arquivo não interessa ao helper …
fn listener() -> UnixListener — Socket herdado do systemd (LISTEN_FDS, fd 3) ou, em dev, criado pelo próprio helper em `$RUSTPLOY_FW…
fn main()
fn handle_conn(stream, range)
fn handle_request(req, range) -> Response
fn ufw_active() -> Option<bool> — `Some(true)` = ufw instalado e ativo; `Some(false)` = instalado e inativo; `None` = não instalado.
fn apply_ufw(port, allow) -> Response
impl Default for PortRange

## rustploy-daemon/crates/daemon/src/bootstrap/

### config.rs — Resolução de diretórios (banco, chave mestra) com fallback para o home e inicialização do log.
fn init_logging(level)
fn fallback_dir() -> PathBuf
fn resolve_data_path(configured) -> PathBuf — Tries to use `configured` as the data directory.
fn resolve_master_key_path(configured) -> PathBuf — Tries to use `configured` as the master key path.
fn can_write_dir(dir) -> bool — Returns true only when `dir` (or its path) is both creatable and writable.

### listeners.rs — Listeners do daemon: registry OCI embutido (storage e rota), proxy de ingress e API HTTP.
fn prepare_registry_storage(config, db_path) -> Option<Arc<RegistryStorage>> — Storage do registry OCI embutido, criado ANTES do `AppState`: o listener HTTP do registry, o handler…
fn publish_registry_route(config, db, ingress, tls) — Domínio público do registry — banco tem precedência sobre o config (mesmo padrão do e-mail ACME).
fn spawn(config, state, ingress, tls, registry_storage) — Sobe o proxy de ingress, a API HTTP/JSON + SSE e o listener do registry.

### mod.rs — Montagem do `rustployd`: config e diretórios, banco, Docker, ingress, `AppState`, recuperação de boo…
fn run() -> Result<()> — Sobe o daemon inteiro e segura o processo (encerramento é o sinal do systemd).

### state.rs — Estado compartilhado do daemon (`AppState`) e os caches do inventário Docker.
type OAuthStates = Arc<Mutex<HashMap<String, String>>> — Pending OAuth handshakes: CSRF `state` → `provider_id`, consumed by the `/oauth/gitea/callback` rout…
struct AppState { db, docker, ingress, bus, secrets, tls, db_path, backup_dir, drain_secs, api, started_at, oauth_states, active_deploys, active_jobs, warming_jobs, warming_services, deploy_queue, docker_cache, registry_storage, registry_internal_token, update_checker }
impl AppState
  fn new(db, docker, ingress, bus, secrets, tls, db_path, backup_dir, drain_secs, api, update, registry_storage, registry_internal_token) -> Self
  fn deploy_ctx() -> DeployContext
  fn jobs_ctx() -> JobsContext
  fn maintenance_ctx() -> MaintenanceContext
  fn watchdog_ctx() -> WatchdogContext
  fn services_ctx() -> ServicesContext
  fn migration_ctx() -> MigrationContext
  fn iac_ctx() -> IacContext
  fn public_base_url() -> String — URL pública do daemon, sem barra final: base do webhook (`{base}/webhook/{service_id}/{token}`) e do…

### workers.rs — Tarefas de fundo do daemon: backup de env, métricas, logs, GC do registry, watchdog, agendadores e r…
fn spawn_standalone(config, db, docker, bus, backup_dir) — Laços que só precisam de banco, Docker e barramento (sobem antes do `AppState`).
fn spawn_stateful(state, registry_storage, tls) — Laços que dependem do `AppState` já montado.

## rustploy-daemon/crates/daemon/src/platform/

### config.rs — Configuração do daemon (`config.toml`): structs de cada seção com defaults e o singleton `CONFIG`.
const CONFIG — Process-wide configuration singleton.
struct EnvBackupConfig { dir, interval_secs }
fn default_env_backup_interval() -> u64
struct RustployConfig { daemon, ingress, docker, deploy, metrics, secrets, api, env_backup, external_ports, registry, update }
struct UpdateConfig { enabled, repo, cache_hours } — Verificação de versão nova (`Command::CheckUpdate`).
fn default_update_enabled() -> bool
fn default_update_repo() -> String
fn default_update_cache_hours() -> u64
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
fn outbound_ip() -> String — Detecta o IP de saída da máquina conectando um socket UDP em 8.8.8.8:80 (sem enviar dados) e lendo o…
impl Default for ApiConfig
impl Default for EnvBackupConfig
impl Default for ExternalPortsConfig
impl Default for RegistryConfig
impl Default for RustployConfig
impl Default for UpdateConfig
(3 testes)

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

### errors.rs — Mensagens de erro amigáveis para o usuário.
fn humanize_db_error(err, subject) -> String — Converte um erro de banco (sqlx) numa mensagem amigável para o usuário.

### event_bus.rs — Bus de eventos em memória (broadcast): o que alimenta o SSE `/api/events` dos clientes.
struct EventBus { sender }
impl EventBus
  fn new() -> Self
  fn publish(event) — Publica um evento no bus, para os assinantes conectados no momento.
  fn subscribe() -> broadcast::Receiver<Event>
const CHANNEL_CAPACITY
impl Default for EventBus

### health.rs — Checagens de saúde HTTP e TCP usadas pelo healthcheck do deploy e pelo watchdog.
fn check_http(url, expected, timeout) -> bool
fn check_tcp(addr, timeout) -> bool

### https_client.rs — Cliente HTTPS de saída mínimo: um GET, hyper cru sobre `tokio-rustls`.
struct HttpsResponse { status, headers, body } — Resposta já lida por inteiro (o corpo é limitado por `max_body_bytes`).
impl HttpsResponse
  fn header(name) -> Option<&str>
struct UrlParts { host, port, path_and_query } — Partes de uma URL `https://host[:porta]/caminho?consulta`.
fn split_https_url(url) -> Result<UrlParts>
fn tls_connector() -> Result<TlsConnector>
fn get(url, headers, timeout, max_body_bytes) -> Result<HttpsResponse> — `GET url` com os cabeçalhos dados.
fn get_inner(url, headers, max_body_bytes) -> Result<HttpsResponse>
(6 testes)

### log_sink.rs — Destino das linhas de log de build/job: o motor de containers escreve aqui sem saber que existe banc…
trait LogSink
  fn append_build_line(deployment_id, line, timestamp)
  fn append_job_line(job_run_id, stream, line, timestamp)
impl LogSink for std::sync::Arc<L>: append_build_line, append_job_line

### mod.rs — Infraestrutura transversal (barramento de eventos, segredos, health checks), sem regra de negócio.

### secrets.rs — `SecretsManager`: secrets por projeto cifrados no banco com a chave mestra do daemon.
struct SecretsManager { passphrase, db }
impl SecretsManager
  fn new(master_key_path, db) -> Result<Self>
  fn encrypt(plaintext) -> Result<String>
  fn decrypt(ciphertext_hex) -> Result<String>
  fn get_raw(project_id, name) -> Result<String>
  fn set(project_id, name, value) -> Result<()>
  fn delete(project_id, name) -> Result<()>
  fn list_names(project_id) -> Result<Vec<String>>
fn generate_key() -> Result<String>

### ttl_cache.rs — Cache de um valor só, com TTL, que junta chamadas concorrentes numa só ida à fonte lenta.
struct TtlCache { ttl, slot } — Single-slot value cache with a TTL.
impl TtlCache<T>
  fn new(ttl) -> Self
  fn get_or_refresh(refresh) -> Result<T, E> — Returns the cached value if still within the TTL, otherwise runs `refresh`, stores and returns it.
  fn invalidate() — Drops the cached value so the next `get_or_refresh` fetches fresh.
