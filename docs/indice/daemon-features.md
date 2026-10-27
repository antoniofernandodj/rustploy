# Índice: daemon: bancos, IaC, observabilidade, backup, templates e wizard

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-daemon/crates/daemon/src/features/

### mod.rs — Um diretório por assunto de produto; a camada `api` só adapta pedidos para estas funcionalidades.

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
fn db_form(req) -> DbFormInput
fn effective_name(req) -> String — Nome efetivo: `app_name` tem precedência sobre `name` (ambos trimados).
fn build_spec(req) -> Result<ServiceSpec, String> — Monta o `ServiceSpec` a partir da requisição do wizard.
fn build_spec_inner(req) -> Result<ServiceSpec, String>

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
  fn kind_id() -> &'static str — `db_kind` que `crate::platform::connection` entende.
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
struct ServiceManifest { name, source, port, host_port, domain, tls, domains, env, env_comments, volumes, healthcheck, replicas, resources, command, args, db, shared }
struct SourceManifest { registry, git, compose, compose_ingress } — Origem do serviço: exatamente uma das três chaves deve estar presente.
struct GitManifest { url, branch, root_path, dockerfile, context, build_stage, submodules, watch_paths, username, credentials, provider }
struct HealthcheckManifest { kind, path, status, interval, timeout, retries, start_period }
struct ResourcesManifest { cpu_shares, mem }
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
(11 testes)

### manifest_apply.rs — `Command::ManifestApply`: reconcilia manifestos YAML de projeto (IaC) com o banco, com prune e deplo…
fn handle(state, manifests, prune, deploy) -> RpResponse — Reconcilia uma lista de manifestos YAML de projeto contra o banco.
fn apply_one(state, manifest, prune, provider_ids, report, changed) -> Result<(), RpResponse> — Aplica um manifesto de projeto: resolve o projeto pelo nome, atualiza env vars só se o manifesto dec…
fn db_err(e) -> RpResponse

### manifest_export.rs — `Command::ManifestExport`: exporta um projeto como manifesto IaC (secrets como `secret:NOME`).
fn handle(state, project_id) -> RpResponse — Exporta o estado atual de um projeto como manifesto declarativo.

### manifest_export_all.rs — `Command::ManifestExportAll`: exporta todos os projetos e serviços num manifesto raiz só.
fn handle(state) -> RpResponse — Exporta TODOS os projetos+serviços como um único manifesto raiz, com todo valor de env var `Plain` r…

### manifest_import.rs — `Command::ManifestImport`: importa um manifesto IaC (raiz `projects:` ou de projeto) mais o `.env`, …
fn handle(state, yaml, dotenv, prune, deploy) -> RpResponse — Importa um manifesto (raiz `projects:` ou de projeto único `project:`) junto com o TOML de variáveis…
fn reconcile_git_providers(db, docs) -> Result<(), RpResponse> — Garante que todo Git provider referenciado em `docs` (chave = nome, o mesmo usado em `source.git.pro…
fn check_git_provider_refs(db, projects) -> Result<(), RpResponse> — Garante que todo `source.git.provider` (nome) referenciado no YAML resolve a um provider existente n…
fn parse_projects(yaml) -> Result<Vec<ProjectManifest>, String> — Extrai a lista de `ProjectManifest` de um YAML colado (raiz ou projeto único).
(5 testes)

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
fn publish_log(bus, service_id, container_id, is_stderr, bytes)
const STREAM_IDLE_TIMEOUT

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
fn unique_path(path) -> PathBuf
fn prune_snapshots(directory)
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
