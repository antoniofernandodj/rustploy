# Índice: daemon: API HTTP, web UI, agente MCP e handlers de Command

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-daemon/crates/daemon/src/api/

### dispatch.rs — `dispatch`: o `match` que manda cada `Command` para o seu handler em `handlers/`.
fn dispatch(state, cmd) -> RpResponse

### mod.rs — API do daemon: adaptadores (HTTP, web UI, agente MCP) que traduzem pedidos em chamadas às features.

## rustploy-daemon/crates/daemon/src/api/agent/

### access_key.rs — Token de conexão: o endereço do daemon e o token da API numa string só.
const SEPARATOR — Separa o endereço do token dentro do token de conexão decodificado.
struct AccessKey { address, token }
enum KeyFormatError { Empty, NotBase64, MissingSeparator, MissingField } — Qual parte do token de conexão não confere — cada uma tem uma correção diferente, e é isso que deixa…
impl KeyFormatError
  fn code() -> &'static str
  fn message() -> String
impl AccessKey
  fn encode() -> String
  fn decode(raw) -> Result<Self, KeyFormatError>
fn split_address(text) -> Option<(&str, &str)> — Parte `<endereço>::<token>` no primeiro `::` que não seja do próprio endereço — um IPv6 entre colche…
(4 testes)

### mcp.rs — Servidor MCP em `POST /mcp` — transporte Streamable HTTP, sempre respondendo `application/json` (o s…
const PROTOCOL_VERSIONS — Versões do protocolo que este servidor fala, da mais nova para a mais velha.
fn handle(state, body, server) -> Option<(StatusCode, Value)> — Atende um corpo de `POST /mcp`.
fn error(id, code, message) -> Value
fn initialize(params, server) -> Value
fn tools_list() -> Value
fn tools_call(state, params) -> Result<Value, (i64, String)> — Falha de ferramenta vai no **resultado** (`isError`), não como erro JSON-RPC: é assim que o modelo d…
fn resources_read(params) -> Result<Value, (i64, String)>
const MANUAL_URI
(3 testes)

### mod.rs — Acesso de agente servido pelo próprio daemon: a chave de acesso, o `POST /api/login`, o manual e o s…
const MANUAL — O manual do agente — fonte única do texto servido em `GET /api/agent/manual`, nas `instructions` do …
fn login(body, expected, host, scheme) -> (StatusCode, Value) — `POST /api/login`: troca o token de conexão pelo token da API.
fn extract_key(body) -> Result<String, KeyFormatError> — Tira o token de conexão do corpo do login.
fn format_error(fail) -> Value — O `400` do login: o que falhou **e** o formato certo, para um agente acertar na tentativa seguinte s…
fn public_address(api, fallback_host) -> String — `host:porta` público deste daemon: `[api].domain` quando definido, senão o IP de saída; a porta é se…
fn access_key_for(address, token) -> Option<String> — Monta o token de conexão a partir do endereço e do `[api].token`.
fn print_access_key_and_exit(config) -> ! — Imprime o token de conexão e sai — `rustployd access-key`.
const CONNECTION_TOKEN_KEY — Chave do `config.toml` onde o daemon deixa o token de conexão pronto.
fn write_connection_token(config) -> anyhow::Result<()> — Grava `connection_token = "<token de conexão>"` logo abaixo do `token` da seção `[api]` do `config.t…
fn config_file_in_use() -> Option<std::path::PathBuf> — O mesmo arquivo que `RustployConfig::load` escolhe: o primeiro da lista que existe **e** faz parse.
fn with_connection_token(contents, api_token, key) -> Option<String> — O texto do `config.toml` com a linha do token de conexão no lugar (logo abaixo do `token` de `[api]`…
(9 testes)

### tools.rs — As ferramentas do agente: os caminhos frequentes já resolvidos ("deployou? por quê não?", "qual é o …
const POLL_INTERVAL — De quanto em quanto tempo o `wait` de um deploy reconsulta o histórico.
const MAX_WAIT — Teto do `timeout_s` aceito em `deploy`.
struct ToolError { code, message }
impl ToolError
  fn new(code, message) -> Self
  fn bad_request(message) -> Self
struct ToolSpec { name, description, input_schema } — Uma ferramenta: nome, descrição para o agente e o JSON Schema dos argumentos.
fn no_arguments() -> Value
fn call(state, name, args) -> Result<Value, ToolError> — Executa a ferramenta `name`.
fn rpc(state, command) -> Result<Value, ToolError> — Executa um `Command` (em JSON) e converte `Response::Err` em falha.
fn rpc_expect(state, command, variante) -> Result<Value, ToolError> — Como [`rpc`], mas exige a variante `variante` e devolve só o payload dela.
fn status(state) -> Result<Value, ToolError>
fn snapshot(state) -> Result<Value, ToolError> — `Snapshot` devolve JSON **dentro** de uma string.
fn list_deploys(state, args) -> Result<Value, ToolError>
fn deploy(state, args) -> Result<Value, ToolError>
fn resolve_service(state, args) -> Result<String, ToolError> — Aceita o id direto (`service_id`) ou o nome (`service`).
fn wait_for_outcome(state, service_id, deployment_id, limite) -> Result<(Value, bool), ToolError> — Reconsulta o histórico até o deployment chegar a um estado terminal (ou o prazo acabar).
fn build_logs(state, args) -> Result<Value, ToolError>
fn build_log_lines(state, deployment_id) -> Result<Vec<String>, ToolError>
fn compact_summary(sum) -> Value — `DeploymentSummary` → linha compacta com o desfecho resolvido.
fn compact_deployment(dep) -> Value — `Deployment` → o que interessa a quem só quer saber como acabou.
fn outcome_ok(estado) -> Value — `true` = no ar, `false` = falhou, `null` = ainda não decidiu.
fn is_terminal(dep) -> bool
fn failure_reason(dep) -> Value — A causa da falha, tirada do `states_log`.
fn service_index(snapshot) -> Vec<Value> — Achata o snapshot no índice de serviços que um agente precisa para agir.
fn status_label(status) -> Value — `ServiceStatus` é externally-tagged e só `Error` tem campo — vira `"Running"` ou `"Error: <causa>"`.
fn source_label(source) -> Value — `ServiceSource` achatado no que identifica a origem, sem arrastar o `compose.content` inteiro (dezen…
const TOOLS
(4 testes)

## rustploy-daemon/crates/daemon/src/api/handlers/

### mod.rs — Handlers agrupados por assunto: um arquivo por `Command` dentro de cada pasta, e o `dispatch` da pas…

## rustploy-daemon/crates/daemon/src/api/handlers/databases/

### managed_database.rs — `Command::ManagedDatabase{List,Create,Delete}`: databases (com usuário próprio) dentro de um servido…
fn server(state, id) -> Result<(Service, Engine), RpResponse>
fn views(state, svc, engine) -> RpResponse
fn list(state, server_service_id) -> RpResponse
fn create(state, req) -> RpResponse
fn delete(state, id) -> RpResponse
fn list_all(state) -> RpResponse

### migration.rs — `Command::Migration*`: assistente de migração para banco compartilhado (ver `crate::features::databa…
fn err(code, m) -> RpResponse
fn start(state, req) -> RpResponse
fn get(state, id) -> RpResponse
fn list(state, project_id) -> RpResponse
fn rollback(state, id) -> RpResponse — Reverte uma migração concluída: env var e serviços voltam ao banco antigo.
fn discard(state, id) -> RpResponse — Descarta o banco antigo (já parado) — **apaga os dados dele**.

### mod.rs — Handlers de databases: o `dispatch` recebe os `Command` da seção e chama o handler certo.
fn dispatch(state, cmd) -> RpResponse

### shared_access.rs — `Command::SharedAccess{List,Grant,Revoke}`: quais projetos alcançam um servidor de banco compartilha…
fn server(state, id) -> Result<Service, RpResponse>
fn view(state, svc) -> Vec<SharedAccess>
fn list(state, server_service_id) -> RpResponse
fn grant(state, server_service_id, project_id) -> RpResponse
fn revoke(state, server_service_id, project_id) -> RpResponse

## rustploy-daemon/crates/daemon/src/api/handlers/deploys/

### deploy_abort.rs — `Command::DeployAbort`: aborta um deploy.
fn handle(state, deployment_id) -> RpResponse

### deploy_delete.rs — `Command::DeployDelete`: apaga um deployment terminado e seus logs de build; recusa um em andamento.
fn handle(state, deployment_id) -> RpResponse

### deploy_engine_status.rs — `Command::DeployEngineStatus`: estado da tela Deploy Engine: fila (na ordem da fila), o que está rod…
fn handle(state) -> RpResponse

### deploy_history.rs — `Command::DeployHistory`: últimos deployments de um serviço, com o `live` velho corrigido por `recon…
fn handle(state, service_id, limit) -> RpResponse

### deploy_queue_pause.rs — `Command::DeployQueuePause`: pausa/retoma a fila global de deploys.
fn handle(state, paused) -> RpResponse — Pausa (`true`) ou retoma (`false`) a fila global.

### deploy_queue_promote.rs — `Command::DeployQueuePromote`: fura a fila (move um enfileirado para o início).
fn handle(state, deployment_id) -> RpResponse — Move um deploy enfileirado para o início da fila ("furar fila").

### deploy_queue_reorder.rs — `Command::DeployQueueReorder`: reordena a fila global (arrastar na GUI).
fn handle(state, order) -> RpResponse — Reordena a fila para a ordem dada (ids de deployment).

### deploy_rollback.rs — `Command::DeployRollback`: acha o deploy Live anterior de um serviço (só consulta; não redeploya).
fn handle(state, service_id) -> RpResponse

### get_build_logs.rs — `Command::GetBuildLogs`: linhas do log de build de um deployment.
fn handle(state, deployment_id) -> RpResponse

### mod.rs — Handlers de deploys: o `dispatch` recebe os `Command` da seção e chama o handler certo.
fn dispatch(state, cmd) -> RpResponse

### recent_deployments.rs — `Command::RecentDeployments`: deployments recentes de todos os serviços, com o Live velho corrigido …
fn handle(state, limit) -> RpResponse

### reconcile.rs — Corrige deployments marcados Live que já não estão no ar (`fix_stale_live`), sem tocar em serviço co…
fn fix_stale_live(state, deployments) -> Vec<Deployment> — Passa para Pruning/Stopped os deployments marcados Live que já não estão no ar (só o mais recente de…
fn service_is_deploying(state, service_id) -> bool — True enquanto um deploy do serviço está em curso ou esperando na fila (`ServiceStatus::Deploying`/`Q…
fn transition_deployment(state, dep, to, msg)
fn is_container_running(state, service_id) -> bool
const RUNNING_CHECK_RETRIES, RUNNING_CHECK_DELAY

## rustploy-daemon/crates/daemon/src/api/handlers/docker/

### docker_cleanup.rs — `Command::DockerCleanupConfig*` e `DockerCleanupRunNow`: config e disparo manual da limpeza automáti…
fn get_config(state) -> RpResponse
fn set_config(state, config) -> RpResponse
fn run_now(state) -> RpResponse — Dispara a limpeza fora do horário agendado (botão "Executar agora"), com os recursos marcados atualm…
fn service_warm_status_list(state) -> RpResponse — `Command::ServiceWarmStatusList`: o status de aquecimento de cada serviço com build (Git ou zip), pa…
fn service_warm(state, service_id, force) -> RpResponse — `Command::ServiceWarm`: aquece um serviço sem fazer deploy dele (`maintenance::service_warm::request…

### docker_inventory.rs — Docker-wide inventory for the Docker tab: every image/volume/network on the host (not just rustploy-…
struct ServiceIndex { by_safe_name, by_registry_image, by_network, by_service_id } — Best-effort project/service ownership lookup, built once per request from the current DB state.
impl ServiceIndex
  fn build(state) -> Self
  fn image_owner(tags) -> (Option<String>, Option<String>) — Resolves an image's owner from its tags: exact match for registry images, `rp_<safe_name>:...` prefi…
  fn network_project(name) -> Option<String> — Resolves a network's owning project from the stored network name.
fn list_images(state) -> RpResponse — Lists every image on the host via `docker system df`, the one Docker Engine endpoint that always com…
fn list_volumes(state) -> RpResponse — Lists every volume on the host via `docker system df`, which (unlike the plain volume-list endpoint)…
fn compute_df_inventory(state) — Runs `docker system df` once and turns it into both the image and volume inventories (ownership attr…
fn list_networks(state) -> RpResponse — Lists every network on the host.
fn compute_networks(state) -> Result<Vec<DockerNetworkInfo>, String> — The cache miss-path of [`list_networks`]: lists networks and cross-references container attachments …
fn list_containers(state) -> RpResponse — Lists every container on the host (running + stopped) for the Docker tab's Containers sub-tab.
fn stop_all_managed(state) -> RpResponse — Stops every rustploy-managed service, regardless of what the DB's status column currently claims — r…

### docker_prune.rs — `Command::Prune*`: limpeza manual de containers, volumes, imagens, build cache e redes; a lógica mor…
fn respond(result) -> RpResponse
fn prune_containers(state) -> RpResponse
fn prune_volumes(state, all) -> RpResponse
fn prune_images(state, all) -> RpResponse
fn prune_build_cache(state) -> RpResponse
fn prune_networks(state) -> RpResponse

### docker_remove.rs — Remoção INDIVIDUAL de um recurso Docker (o par por-item dos `docker_prune`): um container, imagem, v…
fn remove_container(state, id) -> RpResponse — `docker rm <id>` (sem `-f`): remove um container PARADO.
fn remove_image(state, id) -> RpResponse — `docker rmi <id>` (sem `-f`): remove uma imagem sem uso.
fn remove_image_by_name(state, name) -> Result<(), bollard::errors::Error>
fn remove_image_tag_by_tag(state, id) -> Result<(), bollard::errors::Error>
fn needs_untag(e) -> bool — A recusa por ID de imagem com várias tags: o Podman diz "more than one tag", o Docker "referenced in…
fn remove_volume(state, name) -> RpResponse — `docker volume rm <name>` (sem `-f`): remove um volume sem uso.
fn remove_network(state, id) -> RpResponse — `docker network rm <id>`: remove uma rede sem uso.

### mod.rs — Handlers de docker: o `dispatch` recebe os `Command` da seção e chama o handler certo.
fn dispatch(state, cmd) -> RpResponse

## rustploy-daemon/crates/daemon/src/api/handlers/git/

### git_branch_list.rs — `Command::GitBranchList`: branches de um repositório num provedor git conectado.
fn handle(state, provider_id, repo_full_name) -> Response

### git_oauth_start.rs — `Command::GitOAuthStart`: monta a URL de autorização OAuth do provedor git e guarda o `state` CSRF.
fn handle(state, provider_id) -> Response — Produces the provider's authorization URL the client opens in a browser, after stashing a CSRF `stat…
fn pct(s) -> String — Percent-encodes a query-string value (RFC 3986 unreserved set kept as-is).

### git_provider_create.rs — `Command::GitProviderCreate`: conecta um provedor git (Gitea/GitHub) por OAuth ou token pessoal.
fn handle(state, kind, name, base_url, auth_mode, oauth_client_id, oauth_client_secret, pat) -> Response — Registers a Git provider.

### git_provider_delete.rs — `Command::GitProviderDelete`: desconecta (apaga) um provedor git.
fn handle(state, id) -> Response

### git_provider_list.rs — `Command::GitProviderList`: provedores git conectados, sem os tokens.
fn handle(state) -> Response

### git_repo_list.rs — `Command::GitRepoList`: repositórios de um provedor git, com um refresh OAuth e nova tentativa se o …
fn handle(state, provider_id) -> Response

### mod.rs — Handlers de git: o `dispatch` recebe os `Command` da seção e chama o handler certo.
fn dispatch(state, cmd) -> RpResponse

## rustploy-daemon/crates/daemon/src/api/handlers/iac/

### mod.rs — `Command`s de IaC (manifesto e pacote de serviço): só roteia para `features::iac`.
fn dispatch(state, cmd) -> RpResponse

## rustploy-daemon/crates/daemon/src/api/handlers/jobs/

### get_job_logs.rs — `Command::GetJobLogs`: linhas do log de uma execução de job.
fn handle(state, job_run_id) -> RpResponse

### job_create.rs — `Command::JobCreate`: cria um job one-shot (Schedules) de um projeto.
fn handle(state, project_id, trigger_service_id, name, compose, git_source, main_service, env_vars, env_comments, recurrence) -> RpResponse

### job_delete.rs — `Command::JobDelete`: apaga um job (e o registro de aquecimento dele) e limpa o `pre_deploy_job_id` …
fn handle(state, id) -> RpResponse

### job_list.rs — `Command::JobList`: jobs (Schedules) de um projeto.
fn handle(state, project_id) -> RpResponse

### job_list_all.rs — `Command::JobListAll`: jobs de todos os projetos, com nomes de projeto/serviço resolvidos.
fn handle(state) -> RpResponse

### job_run_cancel.rs — `Command::JobRunCancel`: sinaliza o cancelamento de uma execução de job em andamento.
fn handle(state, job_run_id) -> RpResponse — Sinaliza cancelamento pro `job_run` em `state.active_jobs` (ver `docs/plano-cancelamento-de-jobs.md`…

### job_run_history.rs — `Command::JobRunHistory`: últimas execuções de um job.
fn handle(state, job_id, limit) -> RpResponse

### job_run_now.rs — `Command::JobRunNow`: dispara uma execução avulsa de um job (`jobs::runner::spawn`).
fn handle(state, id) -> RpResponse

### job_update.rs — `Command::JobUpdate`: edita um job (compose ou fonte git, serviço principal, env, recorrência, ligad…
fn handle(state, id, name, compose, git_source, main_service, env_vars, env_comments, enabled, recurrence) -> RpResponse

### job_warm.rs — `Command::JobWarm`: aquece um job (baixa as imagens que faltam e popula o cache de build) sem execut…
fn handle(state, job_id, force) -> RpResponse

### job_warm_status.rs — `Command::JobWarmStatus`: se as imagens e o cache de build de um job já estão no host — a fonte únic…
fn handle(state, job_id) -> RpResponse

### mod.rs — Handlers de jobs: o `dispatch` recebe os `Command` da seção e chama o handler certo.
fn dispatch(state, cmd) -> RpResponse

## rustploy-daemon/crates/daemon/src/api/handlers/observability/

### daemon_status.rs — `Command::DaemonStatus`: versão, uptime e contagem de serviços rodando/total.
fn handle(state) -> RpResponse

### ingress.rs — Leitura e conserto da tabela de rotas do ingress proxy.
fn routes(state) -> RpResponse — Foto da tabela viva (domínios + portas).
fn reconcile(state, service_id) -> RpResponse — Recalcula as rotas a partir dos containers que existem de fato no Docker e devolve a tabela já corri…

### logs_get.rs — `Command::LogsGet`: últimas linhas de log de um serviço, do container ou (Compose) via `docker compo…
fn handle(state, service_id, tail) -> RpResponse
fn parse_compose_log_ts(line) -> Option<chrono::DateTime<Utc>> — Extracts the RFC3339 timestamp from a `docker compose logs --timestamps` line.
fn compose_logs(project_name, content, tail, env_vars) -> RpResponse — Logs de um serviço Compose via `docker compose logs --timestamps`, com o timestamp de cada linha ext…

### mod.rs — Handlers de observability: o `dispatch` recebe os `Command` da seção e chama o handler certo.
fn dispatch(state, cmd) -> RpResponse

### ping.rs — `Command::Ping`: responde `Pong` com o uptime.
fn handle(state) -> RpResponse

## rustploy-daemon/crates/daemon/src/api/handlers/projects/

### mod.rs — Handlers de projects: o `dispatch` recebe os `Command` da seção e chama o handler certo.
fn dispatch(state, cmd) -> RpResponse

### project_create.rs — `Command::ProjectCreate`: cria um projeto.
fn handle(state, name, description) -> RpResponse

### project_delete.rs — `Command::ProjectDelete`: apaga um projeto; recusa se ainda tiver serviços ou jobs.
fn handle(state, id) -> RpResponse

### project_env_set.rs — `Command::ProjectEnvSet`: substitui as env vars (e comentários) de nível de projeto.
fn handle(state, project_id, env_vars, env_comments) -> RpResponse

### project_list.rs — `Command::ProjectList`: todos os projetos.
fn handle(state) -> RpResponse

### project_update.rs — `Command::ProjectUpdate`: renomeia/redescreve um projeto.
fn handle(state, id, name, description) -> RpResponse

## rustploy-daemon/crates/daemon/src/api/handlers/registry/

### commands.rs — Sub-aba Docker > Registry: navegação (repositórios/tags), delete e GC do registry OCI embutido (`cra…
fn status(state) -> RpResponse
fn repo_list(state) -> RpResponse
fn tag_list(state, repo) -> RpResponse
fn tag_delete(state, repo, tag) -> RpResponse
fn repo_delete(state, repo) -> RpResponse
fn gc(state) -> RpResponse
fn generate_secret() -> String — Gera 32 bytes aleatórios via `/dev/urandom` (mesmo padrão de `secrets.rs::generate_key`/`db::webhook…
fn token_create(state, name, scope) -> RpResponse
fn token_list(state) -> RpResponse
fn token_revoke(state, name) -> RpResponse

### mod.rs — Handlers de registry: o `dispatch` recebe os `Command` da seção e chama o handler certo.
fn dispatch(state, cmd) -> RpResponse

## rustploy-daemon/crates/daemon/src/api/handlers/secrets/

### mod.rs — Handlers de secrets: o `dispatch` recebe os `Command` da seção e chama o handler certo.
fn dispatch(state, cmd) -> RpResponse

### secret_delete.rs — `Command::SecretDelete`: apaga um secret do projeto.
fn handle(state, project_id, name) -> Response

### secret_list.rs — `Command::SecretList`: nomes (nunca valores) dos secrets de um projeto.
fn handle(state, project_id) -> Response

### secret_set.rs — `Command::SecretSet`: cria/sobrescreve um secret cifrado do projeto.
fn handle(state, project_id, name, value) -> Response

## rustploy-daemon/crates/daemon/src/api/handlers/services/

### get_webhook_url.rs — `Command::GetWebhookUrl`: URL de webhook de deploy de um serviço; `build_url` monta `{base}/webhook/…
fn handle(state, service_id) -> RpResponse
fn build_url(state, service_id, token) -> String — `{public_base_url}/webhook/{service_id}/{token}` — servido pelo listener da API (mesma porta), então…

### mod.rs — Handlers de services: o `dispatch` recebe os `Command` da seção e chama o handler certo.
fn dispatch(state, cmd) -> RpResponse

### regenerate_webhook_token.rs — `Command::RegenerateWebhookToken`: gera um token novo de webhook para o serviço e devolve a URL nova…
fn handle(state, service_id) -> RpResponse

### service_archive_upload.rs — Upload de `.zip` para serviço com fonte Archive: valida o tamanho e extrai com proteção contra path …
fn handle(state, service_id, bytes, original_filename) -> RpResponse
fn archive_extract_dir(db_path, service_id, archive_id) -> PathBuf
fn extract_zip(bytes, dest) -> Result<()>
fn safe_zip_path(name) -> Option<PathBuf>
const MAX_ZIP_BYTES

### service_connection_info.rs — `Command::ServiceConnectionInfo`: host e connection string internos de um serviço (ver `crate::platf…
fn handle(state, service_id) -> RpResponse

### service_get.rs — `Command::ServiceGet`: um serviço pelo id.
fn handle(state, id) -> RpResponse

### service_list.rs — `Command::ServiceList`: serviços de um projeto.
fn handle(state, project_id) -> RpResponse

### service_reload.rs — `Command::ServiceReload`: reinicia o container live de um serviço (stop gracioso + start) sem redepl…
fn handle(state, service_id) -> RpResponse

### wizard.rs — Wizard "Novo serviço" server-side: catálogos (`WizardCatalog`) e criação (`WizardCreate`).
fn catalog(search) -> RpResponse — Catálogos de bancos/brokers/templates prontos como JSON para o contexto do cliente (`new_service_dat…
fn create(state, req) -> RpResponse — Monta o `ServiceSpec` a partir dos campos coletados pelo wizard e cria o serviço — reaproveitando o …

## rustploy-daemon/crates/daemon/src/api/handlers/settings/

### env_backup.rs — `Command::EnvBackupList`/`EnvBackupRestore`: lista e restaura snapshots de env vars (`crate::feature…
fn list(state) -> RpResponse
fn restore(state, snapshot) -> RpResponse

### get_daemon_settings.rs — `Command::GetDaemonSettings`: e-mail ACME, domínio do registry e a URL pública derivada de `[api]`.
fn handle(state) -> RpResponse

### mod.rs — Handlers de settings: o `dispatch` recebe os `Command` da seção e chama o handler certo.
fn dispatch(state, cmd) -> RpResponse

### set_daemon_settings.rs — `Command::SetDaemonSettings`: grava e-mail ACME e domínio do registry, ligando/desligando ACME e a r…
fn handle(state, acme_email, registry_domain) -> RpResponse
fn provision_existing_domains(state) — Emite certificados para todos os services já em execução com tls_enabled.
fn save_optional(state, key, value) -> Result<(), RpResponse>

## rustploy-daemon/crates/daemon/src/api/http/

### dashboard.rs — O painel "Em execução" (`dashboard` no snapshot): só os projetos que têm serviço rodando e, dentro d…
struct DashboardProject { project_id, project_name, services }
struct DashboardService { service_id, name, status, kind, kind_label, branch, repo_url, image, engine, port, domains, webhook, pre_deploy_checks, warm }
struct DashboardCheck { job_id, name }
fn classify(svc) -> (&'static str, String) — Tipo do serviço e o rótulo curto do cartão.
fn is_running(status) -> bool
fn build(projects, services, jobs, webhook_urls, warm) -> Vec<DashboardProject> — A montagem, sem tocar em banco nem em Docker.
fn load(state) -> Vec<DashboardProject> — O painel completo, lendo o que precisa do banco.
const DATABASE_KINDS, BROKER_KINDS
(7 testes)

### mod.rs — HTTP/JSON + SSE control API — the daemon's remote administrative channel.
type ApiBody = BoxBody<Bytes, Infallible> — Unified response body: both the buffered (`Full`) replies and the streaming (`StreamBody`) SSE body …
fn run(state, cfg, tls) — Starts the API listener.
fn serve_conn(io, state, token, peer) — Serves one HTTP/1.1 connection over `io` (plain TCP or a TLS stream).
fn handle(req, state, token, peer) -> Result<Response<ApiBody>, Infallible> — Roteia uma requisição HTTP da API: rotas públicas (webhook, callback OAuth) antes do gate do token B…
fn login(req, expected, peer, scheme) -> Response<ApiBody> — `POST /api/login`: troca o token de conexão pelo token da API (ver `agent::login`).
fn mcp(req, state) -> Response<ApiBody> — `POST /mcp`: o servidor MCP do agente (ver `agent::mcp`).
fn json_value(status, body) -> Response<ApiBody>
fn service_archive_upload(req, state, service_id) -> Response<ApiBody>
fn rpc(req, state) -> Response<ApiBody> — `POST /api/rpc`: decode a `Command`, run it through `dispatch`, encode the `Response` back as JSON.
fn events(state) -> Response<ApiBody> — `GET /api/events`: SSE stream.
fn service_logs(state, service_id) -> Response<ApiBody> — `GET /api/services/{id}/logs`: SSE dedicado aos logs de container de UM serviço.
fn deployment_build_logs(state, deployment_id) -> Response<ApiBody> — `GET /api/deployments/{id}/build-logs`: SSE dedicado à saída de `docker build` de UM deployment.
fn job_run_logs(state, job_run_id) -> Response<ApiBody> — `GET /api/jobs/runs/{id}/logs`: SSE dedicado à saída de UMA execução de job one-shot.
const SEEN_LINES_MAX — Teto do conjunto de linhas já entregues por um stream de logs (a deduplicação da recuperação).
type SeenLines = std::collections::HashSet<(chrono::DateTime<chrono…
fn mark_seen(seen, ts, line) -> bool — Marca uma linha como entregue.
fn recover_lines(db_lines, since, seen) -> Vec<shared::protocol::BuildLogLine> — As linhas do banco que o consumidor perdeu quando o canal descartou eventos (`RecvError::Lagged`): a…
const SSE_KEEPALIVE — Intervalo do comentário de "sinal de vida" nos SSE.
fn sse_response(rx) -> Response<ApiBody> — Response body comum dos endpoints SSE (`events`/`service_logs`/`deployment_build_logs`/`job_run_logs…
fn snapshot(state) -> String — Builds the full dashboard snapshot as one JSON object, reusing `dispatch` for each piece — the same …
fn sse_frame(event, data) -> Bytes — Formats one SSE record.
fn send_log_batch(tx, batch) -> Result<(), ()> — Envia um lote coalescido de eventos de log como uma única frame SSE `bus_batch` (ver o produtor em […
fn auth_check(req, expected) -> Result<(), String> — Checks the `Authorization: Bearer <token>` header against `expected`.
fn boxed(resp) -> Response<ApiBody> — As rotas públicas (`public_routes`) devolvem `Full<Bytes>`; o listener unificado fala `ApiBody`.
fn text(status, body) -> Response<ApiBody>
fn json_ok(bytes) -> Response<ApiBody>
const GZIP_MIN — Corpo mínimo (bytes) para valer a pena comprimir: abaixo disto o overhead do cabeçalho gzip + o cust…
fn accepts_gzip(req) -> bool — O cliente aceita gzip? (`Accept-Encoding` contém "gzip").
fn json_response(bytes, accept_gzip) -> Response<ApiBody> — Resposta JSON, comprimida com gzip quando o cliente aceita e o corpo passa de [`GZIP_MIN`].
fn gzip(data) -> std::io::Result<Vec<u8>> — Comprime `data` em gzip (flate2, mesmo crate do tar do build Docker).
fn constant_time_eq(a, b) -> bool — Short-circuits on length mismatch but is otherwise constant-time over the compared bytes.
(5 testes)

### public_routes.rs — Rotas HTTP **públicas** (sem Bearer): o webhook de deploy e o callback OAuth do Gitea.
fn webhook(req, state) -> Response<Full<Bytes>> — `POST /webhook/{service_id}/{token}` — valida o token e dispara um deploy, a menos que o serviço sej…
fn extract_push_branch(body) -> Option<String> — Extrai o nome curto da branch de um payload de push GitHub/Gitea/Gogs (`{"ref": "refs/heads/main", .…
fn constant_time_eq(a, b) -> bool
fn resp(status, body) -> Response<Full<Bytes>>
fn html(status, title, body) -> Response<Full<Bytes>>
fn oauth_callback(req, state) -> Response<Full<Bytes>> — `GET /oauth/{gitea,github}/callback` — completes an OAuth2 authorization-code flow: validates the CS…
fn callback_redirect_uri(state, kind) -> Option<String> — Builds `{public_base_url}/oauth/{gitea,github}/callback` — a base sai de `[api]` (domínio/porta do l…
fn url_decode_pairs(query) -> Vec<(String, String)> — Tiny `application/x-www-form-urlencoded` query parser (percent-decoding).
fn percent_decode(s) -> String
(5 testes)

## rustploy-daemon/crates/daemon/src/api/web_ui/

### headless_tests.rs — Testes da web UI num Chrome sem cabeça (exigem `google-chrome` instalado): login, telas, janelas e I…
type ApiBody = BoxBody<Bytes, std::convert::Infallible>
fn spawn_static_server() -> std::net::SocketAddr — Sobe um servidor HTTP mínimo (mesmo padrão de accept-loop de `http/mod.rs::listen`) que só chama `se…
const USER_DATA_DIR_COUNTER — `cargo test` roda os 3 testes deste módulo em threads concorrentes; sem um `user_data_dir` próprio, …
fn launch() -> (Browser, tokio::task::JoinHandle<()>)
const ERROR_COLLECTOR_JS — Injeta um coletor de erros ANTES de qualquer script da página rodar (`Page.addScriptToEvaluateOnNewD…
fn open_page(browser, addr) -> chromiumoxide::Page — Abre uma aba em branco, registra o coletor de erros e SÓ ENTÃO navega pra `index.html` — nessa ordem…
fn wait_alpine_ready(page) — Espera até `Alpine.store('app')` existir — só depois do evento `load` (ver o comentário no fim de `w…
const SNAPSHOT_JS — Snapshot mínimo — mesmo formato que `http/mod.rs::snapshot()` monta e `app.js::applySnapshot` consom…
fn seed_connected(page) — Semeia a sessão (bypassa login/SSE, mesma ideia de `boot()` + `define_data` em `templates_render.rs`…
fn eval_str(page, js) -> String — Avalia `js` (uma expressão, possivelmente uma Promise) e devolve o resultado como `String` — os test…
fn visible(page, selector) -> bool — `true` se o elemento existir e estiver de fato visível (`x-show` escreve `display:none` inline; `off…
fn errors(page) -> Vec<String>
(10 testes)

### mod.rs — Servidor de estáticos da web UI/PWA (`crates/daemon/webui/`) — alternativa ao client iced (`rustploy…
type ApiBody = BoxBody<Bytes, std::convert::Infallible>
struct Asset { route, content_type, etag, no_cache, gz } — Um arquivo do app shell, já processado (minificado + gzipado) em tempo de build — ver `Asset` gerado…
fn serve(path) -> Option<Response<ApiBody>> — Serve `path` se casar com algum asset embutido do app shell; `None` se a rota não pertencer à web UI…
const ASSETS
