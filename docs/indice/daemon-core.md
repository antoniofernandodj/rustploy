# Índice: daemon: main, bins, event bus, secrets, métricas e o resto

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

## rustploy-daemon/crates/daemon/src/

### env_backup.rs — Backup periódico das env vars de projetos e serviços em snapshots JSON, com listagem, restauração e …
struct EnvSnapshot { created_at, projects, services } — Conteúdo de um snapshot: todos os projectos e serviços com as suas env vars.
struct ProjectEnvEntry { id, name, env_vars, env_comments }
struct ServiceEnvEntry { id, name, project_id, env_vars }
fn backup_loop(db, backup_dir, interval_secs)
fn write_snapshot(db, backup_dir) -> anyhow::Result<()>
fn collect_snapshot(db) -> anyhow::Result<EnvSnapshot>
fn list_snapshots(backup_dir) -> anyhow::Result<Vec<String>>
fn restore_snapshot(db, backup_dir, snapshot) -> anyhow::Result<usize>
fn cleanup_old(backup_dir) -> anyhow::Result<()>
const _

### env_switch.rs — Troca de uma env var de conexão (`DATABASE_URL`…) num projeto, com o valor antigo guardado para roll…
struct EnvChange { service_id, key, previous } — Onde a chave estava e o que valia (`None` = não existia ali).
fn is_defined(db, project_id, key) -> Result<bool> — A chave já está definida no projeto ou em algum serviço dele?
fn switch(db, project_id, key, value) -> Result<Vec<EnvChange>> — Grava `value` (Plain) em todo lugar onde `key` existe; se em nenhum, no projeto.
fn restore(db, project_id, changes) -> Result<()> — Desfaz [`switch`]: devolve o valor antigo (ou remove a chave criada).

### event_bus.rs — Bus de eventos em memória (broadcast): o que alimenta o SSE `/api/events` dos clientes.
struct EventBus { sender }
impl EventBus
  fn new() -> Self
  fn publish(event) — Publica um evento no bus, para os assinantes conectados no momento.
  fn subscribe() -> broadcast::Receiver<Event>
const CHANNEL_CAPACITY
impl Default for EventBus

### firewall.rs — Cliente do helper privilegiado de firewall (`rustployd-fw`).
const TIMEOUT — `ufw` normalmente responde em milissegundos; o timeout largo cobre o primeiro start do helper via so…
struct FwResponse { ok, backend, error }
fn socket_path() -> String
fn ensure_allowed(port) -> Result<String, String> — Libera `port`/tcp no firewall do host.
fn ensure_denied(port) -> Result<String, String> — Remove a liberação de `port`/tcp.
fn ensure_allowed_bg(port) — Variante fire-and-forget para handlers de RPC: não atrasa a resposta ao cliente; o resultado vai par…
fn ensure_denied_bg(port)
fn request(op, port) -> Result<String, String>
fn do_request(op, port) -> Result<String, String>
const DEFAULT_SOCKET

### health.rs — Checagens de saúde HTTP e TCP usadas pelo healthcheck do deploy e pelo watchdog.
fn check_http(url, expected, timeout) -> bool
fn check_tcp(addr, timeout) -> bool

### logs.rs — Streaming dos logs dos containers gerenciados para o event bus, reconectando e encerrando streams pa…
fn stream_loop(docker, db, bus)
fn stream_container(docker, container_id, service_id, bus) — Transmite os logs de um container para o event bus até o stream acabar, ou até ficar parado por `STR…
fn publish_log(bus, service_id, container_id, is_stderr, bytes)
const STREAM_IDLE_TIMEOUT

### main.rs — Ponto de entrada do `rustployd`: resolve config e diretórios, sobe banco, Docker, ingress, API e reg…
fn main() -> Result<()>
fn init_logging(level)
fn fallback_dir() -> PathBuf
fn resolve_data_path(configured) -> PathBuf — Tries to use `configured` as the data directory.
fn resolve_master_key_path(configured) -> PathBuf — Tries to use `configured` as the master key path.
fn can_write_dir(dir) -> bool — Returns true only when `dir` (or its path) is both creatable and writable.
const GLOBAL

### metrics.rs — Coleta periódica de métricas de CPU/memória/disco do host e por container, publicadas no event bus.
fn collect_loop(docker, db, bus, interval_secs)
fn collect_system_metrics(sys) -> SystemMetricsPoint
fn collect_container_metrics(docker, container_id, service_id, _prev_cpu) -> anyhow::Result<ContainerMetricsPoint>

### migration.rs — Assistente de migração: banco antigo (serviço Compose do projeto) → database gerenciado de um servid…
fn new_record(req, project_id, env_var) -> Record
fn valid_source_db(name) -> bool — Nome de database de **origem**: vai para scripts shell/SQL, então só o conjunto seguro (o de destino…
fn step(state, rec, i, st, detail)
fn log(state, rec, text)
fn pipe_script(e) -> &'static str — Script `bash` do pipe dump|restore.
fn counts_plan(e, db) -> (String, String) — `(sh, stdin)` que imprime `tabela|contagem` do database `db` (um por linha, ordenado), pelo cliente …
struct Side { host, port, user, pass }
fn source_side(e, svc, host, env) -> Result<Side> — Credenciais de administração do banco **antigo**, das env vars resolvidas.
fn dest_image(server) -> Result<String> — Imagem (e tag) do servidor de destino: a mesma traz as ferramentas na versão certa.
fn image_version(image) -> Option<(u32, u32)> — Versão maior/menor de uma tag de imagem (`postgres:18` → (18, 0)); `None` se não der para ler (`late…
fn source_image(svc) -> Option<String>
fn dependents(state, source, host, env_var) -> Result<Vec<Service>> — Serviços do projeto que usam o banco antigo: os rodando que referenciam a env var da conexão ou o ho…
fn docker_run_pipe(engine, image, network, origem, origem_db, destino, destino_db) -> Result<String>
fn run(state, rec) — Roda a migração inteira.
fn run_inner(state, rec, cur) -> Result<()>
fn undo(state, rec, source_stopped) -> Result<()> — Volta tudo: env var antiga, banco antigo de pé (se foi parado) e serviços parados de novo no ar.
const STEPS
(3 testes)

### ports.rs — Alocação automática de portas externas (`ServiceSpec.host_port`).
const AUTO_PORT — Sentinela em `ServiceSpec.host_port` que pede alocação automática.
fn resolve_host_port(db, spec, exclude_id) -> Result<(), String> — Resolve o `host_port` do spec antes de persistir: - `Some(0)` → aloca uma porta livre da faixa confi…
fn port_in_use_by_other(db, port, exclude_id) -> bool — True se `port` ainda é reservada por algum serviço ≠ `exclude_id` (ou pelo próprio daemon) — usado a…
fn used_ports(db, exclude_id) -> Result<Vec<u16>, String> — Portas de host indisponíveis: `host_port` de todos os serviços (exceto `exclude_id`) + portas do pró…
fn allocate(used) -> Result<u16, String> — Varre a faixa configurada e devolve a primeira porta que (a) nenhum serviço reserva e (b) nenhum pro…
(2 testes)

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

### shared_db.rs — Provisionamento de database + usuário dentro de um servidor de banco compartilhado, por motor, via `…
enum Engine { Postgres, MySql, MariaDb, Mongo }
impl Engine
  fn from_kind(kind) -> Option<Self>
  fn default_env_var() -> &'static str — Nome da env var padrão da connection string no projeto consumidor.
  fn kind_id() -> &'static str — `db_kind` que `shared::connection` entende.
  fn admin_sh_pub() -> &'static str
  fn admin_sh() -> &'static str — Script `sh` que abre o cliente de administração (lê o SQL/JS do stdin).
fn validate_name(name) -> Result<()> — Nome de database/usuário: `[a-z][a-z0-9_]*`, até 32 chars (limite do usuário no MySQL).
fn generate_password() -> Result<String> — Senha aleatória alfanumérica (segura em SQL, JS e URL sem escapes).
fn create_script(e, name, pass, conn_limit, stmt_timeout_ms) -> String — Script de criação: database, usuário com acesso só a ele e limites.
fn drop_script(e, name) -> String — Script de remoção (**apaga os dados**).
fn wait_ready(e, container, tries) -> Result<()> — Espera o administrador do servidor responder.
fn exec(e, container, script) -> Result<String> — Roda `script` no cliente de administração dentro de `container`.
fn exec_sh(e, container, sh, script) -> Result<String> — Como [`exec`], com o comando `sh -c` escolhido por quem chama.
(3 testes)

### watchdog.rs — Watchdog dos serviços no ar: checa se o container roda e passa no healthcheck, reinicia com limite d…
struct ServiceState { last_check, consecutive_failures, restart_attempts }
fn watchdog_loop(state) — Laço do watchdog: a cada tick, para cada serviço no ar, confere se o container roda e passa no healt…
fn try_restart(state, svc, container_id, svc_state) — Tenta religar o container de um serviço caído, até `MAX_RESTART_ATTEMPTS`; se o container foi removi…
fn is_not_found_error(e) -> bool
fn trigger_redeploy(state, svc)
fn run_healthcheck(hc, container_id, docker, port, timeout) -> bool
fn get_container_ip(docker, container_id) -> Option<String>
fn container_is_running(docker, container_id) -> bool
fn mark_service(db, bus, service_id, status, container_id)
const BASE_TICK, MAX_RESTART_ATTEMPTS, RESTART_WAIT
