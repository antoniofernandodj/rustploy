# Índice: daemon: bancos, IaC, observabilidade, backup, templates e wizard

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-daemon/crates/daemon/src/features/

### mod.rs — Um diretório por assunto de produto; a camada `api` só adapta pedidos para estas funcionalidades.

### wizard.rs — Lógica do wizard "Novo serviço" (Application / Database / Broker / Storage / Compose /Template): cat…
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
fn generate_password(n) -> Result<String, String> — Senha aleatória alfanumérica (URL-safe) de `n` caracteres, vinda de `/dev/urandom`.
const PASSWORD_LEN — Tamanho das senhas geradas pelo wizard.
fn completar_senhas(f, precisa, precisa_root) -> Result<(), String> — Banco/broker novo nasce com senha forte mesmo que a pessoa não digite nada: campo vazio = "gere para…
fn db_rows_json() -> String — Passo "escolha o banco": inclui os flags de formulário (has_*) e o user default para o cliente pré-p…
fn broker_rows_json() -> String — Passo "escolha o broker".
fn templates_catalog_json(search) -> String — Templates filtrados por `search`, com as variáveis editáveis embutidas em cada linha (`vars: [{idx,l…
fn template_logo(t) -> (String, &'static str)
fn template_slug(t) -> String
struct DbFormInput { db_name, user, password, root_password, image, use_replica_sets } — Campos do formulário de banco/broker.
fn base_spec(name, project_id, source, port, env_vars, product) -> ServiceSpec
fn app_spec(name, project_id) -> ServiceSpec
fn compose_spec(name, project_id) -> ServiceSpec
fn db_spec(db, name, project_id, f) -> ServiceSpec
fn db_env_vars(db, f) -> Vec<EnvVar>
fn db_compose(db, svc, img, f) -> String
fn broker_spec(broker, name, project_id, f) -> ServiceSpec
fn broker_env_vars(broker, f) -> Vec<EnvVar>
fn broker_compose(broker, svc, img, _f) -> String
enum StorageKind { RustFs, MinIo } — Servidores de objetos S3: RustFS e MinIO.
impl StorageKind
  fn label() -> &'static str
  fn default_image() -> &'static str
  fn default_port() -> u16 — Porta da API S3 (a que as apps usam).
  fn console_port() -> u16 — Porta do console web.
  fn kind_id() -> &'static str
  fn from_str(s) -> Option<Self>
  fn default_user() -> &'static str — O usuário (access key) padrão, quando a pessoa não digita um.
  fn credential_keys() -> (&'static str, &'static str) — Nomes das duas envs de credencial: `(access key, secret key)`.
fn storage_rows_json() -> String — Passo "escolha o storage".
fn storage_spec(kind, name, project_id, f) -> ServiceSpec
fn storage_compose(kind, svc, img) -> String
fn template_spec(t, name, project_id, values) -> ServiceSpec
fn db_form(req) -> DbFormInput
fn effective_name(req) -> String — Nome efetivo: `app_name` tem precedência sobre `name` (ambos trimados).
fn build_spec(req) -> Result<ServiceSpec, String> — Monta o `ServiceSpec` a partir da requisição do wizard.
fn build_spec_inner(req) -> Result<ServiceSpec, String>
(5 testes)

## rustploy-daemon/crates/daemon/src/features/backup/

### mod.rs — Backup periódico das env vars de projetos e serviços em snapshots JSON, com listagem, restauração e …
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

## rustploy-daemon/crates/daemon/src/features/databases/

### context.rs — O que a migração de banco precisa do resto do daemon.
struct MigrationContext { db, docker, secrets, deploy }

### env_switch.rs — Troca de uma env var de conexão (`DATABASE_URL`…) num projeto, com o valor antigo guardado para roll…
fn is_defined(db, project_id, key) -> Result<bool> — A chave já está definida no projeto ou em algum serviço dele?
fn switch(db, project_id, key, value) -> Result<Vec<EnvChange>> — Grava `value` (Plain) em todo lugar onde `key` existe; se em nenhum, no projeto.
fn restore(db, project_id, changes) -> Result<()> — Desfaz [`switch`]: devolve o valor antigo (ou remove a chave criada).

### managed_view.rs — Visão de um banco gerenciado: monta a connection URL como o projeto consumidor a enxerga.
fn host_for(svc, project_id) -> String — Como o projeto consumidor enxerga o servidor: o alias global (outro projeto) ou a chave do YAML (o p…
fn to_view(svc, engine, r) -> ManagedDatabase

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

### mod.rs — Bancos de dados gerenciados: assistente de migração, acesso compartilhado e troca de variável de con…

### shared_db.rs — Provisionamento de database + usuário dentro de um servidor de banco compartilhado, por motor, via `…
enum Engine { Postgres, MySql, MariaDb, Mongo }
impl Engine
  fn from_kind(kind) -> Option<Self>
  fn default_env_var() -> &'static str — Nome da env var padrão da connection string no projeto consumidor.
  fn kind_id() -> &'static str — `product` que `crate::platform::connection` entende.
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

## rustploy-daemon/crates/daemon/src/features/iac/

### context.rs — O que o manifesto e o pacote de serviço (export/import) precisam do resto do daemon.
struct IacContext { db, secrets, api, services, deploy }
impl IacContext
  fn public_base_url() -> String — URL pública do daemon, sem barra final (base do webhook de cada serviço).

### manifest.rs — Infra-as-Code: structs do manifesto declarativo (`rustploy.yml`).
struct ServiceManifest { name, source, port, host_port, domain, tls, domains, env, env_comments, volumes, healthcheck, replicas, resources, command, args, product, shared }
struct SourceManifest { registry, git, compose, compose_ingress } — Origem do serviço: exatamente uma das três chaves deve estar presente.
struct GitManifest { url, branch, root_path, dockerfile, context, build_stage, submodules, watch_paths, username, credentials, provider }
struct HealthcheckManifest { kind, path, status, interval, timeout, retries, start_period }
struct ResourcesManifest { cpu_shares, mem }
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
fn env_map_to_vars(map) -> Vec<EnvVar>
fn env_vars_to_map(vars) -> BTreeMap<String, String>
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
(2 testes)

### mod.rs — Infraestrutura como código: manifesto de projeto/servidor e pacote de serviço (export/import).

### service_bundle.rs — Pacote de serviço: UM serviço (mais as variáveis do projeto que o usuário escolheu levar) num arquiv…
const BUNDLE_KIND — Valor de `kind` num pacote de serviço (o IaC do servidor não tem `kind`).
struct BundleOrigin { server, project, exported_at, daemon } — De onde o pacote saiu.
struct ServiceBundle { api_version, kind, origin, service, project_env } — O arquivo.
impl ServiceBundle
  fn from_service(svc, providers, project_env, include_values, origin) -> Result<Self, String> — Monta o pacote a partir de um serviço do banco.
  fn validate() -> Result<(), String> — Confere um pacote JÁ desserializado (o parse do YAML é de quem chama — o `rustploy-shared` não depen…
  fn project_env_vars() -> Vec<EnvVar> — Variáveis do projeto do pacote, já como `EnvVar` (para gravar no destino).
fn redact_values(map) — Todo valor `Plain` vira `${CHAVE}`; `secret:` passa intacto.
fn referenced_names(text, out) — Nomes de variáveis citados em `text` como `${NOME}`, `${NOME:-padrão}` ou `$NOME` (sintaxe de shell/…
fn suggested_project_vars(spec, project_keys) -> BTreeSet<String> — Quais variáveis DO PROJETO este serviço provavelmente usa: as citadas como `${VAR}`/`$VAR` nos valor…
(6 testes)

### service_export.rs — `Command::ServiceExportPlan` e `Command::ServiceExport`: o lado de cá de copiar um serviço para outr…
fn load(state, service_id) -> Result<(Service, Project), RpResponse> — Carrega o serviço e o projeto dele, ou já a resposta de erro.
fn plan_var(key, value, suggested) -> PlanVar
fn plan(state, service_id) -> RpResponse — O que dá para levar: as variáveis do serviço (todas vão) e as do projeto (o usuário marca; as citada…
fn export(state, service_id, include_values, project_env_keys) -> RpResponse — Gera o pacote.

### service_import.rs — `Command::ServiceImport`: cria UM serviço novo a partir de um [`crate::features::iac::service_bundle…
const MAX_YAML_BYTES — Um pacote maior que isto não é um pacote de serviço.
type Fail = (&'static str, String) — Erro de pedido: `(code, mensagem)` já no formato de `Response::err`.
fn warn_item(code, message) -> ImportWarning
struct Analysis { report, project, spec, project_env_writes, secrets_to_set } — Tudo o que a análise concluiu — o relatório e o necessário para aplicar.
fn is_placeholder(key, value) -> bool — `${CHAVE}` sob a PRÓPRIA chave é o marcador de "valor não exportado".
fn resolve_placeholders(map, provided) -> Vec<String> — Resolve os marcadores de `map` com `provided`; devolve as chaves sem valor.
fn secret_names(map) -> BTreeSet<String>
fn rename_compose_alias(content, old, new) -> String — Em um serviço Compose de banco/broker o nome do serviço interno é `rp_<nome>` (é o DNS da connection…
fn analyze(state, req) -> Result<Analysis, Fail>
fn apply(state, req, a) -> RpResponse — Grava o que a análise decidiu.
fn write_all(state, req, a) -> Result<(), RpResponse>
fn handle(state, req) -> RpResponse
const SECRET_PREFIX
(12 testes)

## rustploy-daemon/crates/daemon/src/features/observability/

### container_logs.rs — Streaming dos logs dos containers gerenciados para o event bus, reconectando e encerrando streams pa…
fn stream_loop(docker, db, bus)
fn stream_container(docker, container_id, service_id, bus) — Transmite os logs de um container para o event bus até o stream acabar, ou até ficar parado por `STR…
fn register_exec(docker, service_id, container_id) — Anota no armazenamento a que deploy e a que imagem pertence a execução (um `inspect_container`, feit…
fn last_persisted(service_id, container_id) -> Option<DateTime<Utc>> — Último instante já gravado desse container (se o armazenamento está ligado).
struct Stamper { prev, repeats } — Dá a cada linha de um stream um instante **único e determinístico**.
impl Stamper
  fn stamp(real) -> DateTime<Utc>
fn split_timestamp(raw) -> (DateTime<Utc>, &str) — Separa o prefixo de instante que o Docker põe com `timestamps: true` (`2026-…Z texto`).
fn publish_log(bus, stamper, service_id, container_id, is_stderr, bytes) — Uma saída do Docker pode trazer várias linhas: cada uma é gravada e publicada separadamente.
fn capture_final(docker, service_id, container_id) — Lê o que um container ainda tem e o grava, **antes** de ele ser removido (troca no deploy, rollback …
const STREAM_IDLE_TIMEOUT
(2 testes)

### context.rs — O que o watchdog precisa do resto do daemon.
struct WatchdogContext { db, docker, bus, deploy_queue }

### metrics.rs — Coleta periódica de métricas de CPU/memória/disco do host e por container, publicadas no event bus.
fn collect_loop(docker, db, bus, interval_secs)
fn collect_system_metrics(sys) -> SystemMetricsPoint
fn collect_container_metrics(docker, container_id, service_id, _prev_cpu) -> anyhow::Result<ContainerMetricsPoint>

### mod.rs — Observabilidade: watchdog dos serviços, métricas do host e transmissão de logs de container.

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

## rustploy-daemon/crates/daemon/src/features/state_git/

### git.rs — Invólucro fino do binário `git` do sistema (decisão da R9.0: nada de `git2`/`reqwest`).
struct Auth { username, token } — Credencial HTTPS (usuário + token) para falar com o remoto.
struct CommitInfo { sha, at, subject }
fn available() -> bool — O `git` existe e responde? Resultado guardado: o snapshot pergunta isto a cada atualização, e execut…
fn redact(text, auth) -> String — Troca o token (e a sua forma em base64, que é o que vai no cabeçalho) por `***`.
fn command(dir, auth) -> Command
fn run(dir, args, auth) -> Result<String> — Roda `git <args>` em `dir` e devolve o stdout.
fn validate_remote(url) -> Result<()> — Remoto aceito: `https://` sem credencial embutida, ou `file://`/caminho absoluto (cópia local e test…
fn strip_userinfo(url) -> String — Para mensagens de erro: nunca ecoa `usuario:senha@`.
fn ensure_repo(dir, branch, remote) -> Result<()> — Garante o repositório em `dir`: `git init`, branch certa, `.gitignore` e o remote `origin`.
fn head(dir) -> Result<Option<CommitInfo>> — O último commit, ou `None` se o repositório ainda não tem nenhum.
fn commit_all(dir, message) -> Result<Option<CommitInfo>> — `git add -A` e, se houver diferença, um commit.
fn push(dir, branch, auth) -> Result<()> — `git push origin <branch>`.
fn run_bytes(dir, args) -> Result<Vec<u8>> — Como [`run`], mas devolve os **bytes** do stdout (conteúdo de arquivo: não pode passar por UTF-8 com…
fn fetch(dir, branch, auth) -> Result<bool> — `git fetch` da branch para `refs/remotes/origin/<branch>`.
fn remote_head(dir, branch) -> Option<String> — O commit em `refs/remotes/origin/<branch>`, se existir.
fn ahead_behind(dir, branch) -> Result<(usize, usize)> — `(à frente, atrás)`: commits locais que o remoto não tem, e commits do remoto que o local não tem.
fn shares_history(dir, branch) -> bool — O histórico local e o de `refs/remotes/origin/<branch>` têm um ancestral comum? `false` também quand…
fn is_clean(dir) -> Result<bool> — Não há nada por commitar nem arquivo novo/alterado/apagado (fora o que o `.gitignore` esconde)?
fn files_at(dir, rev, keep) — Os arquivos do commit `rev`, **sem fazer checkout**: `caminho → bytes`.
fn backup_branch(dir, name, rev) -> Result<()> — `git branch <nome> <rev>`: guarda um commit (ex.: o histórico local que vai ser descartado).
fn push_backup_ref(dir, sha, name, auth) -> Result<()> — Copia `sha` para a branch `refs/heads/<name>` **do remoto** (um backup ao lado do que vai ser sobres…
fn push_force_with_lease(dir, branch, expected, auth) -> Result<()> — `push --force-with-lease`: sobrescreve a branch do remoto **só se ela ainda estiver em `expected`** …
fn reset_hard(dir, rev) -> Result<()> — Move a branch e a árvore de trabalho para `rev` (o estado já foi gravado, byte a byte igual).
const GITIGNORE
(7 testes)

### mod.rs — Sincronização do estado de infra com um repositório git (Fase 6 do plano `docs/plano-backup-estado-e…
const DEFAULT_FETCH_EVERY — Padrão do `fetch` nos testes (em produção vem de `[state_git] fetch_interval_secs`).
enum Mode { Off, Commit, Sync }
impl Mode
  fn as_str() -> &'static str
  fn parse(s) -> Option<Self>
struct Settings { mode, remote, branch, provider_id }
enum StateGitError { Plaintext, Invalid, Other }
enum Applied { Yes, Later, No } — O que aconteceu ao tentar aplicar um commit vindo de fora.
enum Reconciled { Push, Done }
struct RetryLater — O pull precisa esperar: há mudança local ainda não commitada.
struct GitStuck(String) — O estado foi aplicado, mas o `git reset` que alinha o repositório falhou.
struct Runtime { paused, incoming_error, last_fetch_at, ahead, behind, last_commit, last_push_ok, last_push_at, last_error, blocked, needs_push }
struct StateGit { db, secrets, rt, wake, cycle_lock, debounce, fetch_every, after_restore }
fn fingerprint(project, scope, what) -> String
fn blocking(state, allow) -> Vec<Finding> — Achados que **impedem** o commit: as certezas sempre, e as suspeitas que o usuário não liberou.
fn validate_incoming(root, new, allow, secrets, require_clean) -> anyhow::Result<()> — Valida uma árvore vinda de fora ANTES de aplicar qualquer coisa (R6, itens 1 e 2): sem valor em clar…
fn decifra_provider(secrets, name, v) -> anyhow::Result<()>
struct RemovedItem { id, kind, name, project, compose_project }
struct StateDiff { removed, spec_changed }
fn diff_states(old, new) -> StateDiff — O que um `pull` mudou de relevante para quem opera: o que sumiu e o que precisa de redeploy.
fn subject(paths, state) -> String — Assunto do commit a partir dos arquivos alterados (nomes legíveis, não ids).
impl StateGit
  fn new(db, secrets) -> Arc<Self>
  fn with_debounce(db, secrets, debounce) -> Arc<Self>
  fn with_intervals(db, secrets, debounce, fetch_every) -> Arc<Self>
  fn set_after_restore(f) — Define o que fazer depois de uma restauração (uma vez só, no bootstrap).
  fn settings() -> anyhow::Result<Settings>
  fn allow_set() -> anyhow::Result<BTreeSet<String>>
  fn allowed() -> BTreeSet<String> — As suspeitas que o usuário liberou (fingerprints), para marcar a lista do `InfraAudit`.
  fn fingerprint(project, scope, what) -> String
  fn set_allowed(project, scope, what, allowed) -> Result<(), StateGitError> — Libera (ou desfaz) uma **suspeita** da auditoria.
  fn status() -> anyhow::Result<StateGitStatus>
  fn configure(mode, remote, branch, provider_id) -> Result<StateGitStatus, StateGitError> — Liga/desliga/configura.
  fn refuse_unrelated_history(remote, branch, provider_id) -> Result<(), StateGitError> — Recusa ligar o `sync` num servidor que **já tem estado** apontando para um repositório que **já tem …
  fn restore_if_virgin() -> anyhow::Result<()> — **Restauração num servidor limpo (Fase 8).** Só age quando o servidor é virgem: nenhum projeto e nen…
  fn sync_now() -> Result<StateGitStatus, StateGitError> — "Sincronizar agora": commit pendente e, em `sync`, push.
  fn cycle(force_push) — Um ciclo: audita, commita (se houver diff) e, em `sync`, envia.
  fn commit_local(s) -> anyhow::Result<bool> — Commit do que há de local: dentro do lock de escrita, para o `git` só ver o diretório entre transaçõ…
  fn cycle_inner(force_push) -> anyhow::Result<()>
  fn reconcile(s) -> anyhow::Result<Reconciled> — `fetch` + comparação dos históricos.
  fn apply_incoming(sha, force) -> Applied — Aplica o que chegou por fora: lê a árvore do commit **sem checkout**, valida tudo e grava como uma t…
  fn record_marks(sha, old, new) -> anyhow::Result<()> — Registra o que o `pull` deixou para decisão humana: o que **sumiu** (nunca apagamos container nem da…
  fn resume() -> Result<StateGitStatus, StateGitError> — Retomar depois de resolver o histórico no repositório: tira a pausa e reavalia.
  fn resolve(choice) -> Result<StateGitStatus, StateGitError> — Resolve um histórico divergido **sem terminal**, descartando um dos lados — com cópia de segurança d…
  fn dismiss_removal(id) -> Result<(), StateGitError> — Esquece uma marca de "removido no git" (a pessoa decidiu manter o que ainda roda, ou já limpou).
  fn auth_for(s) -> anyhow::Result<Option<Auth>> — Resolve a credencial do remoto: `https://` usa o token do provider; `file://` não usa nada.
  fn push(s)
  fn spawn() — O worker: acorda a cada mudança do estado (com debounce), a cada "sincronizar agora" e periodicament…
const KEY_MODE, KEY_REMOTE, KEY_BRANCH, KEY_PROVIDER, KEY_ALLOW, DEFAULT_BRANCH, DEBOUNCE
(29 testes)

## rustploy-daemon/crates/daemon/src/features/templates/

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

## rustploy-daemon/crates/daemon/src/features/update/

### apply.rs — `ApplyUpdate`: baixa o pacote assinado e o entrega ao helper root `rustployd-update`.
const HELPER_SOCKET — Onde o helper escuta (`rustployd-update.socket`).
const UPDATES_DIR — Único diretório de onde o helper aceita um `.deb`; o daemon escreve aqui.
const OUTCOME_FILE — Resultado da última instalação, gravado pelo helper (legível por todos).
const DOWNLOAD_HOSTS — Os arquivos de uma release do GitHub saem de github.com e redirecionam para `*.githubusercontent.com…
const SNAPSHOTS_KEPT — Quantos snapshots do banco ficam (os mais recentes).
const STALE_IN_PROGRESS_SECS — Um "instalando" mais velho que isto é resto de um helper que morreu, não uma instalação viva.
struct HelperPaths { socket, updates_dir, outcome_file }
struct ApplyState { stage, target_version, message, started_unix, finished_at }
impl ApplyState
  fn idle() -> Self
struct UpdateApplier { paths, allow_agent_apply, state }
struct ApplyPlan { release, repo, db, snapshots_dir, current_version } — Tudo de que a tarefa de fundo precisa (ela sobrevive ao handler que a disparou).
fn now_unix() -> u64
impl UpdateApplier
  fn new(config) -> Self
  fn with_paths(config, paths) -> Self
  fn allow_agent_apply() -> bool — `[update] allow_agent_apply`: se o agente MCP pode pedir `ApplyUpdate`/`UpdateRollback`.
  fn is_helper_available() -> bool — O helper existe nesta máquina? (Sem systemd ou instalação por `cargo`, o socket não existe.)
  fn try_begin(target_version) -> Result<(), String> — Marca o início de uma tentativa.
  fn set(stage, message)
  fn status(running_version) -> UpdateProgress — O que a UI mostra: o passo em memória e, depois do reinício (ou quando o helper já terminou), o que …
  fn run(plan) — Roda a atualização inteira em segundo plano; qualquer erro vira `Failed` com a causa.
  fn run_steps(plan) -> Result<(), String>
  fn rollback() -> Result<(), String> — `UpdateRollback`: pede ao helper que restaure os binários do último backup.
fn from_outcome(outcome, target_version, running_version, from_this_attempt)
fn package_matches_running(package_version, running_version) -> bool — `0.1.10-1` (versão do pacote) casa com `0.1.10` (versão do crate em execução).
fn prepare_updates_dir(directory) -> std::io::Result<()> — Cria o diretório e tira o que sobrou de tentativas anteriores (pacote velho não pode ser confundido …
fn snapshot_database(db, directory, tag) -> Result<PathBuf, String> — Cópia consistente do banco **em uso** (`VACUUM INTO`), antes de a versão nova poder migrar o esquema…
fn snapshot_infra(db, dir) -> Result<(), String> — Grava o estado de infra atual em `dir`, no mesmo layout de `<db_path>/infra/`.
fn create_private_dir(dir) -> std::io::Result<()>
fn unique_path(path) -> PathBuf
fn prune_snapshots(directory) — Remove os snapshots mais antigos.
fn helper_io_error(error) -> String — Um helper que cai no meio (ou fecha antes de ler) aparece como conexão resetada/quebrada: para quem …
struct HelperReply { ok, error }
fn helper_call(socket, request, timeout) -> Result<HelperReply, String> — Uma linha JSON para o helper e uma de volta (ver `src/bin/rustployd-update/protocol.rs`).
const MAX_DEB_BYTES, MAX_SIGNATURE_BYTES, DOWNLOAD_TIMEOUT, HELPER_ACCEPT_TIMEOUT
impl Default for HelperPaths
(15 testes)

### checker.rs — `UpdateChecker`: compara a versão do daemon com a última release, com cache.
const FORCE_MIN_SPACING — Uma consulta *forçada* ("Verificar agora") ainda respeita este intervalo mínimo: um botão clicado em…
const PERIODIC_FIRST_DELAY — A primeira consulta automática espera o daemon terminar de subir (e não disputa a rede com a recuper…
struct Cached { fetched_at, info, release }
struct UpdateChecker { enabled, repo, ttl, force_spacing, check_interval, cached }
impl UpdateChecker
  fn new(config) -> Self
  fn with_timings(config, ttl, force_spacing) -> Self
  fn cached_release() -> Option<LatestRelease> — A última release vista por [`check`](Self::check), com os arquivos dela.
  fn cached_info() -> Option<UpdateInfo> — O último resultado, sem ir ao GitHub: o snapshot de 2 s das UIs lê daqui.
  fn periodic_loop() — Consulta o GitHub a cada `check_interval_hours` (`[update]`).
  fn repo() -> &str — O repositório consultado (`dono/nome`); `ApplyUpdate` só baixa de URLs dele.
  fn check(force) -> Result<UpdateInfo> — Consulta o GitHub de verdade (a menos que o cache sirva).
  fn check_with(force, current_version, fetch) -> Result<UpdateInfo>
fn build_info(current_version, release, repo) -> Result<UpdateInfo> — Compara o que roda com o daemon que a release **traz** (a versão do pacote `rustployd_<v>_<arq>.deb`…
(12 testes)

### github_release.rs — A última release estável de um repositório do GitHub (`GET /repos/{repo}/releases/latest`).
const MAX_BODY_BYTES — O JSON de uma release cabe em poucos KB; o teto só protege de uma resposta absurda.
struct LatestRelease { tag, url, published_at, assets }
struct ReleasePayload { tag_name, html_url, published_at, assets }
struct AssetPayload { name, browser_download_url }
fn is_valid_repo(repo) -> bool — `dono/nome`, só com os caracteres que o GitHub aceita.
fn fetch_latest_owned(repo) -> Result<LatestRelease> — Versão que recebe a `String` por valor, para o `future` não emprestar nada do chamador.
fn fetch_latest(repo) -> Result<LatestRelease>
fn parse_response(status, rate_limit_remaining, body) -> Result<LatestRelease>
const REQUEST_TIMEOUT
(7 testes)

### mod.rs — Verificação de versão: pergunta ao GitHub qual é a última release e compara com a do daemon.
