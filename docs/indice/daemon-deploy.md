# Índice: daemon: deploy, jobs e manutenção

> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## crates/daemon/src/deploy/

### env_resolve.rs — Resolução de env vars com secrets decifradas — extraído de `DeployExecutor::resolve_env` pra ser rea…
fn resolve_project_env(db, secrets, project_id) -> Result<HashMap<String, String>> — Só as env vars do projeto (base), secrets decifradas — sem nenhuma específica de serviço.
fn resolve(db, secrets, svc) -> Result<Vec<(String, String)>> — Funde env vars do projeto (base) com as do serviço (sobrescreve por chave), decifrando `EnvVarValue:…
fn resolve_project_only(db, secrets, project_id) -> Result<Vec<(String, String)>> — Só as env vars do projeto — usado por jobs sem serviço gatilho (`Job::trigger_service_id: None`, job…
fn resolve_job(db, secrets, job) -> Result<Vec<(String, String)>> — Env vars completas de um `Job`: base (projeto, + serviço gatilho quando houver) por baixo, `job.env_…

### executor.rs
fn copy_dir_all(src, dst) -> Result<()>
struct DeployExecutor { db, docker, ingress, bus, secrets, tls, db_path, drain_secs, registry_internal_token }
fn is_embedded_registry_image(image, port, domain) -> bool — Reconhece se `image` aponta para o registry Docker embutido do próprio rustployd, seja por loopback …
fn failure_log_lines(state_label, err) -> Vec<String> — Quebra a causa de um step falho nas linhas que vão para o `build_log`.
fn missing_dockerfile_msg(source, dockerfile_path) -> String — Mensagem de "Dockerfile não encontrado", especializada por fonte.
fn rollback_cause(dep) -> String — Texto que vai para `ServiceStatus::Error` quando um deploy cai em rollback.
fn truncate_chars(s, max) -> String — Trunca por CARACTERE (não por byte): cortar um `&str` com índice de byte no meio de um multibyte ent…
impl DeployExecutor
  fn run(deployment_id)
  fn execute(deployment_id) -> Result<()>
  fn step(dep, svc) -> Result<DeployState>
  fn poll_healthcheck(ip, container_id, svc, dep) -> Result<()>
  fn clone_dir(deployment_id) -> PathBuf
  fn short(id) -> &'a str
  fn ensure_live_names_free(svc) -> Result<()> — Falha o deploy se algum nome live que ele vai ocupar (`rp_<id8>_<safe>`, `…_r<i>`) já for de um cont…
  fn describe_service(service_id) -> String — "o serviço `api` do projeto `loja`", para mensagens de erro.
  fn ensure_firewall(deployment_id, service_id, host_port) — Garante a liberação da porta externa no firewall do host (helper `rustployd-fw`) e registra o result…
  fn log_step(deployment_id, service_id, line) — Persiste uma linha de log de build no banco e a publica no event bus.
  fn registry_credentials_for(image) -> Option<bollard::auth::DockerCredentials> — Se `image` aponta pro registry embutido do próprio rustployd, devolve as credenciais do token intern…
  fn image_for(dep, svc) -> String
  fn archive_dir(service_id, archive_id) -> PathBuf
  fn network_name(project_id) -> String
  fn ensure_network(project_id) -> Result<String>
  fn resolve_env(svc) -> Result<Vec<(String, String)>> — Wrapper fino: a lógica de verdade mora em `deploy::env_resolve::resolve` (reaproveitada pelo `JobRun…
  fn load_deployment(id) -> Result<Deployment>
  fn load_service(id) -> Result<Option<Service>>
  fn transition(deployment_id, from, to, message) -> Result<()>
(20 testes)

### git.rs
struct CloneOptions { url, branch, token, username, dir }
struct CloneProgress { _phase, percent, description }
fn clone(opts, on_progress) -> Result<()> — Clone a git repository into `opts.dir`, calling `on_progress` for each stderr line emitted by git.
fn resolve_clone_credentials(db, secrets, provider_id, credentials, manual_username, project_id) -> (Option<String>, Option<String>) — Resolve o token + username de clone a partir de `provider_id` (conta git conectada — tem precedência…
fn inject_token(url, token, username) -> String — Embeds a personal-access token into an HTTPS URL.
fn redact_url(url) -> String — Returns the URL with any embedded credentials replaced by `***`.
fn parse_progress(line) -> CloneProgress — Maps a raw git stderr line to a `CloneProgress` value.
fn extract_percent(line) -> u8 — Extracts the integer percentage from a git progress line.

### mod.rs

### queue.rs — Fila **global** de deploys: no máximo um deploy rodando por vez no daemon.
struct QueueInner { queued, running, paused }
struct DeployQueue { inner, notify } — Handle compartilhado da fila (fica no `AppState`).
impl DeployQueue
  fn new() -> Arc<Self>
  fn enqueue(dep_id) — Enfileira um deployment e acorda o worker.
  fn remove_queued(dep_id) -> bool — Remove um deployment **enfileirado** (não afeta o que está rodando — abortar o running é papel do `a…
  fn promote(dep_id) — Move um enfileirado para o início da fila ("furar fila").
  fn reorder(order) — Reordena a fila para a ordem dada.
  fn set_paused(paused) — Pausa/retoma a fila.
  fn snapshot() -> (Option<String>, Vec<String>, bool) — Snapshot para o handler de status: `(running, queued_em_ordem, paused)`.
  fn take_next() -> Option<String> — Tira o próximo da fila e marca como running.
  fn clear_running()
  fn wait()
fn run_worker(state) — Worker único da fila global.
fn run_one(state, dep_id) — Roda um deployment: marca o serviço como `Deploying`, spawna o executor como task (guardando o `Abor…
(5 testes)

### recovery.rs
fn recover(db, docker, ingress, bus, secrets, tls, db_path, drain_secs, registry_internal_token) -> Vec<String>
fn reconcile(db, docker, ingress, tls) — Reconciles every service's DB status against actual Docker container state.
fn compose_ingress_ip(docker, svc, net) -> Option<String> — IP do container que atende o ingress numa stack Compose, ou `None` quando a stack não está no ar.
fn reconcile_routes(svc, ips, ingress, tls)
fn restore_routes(db, docker, ingress, tls)

## crates/daemon/src/jobs/

### mod.rs

### runner.rs — Execução de um `Job` (tarefa one-shot via docker-compose): resolve rede + env vars do serviço gatilh…
struct JobRunner { db, docker, bus, secrets, db_path, registry_internal_token }
fn spawn(state, job) -> Result<JobRun> — Cria o `job_run` e dispara a execução em background (`tokio::spawn`) — usado tanto pelo `scheduler_l…
impl JobRunner
  fn run(job, run_id, cancel_rx)
  fn run_inner_mirrored(job, run_id, mirror_deployment, cancel_rx) -> Result<i32> — Roda `job` até o `main_service` terminar e devolve o exit code — sem tocar em `job.recurrence`/`resc…
  fn reschedule(job)

### scheduler.rs — Ticker de agendamento dos jobs one-shot — mesmo formato de `metrics.rs`/ `env_backup.rs`: `tokio::ti…
fn scheduler_loop(state)
const TICK_SECS

## crates/daemon/src/maintenance/

### mod.rs — Limpeza automática (agendada) de recursos Docker não usados — ver `docs/plano-limpeza-automatica-doc…

### run.rs — Execução de uma limpeza (agendada ou "Executar agora"): roda os recursos marcados em `DockerCleanupC…
fn run(state, config) -> DockerCleanupConfig — Ordem importa: containers primeiro (libera imagens/redes em uso por eles), depois imagens, depois vo…
fn record(resource, result) -> DockerCleanupResourceResult
fn save_config(state, config) -> anyhow::Result<()>
fn load_config(state) -> anyhow::Result<DockerCleanupConfig>
fn load_last_run(state) -> Option<DockerCleanupLastRun>

### scheduler.rs — Ticker que verifica se a limpeza automática de Docker está devida — mesmo idioma de `jobs::scheduler…
fn scheduler_loop(state)
const TICK_SECS
