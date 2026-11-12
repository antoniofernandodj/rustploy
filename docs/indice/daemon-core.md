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

### rustploy-cli.rs — `rustploy-cli`: ferramentas de linha de comando que **não precisam do daemon rodando**.
fn main()
fn run(args) -> Result<String>
fn keygen() -> String
struct Opts { recipients, identity_file, config, positional } — Opções comuns, tiradas dos argumentos (`-r X`, `--recipient X`, `--recipient=X`…).
fn parse_opts(args) -> Result<Opts>
fn read_secrets_section(path) -> Result<(Option<String>, Option<String>)> — Lê `[secrets]` do config.toml: `(age_identity, recovery_recipient)`.
fn config_path(o) -> PathBuf
fn resolve_recipients(o) -> Result<Vec<x25519::Recipient>> — Os destinatários: os de `--recipient`, ou os do config (chave pública do servidor + recuperação).
fn resolve_identity(o) -> Result<x25519::Identity>
fn read_secret_value() -> Result<String> — O valor sensível: do stdin; se for um terminal, pede (sem eco, quando o `stty` existe).
fn secret_encrypt(args) -> Result<String>
fn secret_decrypt(args) -> Result<String>
const DEFAULT_CONFIG, USAGE
(6 testes)

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

## rustploy-daemon/crates/daemon/src/bin/rustployd-update/

### main.rs — `rustployd-update` — o helper root que instala pacotes **assinados** do rustploy.
fn listener() -> (UnixListener, bool) — Socket herdado do systemd (LISTEN_FDS, fd 3) ou, em desenvolvimento, criado em `$RUSTPLOY_UPDATE_SOC…
fn trusted_keys() -> Vec<String> — As chaves em que este helper confia: as de produção embutidas em `update-core`.
fn directory(variable, default, development) -> PathBuf
fn main()
fn serve(stream, updater)
fn handle_line(updater, line, reply) — Um pedido.
const ALLOWED_DIR, STATE_DIR, MAX_DEB_BYTES, MAX_REQUEST_BYTES
(4 testes)

### protocol.rs — O protocolo do socket: uma linha JSON por pedido, uma de resposta.
enum Request { Install, Rollback }
struct Response { ok, version, error }
impl Response
  fn accepted(version) -> Self
  fn failed(error) -> Self
  fn to_line() -> String
(3 testes)

## rustploy-daemon/crates/daemon/src/bootstrap/

### config.rs — Resolução de diretórios (banco, chave age) com fallback para o home e inicialização do log.
fn init_logging(level)
fn fallback_dir() -> PathBuf
fn resolve_data_path(configured) -> PathBuf — Tries to use `configured` as the data directory.
fn resolve_age_key_path(cfg) -> PathBuf — Onde fica o arquivo da identidade `age` do servidor: `age_key_path`; senão `/etc/rustploy/age.key`.
fn can_write_dir(dir) -> bool — Returns true only when `dir` (or its path) is both creatable and writable.
(2 testes)

### listeners.rs — Listeners do daemon: registry OCI embutido (storage e rota), proxy de ingress e API HTTP.
fn prepare_registry_storage(config, db_path) -> Option<Arc<RegistryStorage>> — Storage do registry OCI embutido, criado ANTES do `AppState`: o listener HTTP do registry, o handler…
fn publish_registry_route(config, db, ingress, tls) — Domínio público do registry — banco tem precedência sobre o config (mesmo padrão do e-mail ACME).
fn spawn(config, state, ingress, tls, registry_storage) — Sobe o proxy de ingress, a API HTTP/JSON + SSE e o listener do registry.

### mod.rs — Montagem do `rustployd`: config e diretórios, banco, Docker, ingress, `AppState`, recuperação de boo…
fn run() -> Result<()> — Sobe o daemon inteiro e segura o processo (encerramento é o sinal do systemd).

### state.rs — Estado compartilhado do daemon (`AppState`) e os caches do inventário Docker.
type OAuthStates = Arc<Mutex<HashMap<String, String>>> — Pending OAuth handshakes: CSRF `state` → `provider_id`, consumed by the `/oauth/gitea/callback` rout…
struct AppState { db, docker, ingress, bus, secrets, tls, db_path, backup_dir, drain_secs, api, started_at, oauth_states, active_deploys, active_jobs, warming_jobs, warming_services, deploy_queue, docker_cache, registry_storage, registry_internal_token, update_checker, update_applier, state_git }
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

### workers.rs — Tarefas de fundo do daemon: backup de env, métricas, logs, GC do registry, watchdog, agendadores, co…
fn spawn_standalone(config, db, docker, bus, backup_dir) — Laços que só precisam de banco, Docker e barramento (sobem antes do `AppState`).
fn spawn_stateful(state, registry_storage, tls) — Laços que dependem do `AppState` já montado.

## rustploy-daemon/crates/daemon/src/platform/

### age_format.rs — O formato `ENC[age:<base64>]` dos segredos, **sem nenhuma dependência do daemon** (nada de banco, co…
fn is_enc(value) -> bool — O valor está no formato `ENC[age:…]`?
fn encrypt(plaintext, recipients) -> Result<String> — Cifra para todos os `recipients` (a chave do servidor e, se houver, a de recuperação).
fn decrypt(value, identity) -> Result<String> — Decifra um `ENC[age:…]` com a identidade.
const AGE_MAGIC — Cabeçalho de todo arquivo age (binário).
fn is_age_file(path) -> bool — O arquivo `path` é um arquivo age? (Só olha os primeiros bytes.)
fn encrypt_file(src, dst, recipients) -> Result<()> — Cifra o arquivo `src` em `dst` (em fluxo: não carrega o arquivo na memória).
fn decrypt_file(src, dst, identity) -> Result<()> — Decifra o arquivo age `src` em `dst` com a identidade (em fluxo).
fn parse_identity(text) -> Result<x25519::Identity>
fn parse_recipient(text) -> Result<x25519::Recipient>
const ENC_PREFIX
(4 testes)

### config.rs — Configuração do daemon (`config.toml`): structs de cada seção com defaults e o singleton `CONFIG`.
const CONFIG — Process-wide configuration singleton.
struct EnvBackupConfig { dir, interval_secs }
fn default_env_backup_interval() -> u64
struct RustployConfig { daemon, ingress, docker, deploy, metrics, secrets, api, env_backup, external_ports, registry, update, state_git, logs }
struct LogsConfig { enabled, retention_days, max_bytes_per_service } — `[logs]`: persistência do log de runtime dos containers (`platform::log_store`).
fn default_logs_enabled() -> bool
fn default_logs_retention_days() -> u32
fn default_logs_max_bytes() -> u64
struct StateGitConfig { fetch_interval_secs } — `[state_git]`: só o ritmo do `fetch`.
fn default_fetch_interval_secs() -> u64
struct UpdateConfig { enabled, repo, cache_hours, check_interval_hours, allow_agent_apply } — Verificação de versão nova (`Command::CheckUpdate`).
fn default_update_enabled() -> bool
fn default_update_repo() -> String
fn default_update_cache_hours() -> u64
fn default_update_check_interval_hours() -> u64
struct ExternalPortsConfig { range_start, range_end } — Faixa de portas de host que o rustploy pode alocar automaticamente para exposição externa de serviço…
fn default_external_range_start() -> u16
fn default_external_range_end() -> u16
impl ExternalPortsConfig
  fn contains(port) -> bool
struct ApiConfig { enabled, bind_address, port, token, master_token, visitor_level, domain, max_connections } — Configuration for the HTTP/JSON + SSE control API — o canal administrativo remoto.
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
fn default_db_path() -> String
fn default_log_level() -> String
struct IngressConfig { http_port, https_port, bind_address, acme }
struct AcmeConfig { enabled, email, directory }
struct DockerConfig { socket_path }
struct DeployConfig { drain_secs, image_cache }
struct MetricsConfig { interval_secs, history_points }
struct SecretsConfig { age_key_path, age_identity, recovery_recipient }
impl RustployConfig
  fn global() -> &'static RustployConfig — Returns the process-wide config, loading it on first call.
  fn global_path() -> Option<&'static Path> — Arquivo de onde `global()` leu a config (`None` = defaults).
  fn load() -> Self
  fn load_with_path() -> (Self, Option<std::path::PathBuf>) — Como [`load`](Self::load), devolvendo também o arquivo de onde a config saiu (`None` = veio dos defa…
  fn apply_env_overrides(cfg) -> Self
fn user_home() -> Option<String> — The single place the process reads `$HOME`.
fn fallback_data_dir() -> PathBuf — `~/.local/share/rustploy`, falling back to `/tmp` when `$HOME` is unset.
fn dirs_config_path() -> Option<PathBuf>
fn outbound_ip() -> String — Detecta o IP de saída da máquina conectando um socket UDP em 8.8.8.8:80 (sem enviar dados) e lendo o…
const CONFIG_PATH
impl Default for ApiConfig
impl Default for DaemonConfig
impl Default for EnvBackupConfig
impl Default for ExternalPortsConfig
impl Default for LogsConfig
impl Default for RegistryConfig
impl Default for RustployConfig
impl Default for StateGitConfig
impl Default for UpdateConfig
(4 testes)

### connection.rs — Connection string de banco/broker, montada num lugar só.
fn pct_encode(s) -> String — Percent-encoding de um componente de userinfo (`@`, `:`, `/`, `?`, `#`…).
fn scheme(product) -> Option<&'static str> — Esquema de URI por `product`; `None` = sem esquema (Kafka, serviço comum).
struct ConnTarget { host, port, database, user, password, auth_source } — Destino de uma conexão.
fn non_empty(s) -> Option<&str>
fn connection_url(product, t) -> String — URI padrão (`scheme://user:senha@host:porta/database`) que os drivers aceitam como `DATABASE_URL`.
fn credentials(product, get) — `(database, usuário, senha, authSource)` de um banco/broker, lidos das env vars nas convenções que o…
fn compose_services(content) -> Vec<(String, Option<String>)> — Serviços declarados num YAML de Compose: `(chave, imagem)`, na ordem.
fn compose_host(content, ingress_service, product) -> Option<String> — Host (chave do serviço no YAML) que recebe a conexão numa stack Compose: `ingress_service` se preenc…
fn is_sql_or_mongo(product) -> bool — Bancos que podem ser servidor compartilhado / ter databases gerenciados.
fn validate_shared_rules(name, source, product, shared) -> Result<(), String> — Regras do spec ligadas ao banco compartilhado, checadas onde o spec é gravado: (1) nenhum nome de se…
(6 testes)

### env_validation.rs — Validação das env vars gravadas por serviço, projeto e job: nome válido e sem chave repetida.
fn valid_key(key) -> bool — Nome aceito: começa com letra ou `_`; o resto são letras, dígitos, `_`, `.` e `-` (`.`/`-` aparecem …
fn validate(env_vars) -> Option<String> — Primeira violação, já em texto para o usuário; `None` se está tudo certo.
(3 testes)

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
fn open(url, headers) — Conecta, faz o `GET` e devolve status, cabeçalhos e o corpo **ainda por ler**.
fn host_allowed(host, allowed) -> bool — `host` é um dos `allowed` ou um subdomínio de algum (`objects.githubusercontent.com` casa com `githu…
fn redirect_target(status, headers) -> Result<Option<String>> — Para onde seguir depois desta resposta: `None` se não é redirecionamento.
fn download(url, headers, destination, max_bytes, timeout, allowed_hosts) -> Result<()> — Baixa `url` para `destination`, seguindo até [`MAX_REDIRECTS`] redirecionamentos, **somente para hos…
fn download_inner(url, headers, partial, max_bytes, allowed_hosts) -> Result<()>
const MAX_REDIRECTS
(9 testes)

### log_sink.rs — Destino das linhas de log de build/job: o motor de containers escreve aqui sem saber que existe banc…
trait LogSink
  fn append_build_line(deployment_id, line, timestamp)
  fn append_job_line(job_run_id, stream, line, timestamp)
impl LogSink for std::sync::Arc<L>: append_build_line, append_job_line

### log_store.rs — Armazenamento do **log de runtime** dos containers (plano: `docs/plano-log-de-runtime-persistido.md`…
const SEGMENT_BYTES — Tamanho a partir do qual o segmento atual fecha e começa o próximo.
const FLUSH_LINES — Descarrega o buffer em disco a cada tanto de linhas, ou a cada [`FLUSH_EVERY`].
const PRUNE_EVERY — De quanto em quanto tempo o escritor reaplica a retenção por idade em todos os serviços.
const TAIL_SCAN_BYTES — Quanto do fim do segmento é lido para descobrir o último instante de um container.
fn global() -> Option<Arc<LogStore>> — O armazenamento global, se foi iniciado.
fn init(root, cfg) -> Option<Arc<LogStore>> — Inicia o armazenamento (uma vez) e o instala como global.
struct Record { t, c, s, m }
struct ExecMeta { container, deployment_id, image, first_ts, last_ts, lines, bytes, partial } — O que se sabe de uma execução (um container) — o `meta.json` da pasta dela.
enum Msg { Line, Exec, Flush, Delete, Retain, DeleteExecs }
struct LogStore { root, tx }
fn short_id(container_id) -> String — Id curto (12 caracteres) usado no arquivo e na API.
impl LogStore
  fn open(root, cfg) -> Self
  fn append(service_id, container_id, stderr, at, text) — Enfileira uma linha para gravar.
  fn set_exec(service_id, container_id, deployment_id, image) — Registra a que deploy e a que imagem pertence a execução (container).
  fn flush() — Espera o escritor gravar tudo o que já foi enfileirado.
  fn delete_service(service_id) — Apaga o histórico de um serviço (serviço removido).
  fn retain_services(ids) — Apaga as pastas de serviços que não existem mais.
  fn last_timestamp(service_id, container_id) -> Option<DateTime<Utc>> — Último instante gravado para esse container (ou `None`).
  fn read(service_id, before, limit, containers) — Até `limit` linhas anteriores a `before` (ou as mais recentes), em ordem cronológica, mais o instant…
  fn executions(service_id) -> Vec<ExecMeta> — As execuções guardadas do serviço, da mais nova para a mais velha.
  fn delete_executions(service_id, containers) -> (u32, u64) — Apaga execuções (pastas inteiras).
fn segment_name(idx) -> String
fn segments(dir) -> Vec<(u32, PathBuf)> — Segmentos da pasta, `(índice, caminho)` em ordem crescente.
fn exec_dirs(service_dir) -> Vec<(String, PathBuf)> — Pastas de execução de um serviço (subpastas; os segmentos legados, soltos, não contam).
fn dir_bytes(dir) -> u64
fn open_append(path) -> std::io::Result<File>
fn create_dir_private(dir) -> std::io::Result<()>
fn parse_record(line) -> Option<(DateTime<Utc>, Record)>
fn write_meta(dir, meta) — Grava o `meta.json` (escreve num temporário e renomeia, para nunca ficar pela metade).
fn open_truncate(path) -> std::io::Result<File>
fn load_meta(dir, container) -> ExecMeta — `meta.json` da pasta; sem ele (ou ilegível), reconstrói varrendo os segmentos.
fn derive_meta(dir, container) -> ExecMeta
fn list_execs(service_dir) -> Vec<ExecMeta>
fn scan_last_ts(exec_dir) -> Option<DateTime<Utc>> — Último instante gravado na pasta de uma execução (só olha o fim do segmento mais novo).
fn entry_of(at, r) -> LogHistoryEntry
fn read_exec_tail(dir, before, limit) -> (Vec<LogHistoryEntry>, bool) — Até `limit` linhas de **uma** pasta de execução, anteriores a `before`, mais novas primeiro.
fn read_history(service_dir, before, limit, containers)
fn remove_exec(dir) -> u64 — Apaga uma pasta de execução; devolve os bytes liberados.
fn prune_segments_to(dir, budget) -> bool — Só segmentos: apaga os mais velhos (menos o mais novo) até a pasta caber em `budget`.
fn prune_service(service_dir, cfg) -> (Vec<String>, Vec<String>) — Aplica a retenção a um serviço: apaga **execuções inteiras** por idade e por tamanho, da mais velha …
fn migrate_legacy(root) — Redistribui por container os segmentos que ficavam soltos na pasta do serviço (layout da v0.1.40), n…
struct MigWriter { dir, idx, out, size, meta }
impl MigWriter
  fn new(dir, container) -> std::io::Result<Self>
  fn push(at, rec) -> std::io::Result<()>
  fn finish() -> std::io::Result<()>
struct Open { dir, idx, out, size, last, pending, meta, meta_dirty }
struct Writer { root, cfg, open, deleted, deleted_execs }
impl Writer
  fn new(root, cfg) -> Self
  fn run(rx)
  fn delete_execs(service, containers) -> (u32, u64)
  fn retain(keep)
  fn prune_all()
  fn prune_one(service) — Retenção de um serviço, mantendo o estado em memória coerente com o disco.
  fn flush_all()
  fn open_exec(service, container) -> std::io::Result<Open> — Abre (ou cria) o segmento mais novo da execução para anexar.
  fn get_open(service, container) -> Option<&mut Open>
  fn exec(service, container, deployment_id, image)
  fn line(service, container, stderr, at, text)
const FLUSH_EVERY, META_FILE, GLOBAL
(16 testes)

### mod.rs — Infraestrutura transversal (barramento de eventos, segredos, health checks), sem regra de negócio.

### s3.rs — Cliente S3 mínimo, em hyper cru (o daemon não usa `reqwest`), para o backup dos dados dos serviços (…
type HmacSha256 = Hmac<Sha256>
const PART_SIZE — Tamanho de cada parte do multipart (o S3 exige no mínimo 5 MiB, exceto na última).
const SINGLE_PUT_MAX — Até este tamanho o objeto sobe num único PUT.
struct S3Config { endpoint, region, bucket, access_key, secret_key, path_style } — Destino S3.
struct Object { key, size, last_modified } — Um objeto da listagem.
impl S3Config
  fn validate() -> Result<()>
fn uri_encode(s, keep_slash) -> String — Codifica como o SigV4 pede (só `A-Z a-z 0-9 - _ .
fn hmac(key, data) -> Vec<u8>
fn sha256_hex(data) -> String
fn canonical_query(query) -> String — Query canônica: pares codificados e ordenados.
fn authorization(method, host, path, query, payload_hash, amz_date, region, access_key, secret_key) -> String — O cabeçalho `Authorization` de uma requisição.
struct Target { host_header, connect_host, port, tls, path }
fn target(cfg, key) -> Result<Target>
struct Reply { status, headers, body }
impl Reply
  fn header(name) -> Option<&str>
  fn text() -> Result<String>
fn send_once(cfg, method, key, query, body) -> Result<Reply>
fn exchange(io, request) -> Result<hyper::Response<Incoming>>
fn send(cfg, method, key, query, body) -> Result<Reply> — Envia com até [`ATTEMPTS`] tentativas em falha de rede ou 5xx (espera 1 s, 2 s).
fn s3_error(xml) -> String — `Code: Message` de um corpo de erro do S3.
fn expect_ok(reply, what) -> Result<Reply> — Confere o status e, se falhou, devolve o erro com a causa do S3.
fn xml_unescape(s) -> String
fn xml_escape(s) -> String
fn first_tag(xml, tag) -> Option<String> — Conteúdo da primeira `<tag>…</tag>`.
fn blocks(xml, tag) -> Vec<&'a str> — Os blocos `<tag>…</tag>` (conteúdo cru), na ordem.
fn read_full(f, buf) -> Result<usize> — Lê até `buf.len()` bytes (menos só no fim do arquivo).
fn put_file(cfg, key, path) -> Result<()> — Sobe `path` como o objeto `key`.
fn upload_parts(cfg, key, upload_id, file) -> Result<()>
fn get_to_file(cfg, key, dest) -> Result<u64> — Baixa `key` para `dest`.
fn list(cfg, prefix) -> Result<Vec<Object>> — Lista os objetos sob `prefix` (segue a paginação).
fn delete(cfg, key) -> Result<()> — Apaga `key` (apagar o que não existe também é sucesso, como no S3).
fn test(cfg, prefix) -> Result<()> — **Testar conexão**: grava um objeto pequeno, lê de volta, confere e apaga.
fn prefix_with_slash(prefix) -> String
const REQUEST_TIMEOUT, MAX_XML, ATTEMPTS
impl std::fmt::Debug for S3Config
(8 testes)

### secrets.rs — `SecretsManager`: cifra e decifra os segredos do daemon.
struct SecretsManager { identity, recovery, transient, config_path, db }
impl SecretsManager
  fn open(key_file, db, cfg, config_path) -> Result<Self> — Abre o gerenciador do daemon.
  fn new(db) -> Result<Self> — Para testes: identidade efêmera (some quando o objeto some), sem tocar na config.
  fn with_keys(db, identity, recovery) -> Result<Self> — Para testes: com uma identidade e uma recuperação escolhidas.
  fn has_recovery() -> bool — Há uma chave de recuperação (`[secrets] recovery_recipient`) configurada?
  fn recovery() -> Option<x25519::Recipient>
  fn public_key() -> String — A chave pública do servidor (`age1…`).
  fn encrypt(plaintext) -> Result<String> — Cifra para a identidade do servidor (e a de recuperação, se houver).
  fn encrypt_file(src, dst) -> Result<()> — Cifra um arquivo para a identidade do servidor (e a de recuperação, se houver).
  fn decrypt_file(src, dst) -> Result<()> — Decifra um arquivo age com a identidade deste servidor (ou, durante uma recuperação, com a identidad…
  fn decrypt(value) -> Result<String> — Decifra um valor `ENC[age:…]` com a identidade deste servidor (ou, durante uma recuperação, com a de…
  fn begin_recovery(secret_key) -> Result<String> — Começa uma recuperação: guarda a identidade **privada** de recuperação só na memória (para abrir o q…
  fn recovery_in_progress() -> bool — Há uma recuperação em curso (a identidade de recuperação está na memória)?
  fn end_recovery() — Termina a recuperação: descarta a identidade privada de recuperação da memória.
  fn rekey(value) -> Result<String> — Recifra um valor `ENC[age:…]` para a identidade **deste** servidor e a chave pública de recuperação …
  fn get_raw(project_id, name) -> Result<String> — Secret do **projeto**.
  fn get_scoped(project_id, scope, name) -> Result<String> — Secret de um escopo: serviço = só daquele serviço; job = só daquele job; projeto = só do projeto.
  fn resolve_for_job(project_id, job_id, trigger_service_id, name) -> Result<String> — Resolução de uma env `Secret(name)` **de um job**: o secret do **job** primeiro, depois o do serviço…
  fn resolve(project_id, service_id, name) -> Result<String> — Resolução de uma env `Secret(name)`: o escopo do **serviço** primeiro, depois o do projeto.
  fn set(project_id, name, value) -> Result<()>
  fn set_scoped(project_id, scope, name, value) -> Result<()>
  fn delete_scoped(project_id, scope, name) -> Result<()>
  fn list_names(project_id) -> Result<Vec<String>>
  fn list_names_scoped(project_id, scope) -> Result<Vec<String>>
  fn reveal(project_id, scope, name) -> Result<String> — "Ver valor": decifra e **grava uma linha de auditoria** (quem/qual/quando — nunca o valor).
  fn move_secret(project_id, scope, name, op) -> Result<()> — As quatro movimentações manuais da R5, cada uma **atômica** (uma transação do repositório): ou o sec…
  fn rename_env_key(project_id, scope, from, to) -> Result<()> — Renomeia a **chave** de uma env do projeto (`service_id: None`) ou de um serviço, sem tocar no valor…
  fn set_env_value(project_id, scope, key, value) -> Result<()> — Troca só o **valor** de uma env do projeto (`service_id: None`) ou de um serviço, na mesma posição d…
  fn rename_secret(project_id, scope, from, to) -> Result<()> — Renomeia um **secret** (do projeto, ou de um serviço) e, na mesma transação, reaponta as envs `Secre…
fn valid_env_key(name) -> Result<()> — Nome válido para a chave de uma env: `[A-Za-z_][A-Za-z0-9_]*` (o que o Docker e o `.env` aceitam).
fn valid_secret_name(name) -> Result<()> — Nome válido para um secret: não vazio, só `[A-Za-z0-9_.-]` — fica de fora o que quebraria a referênc…
fn rename_in_list(list, from, to, onde) -> Result<()> — Renomeia o secret `from` para `to` na lista, recusando se `from` não existe ou `to` já existe.
fn repoint_secret_refs(vars, from, to) — Reaponta toda env `Secret(from)` para `Secret(to)`.
fn new_secret(name, encrypted) -> SecretInfra
fn project_mut(st, project_id) -> Result<&'a mut ProjectInfra>
fn service_mut(p, service_id) -> Result<&'a mut ServiceInfra>
fn job_mut(p, job_id) -> Result<&'a mut JobInfra>
fn secret_list_mut(st, project_id, scope) -> Result<&'a mut Vec<SecretInfra>> — A lista de secrets do escopo pedido (projeto, um serviço ou um job dele).
enum Scope { Project, Service, Job } — De quem é um secret/env: do projeto, de um serviço ou de um job.
impl Scope<'a>
  fn from_ids(service_id, job_id) -> Result<Self> — O escopo pedido por um `Command` (`service_id` e `job_id` são exclusivos).
fn take_plain_env(vars, key, onde) -> Result<String> — Tira o valor `Plain` da env `key` (erro claro se não existe ou já é um secret).
fn set_env(vars, key, value)
fn secret_into_env(vars, name, plain, onde) -> Result<()> — Secret → env: as envs que apontavam para o secret viram `Plain(valor)`; se nenhuma apontava, cria-se…
fn write_private(path, contents) -> Result<()> — Grava `contents` num arquivo que já nasce 0600 (sem janela em que ele exista legível).
fn parse_identity(text) -> Result<x25519::Identity>
fn has_enc_values(db) -> bool — Há algum segredo já no formato novo gravado no YAML?
fn persist_identity(identity, config_path, key_file) -> Result<()> — Grava a identidade recém-gerada no `config.toml` (seção `[secrets]`); se não der, no `age.key` (0600…
fn write_identity_into_config(path, line) -> Result<()> — Insere `line` logo abaixo de `[secrets]` (ou cria a seção no fim), preservando o resto do arquivo e …
fn set_secrets_key_in_config(path, key, value) -> Result<()> — Define `key = "value"` em `[secrets]` do `config.toml` (troca a linha se já existe, senão insere log…
impl From<Option<&'a str>> for Scope<'a>
(30 testes)

### ttl_cache.rs — Cache de um valor só, com TTL, que junta chamadas concorrentes numa só ida à fonte lenta.
struct TtlCache { ttl, slot } — Single-slot value cache with a TTL.
impl TtlCache<T>
  fn new(ttl) -> Self
  fn get_or_refresh(refresh) -> Result<T, E> — Returns the cached value if still within the TTL, otherwise runs `refresh`, stores and returns it.
  fn invalidate() — Drops the cached value so the next `get_or_refresh` fetches fresh.
