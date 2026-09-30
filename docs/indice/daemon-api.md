# Índice: daemon: API HTTP e handlers de Command

> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## crates/daemon/src/api/handlers/

### daemon_status.rs — `Command::DaemonStatus`: versão, uptime e contagem de serviços rodando/total.
fn handle(state) -> RpResponse

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

### deploy_start.rs — `Command::DeployStart`: valida a fonte do serviço, cria o deployment e o enfileira na fila global (q…
fn handle(state, service_id) -> RpResponse

### docker_cleanup.rs — `Command::DockerCleanupConfig*` e `DockerCleanupRunNow`: config e disparo manual da limpeza automáti…
fn get_config(state) -> RpResponse
fn set_config(state, config) -> RpResponse
fn run_now(state) -> RpResponse — Dispara a limpeza fora do horário agendado (botão "Executar agora"), com os recursos marcados atualm…

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

### docker_prune.rs — `Command::Prune*`: limpeza manual de containers, volumes, imagens, build cache e redes; as versões `…
struct PruneStat { count, reclaimed_bytes } — Resultado tipado de uma limpeza — o denominador comum entre o botão manual (embrulhado em `RpRespons…
fn prune_containers(state) -> RpResponse
fn prune_containers_core(state) -> Result<PruneStat, String>
fn prune_volumes(state, all) -> RpResponse — `all=true` mirrors `docker volume prune --all`: considers every unused volume, not just anonymous on…
fn prune_volumes_core(state, all) -> Result<PruneStat, String>
fn prune_images(state, all) -> RpResponse — `all=true` mirrors `docker image prune -a`: removes every image unused by any container, not just da…
fn prune_images_core(state, all) -> Result<PruneStat, String>
fn prune_build_cache(_state) -> RpResponse — Usa `docker builder prune -f` via subprocess — a API REST do BuildKit não está exposta pelo bollard …
fn prune_build_cache_core() -> Result<PruneStat, String>
fn parse_reclaimed_space(output) -> u64
fn prune_networks(state) -> RpResponse
fn prune_networks_core(state) -> Result<PruneStat, String>

### docker_remove.rs — Remoção INDIVIDUAL de um recurso Docker (o par por-item dos `docker_prune`): um container, imagem, v…
fn remove_container(state, id) -> RpResponse — `docker rm <id>` (sem `-f`): remove um container PARADO.
fn remove_image(state, id) -> RpResponse — `docker rmi <id>` (sem `-f`): remove uma imagem sem uso.
fn remove_volume(state, name) -> RpResponse — `docker volume rm <name>` (sem `-f`): remove um volume sem uso.
fn remove_network(state, id) -> RpResponse — `docker network rm <id>`: remove uma rede sem uso.

### env_backup.rs — `Command::EnvBackupList`/`EnvBackupRestore`: lista e restaura snapshots de env vars (`crate::env_bac…
fn list(state) -> RpResponse
fn restore(state, snapshot) -> RpResponse

### get_build_logs.rs — `Command::GetBuildLogs`: linhas do log de build de um deployment.
fn handle(state, deployment_id) -> RpResponse

### get_daemon_settings.rs — `Command::GetDaemonSettings`: e-mail ACME, domínio do registry e a URL pública derivada de `[api]`.
fn handle(state) -> RpResponse

### get_job_logs.rs — `Command::GetJobLogs`: linhas do log de uma execução de job.
fn handle(state, job_run_id) -> RpResponse

### get_webhook_url.rs — `Command::GetWebhookUrl`: URL de webhook de deploy de um serviço; `build_url` monta `{base}/webhook/…
fn handle(state, service_id) -> RpResponse
fn build_url(state, service_id, token) -> String — `{public_base_url}/webhook/{service_id}/{token}` — servido pelo listener da API (mesma porta), então…

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

### ingress.rs — Leitura e conserto da tabela de rotas do ingress proxy.
fn routes(state) -> RpResponse — Foto da tabela viva (domínios + portas).
fn reconcile(state, service_id) -> RpResponse — Recalcula as rotas a partir dos containers que existem de fato no Docker e devolve a tabela já corri…

### job_create.rs — `Command::JobCreate`: cria um job one-shot (Schedules) de um projeto.
fn handle(state, project_id, trigger_service_id, name, compose, git_source, main_service, env_vars, env_comments, recurrence) -> RpResponse

### job_delete.rs — `Command::JobDelete`: apaga um job e limpa o `pre_deploy_job_id` dos serviços que o usavam como gate…
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

### logs_get.rs — `Command::LogsGet`: últimas linhas de log de um serviço, do container ou (Compose) via `docker compo…
fn handle(state, service_id, tail) -> RpResponse
fn parse_compose_log_ts(line) -> Option<chrono::DateTime<Utc>> — Extracts the RFC3339 timestamp from a `docker compose logs --timestamps` line.
fn compose_logs(project_name, content, tail, env_vars) -> RpResponse — Logs de um serviço Compose via `docker compose logs --timestamps`, com o timestamp de cada linha ext…

### managed_database.rs — `Command::ManagedDatabase{List,Create,Delete}`: databases (com usuário próprio) dentro de um servido…
fn server(state, id) -> Result<(Service, Engine), RpResponse>
fn host_for(svc, project_id) -> String — Como o projeto consumidor enxerga o servidor: o alias global (outro projeto) ou a chave do YAML (o p…
fn to_view(svc, engine, r) -> ManagedDatabase
fn views(state, svc, engine) -> RpResponse
fn list(state, server_service_id) -> RpResponse
fn create(state, req) -> RpResponse
fn delete(state, id) -> RpResponse
fn list_all(state) -> RpResponse

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

### migration.rs — `Command::Migration*`: assistente de migração para banco compartilhado (ver `crate::migration`).
fn err(code, m) -> RpResponse
fn start(state, req) -> RpResponse
fn get(state, id) -> RpResponse
fn list(state, project_id) -> RpResponse
fn rollback(state, id) -> RpResponse — Reverte uma migração concluída: env var e serviços voltam ao banco antigo.
fn discard(state, id) -> RpResponse — Descarta o banco antigo (já parado) — **apaga os dados dele**.

### mod.rs — Um arquivo por `Command` (o `match` que escolhe o handler fica em `api/routes.rs`) e utilitários com…
fn humanize_db_error(err, subject) -> String — Converte um erro de banco (sqlx) numa mensagem amigável para o usuário.

### ping.rs — `Command::Ping`: responde `Pong` com o uptime.
fn handle(state) -> RpResponse

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

### recent_deployments.rs — `Command::RecentDeployments`: deployments recentes de todos os serviços, com o Live velho corrigido …
fn handle(state, limit) -> RpResponse

### reconcile.rs — Corrige deployments marcados Live que já não estão no ar (`fix_stale_live`), sem tocar em serviço co…
fn fix_stale_live(state, deployments) -> Vec<Deployment> — Passa para Pruning/Stopped os deployments marcados Live que já não estão no ar (só o mais recente de…
fn service_is_deploying(state, service_id) -> bool — True enquanto um deploy do serviço está em curso ou esperando na fila (`ServiceStatus::Deploying`/`Q…
fn transition_deployment(state, dep, to, msg)
fn is_container_running(state, service_id) -> bool
const RUNNING_CHECK_RETRIES, RUNNING_CHECK_DELAY

### regenerate_webhook_token.rs — `Command::RegenerateWebhookToken`: gera um token novo de webhook para o serviço e devolve a URL nova…
fn handle(state, service_id) -> RpResponse

### registry.rs — Sub-aba Docker > Registry: navegação (repositórios/tags), delete e GC do registry OCI embutido (`cra…
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

### secret_delete.rs — `Command::SecretDelete`: apaga um secret do projeto.
fn handle(state, project_id, name) -> Response

### secret_list.rs — `Command::SecretList`: nomes (nunca valores) dos secrets de um projeto.
fn handle(state, project_id) -> Response

### secret_set.rs — `Command::SecretSet`: cria/sobrescreve um secret cifrado do projeto.
fn handle(state, project_id, name, value) -> Response

### service_archive_upload.rs — Upload de `.zip` para serviço com fonte Archive: valida o tamanho e extrai com proteção contra path …
fn handle(state, service_id, bytes, original_filename) -> RpResponse
fn archive_extract_dir(db_path, service_id, archive_id) -> PathBuf
fn extract_zip(bytes, dest) -> Result<()>
fn safe_zip_path(name) -> Option<PathBuf>
const MAX_ZIP_BYTES

### service_connection_info.rs — `Command::ServiceConnectionInfo`: host e connection string internos de um serviço (ver `shared::conn…
fn handle(state, service_id) -> RpResponse

### service_create.rs — `Command::ServiceCreate`: cria um serviço, alocando a porta externa automática e liberando-a no fire…
fn handle(state, spec) -> RpResponse

### service_delete.rs — `Command::ServiceDelete`: apaga um serviço, tira suas rotas do ingress e fecha a porta externa no fi…
fn handle(state, id) -> RpResponse

### service_get.rs — `Command::ServiceGet`: um serviço pelo id.
fn handle(state, id) -> RpResponse

### service_list.rs — `Command::ServiceList`: serviços de um projeto.
fn handle(state, project_id) -> RpResponse

### service_reload.rs — `Command::ServiceReload`: reinicia o container live de um serviço (stop gracioso + start) sem redepl…
fn handle(state, service_id) -> RpResponse

### service_stop.rs — `Command::ServiceStop`: para um serviço (todas as réplicas live, ou a stack Compose) e marca o deplo…
fn handle(state, service_id) -> RpResponse
fn stop_compose(state, service_id, project_name, content, network_name, env_vars) -> RpResponse
fn finish_stop(state, service_id, container_id) -> RpResponse

### service_update.rs — `Command::ServiceUpdate`: grava um ServiceSpec novo (porta externa, TLS dos domínios, firewall) sem …
fn handle(state, id, spec) -> RpResponse
fn sync_firewall(state, id, old, new) — Porta mudou/removida → fecha a antiga (se mais ninguém a usa) e o listener do ingress; porta nova/ma…

### set_daemon_settings.rs — `Command::SetDaemonSettings`: grava e-mail ACME e domínio do registry, ligando/desligando ACME e a r…
fn handle(state, acme_email, registry_domain) -> RpResponse
fn provision_existing_domains(state) — Emite certificados para todos os services já em execução com tls_enabled.
fn save_optional(state, key, value) -> Result<(), RpResponse>

### shared_access.rs — `Command::SharedAccess{List,Grant,Revoke}`: quais projetos alcançam um servidor de banco compartilha…
fn server(state, id) -> Result<Service, RpResponse>
fn view(state, svc) -> Vec<SharedAccess>
fn list(state, server_service_id) -> RpResponse
fn grant(state, server_service_id, project_id) -> RpResponse
fn revoke(state, server_service_id, project_id) -> RpResponse

### wizard.rs — Wizard "Novo serviço" server-side: catálogos (`WizardCatalog`) e criação (`WizardCreate`).
fn catalog(search) -> RpResponse — Catálogos de bancos/brokers/templates prontos como JSON para o contexto do cliente (`ns_dbs`/`ns_bro…
fn create(state, req) -> RpResponse — Monta o `ServiceSpec` a partir dos campos coletados pelo wizard e cria o serviço — reaproveitando o …

## crates/daemon/src/api/

### http_api.rs — HTTP/JSON + SSE control API — the daemon's remote administrative channel.
type ApiBody = BoxBody<Bytes, Infallible> — Unified response body: both the buffered (`Full`) replies and the streaming (`StreamBody`) SSE body …
fn run(state, cfg, tls) — Starts the API listener.
fn serve_conn(io, state, token, peer) — Serves one HTTP/1.1 connection over `io` (plain TCP or a TLS stream).
fn handle(req, state, token, peer) -> Result<Response<ApiBody>, Infallible> — Roteia uma requisição HTTP da API: rotas públicas (webhook, callback OAuth) antes do gate do token B…
fn service_archive_upload(req, state, service_id) -> Response<ApiBody>
fn rpc(req, state) -> Response<ApiBody> — `POST /api/rpc`: decode a `Command`, run it through `dispatch`, encode the `Response` back as JSON.
fn events(state) -> Response<ApiBody> — `GET /api/events`: SSE stream.
fn service_logs(state, service_id) -> Response<ApiBody> — `GET /api/services/{id}/logs`: SSE dedicado aos logs de container de UM serviço.
fn deployment_build_logs(state, deployment_id) -> Response<ApiBody> — `GET /api/deployments/{id}/build-logs`: SSE dedicado à saída de `docker build` de UM deployment.
fn job_run_logs(state, job_run_id) -> Response<ApiBody> — `GET /api/jobs/runs/{id}/logs`: SSE dedicado à saída de UMA execução de job one-shot.
fn sse_response(rx) -> Response<ApiBody> — Response body comum dos endpoints SSE (`events`/`service_logs`/`deployment_build_logs`): drena o `rx…
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
(2 testes)

### mod.rs — API do daemon: `AppState` (estado compartilhado por todos os handlers) e os caches do inventário Doc…
type OAuthStates = Arc<Mutex<HashMap<String, String>>> — Pending OAuth handshakes: CSRF `state` → `provider_id`, consumed by the `/oauth/gitea/callback` rout…
type ActiveDeploys = Arc<Mutex<HashMap<String, tokio::task::AbortHandle… — Handles de abort para deploys activos: deployment_id → AbortHandle.
type ActiveJobs = Arc<Mutex<HashMap<String, tokio::sync::watch::Send… — Sinais de cancelamento para `job_run`s em execução: job_run_id → sender de um `watch<bool>` (valor `…
const DOCKER_CACHE_TTL — How long the host-wide Docker inventory (`docker system df` + the network cross-reference) stays cac…
struct TtlCache { ttl, slot } — Single-slot value cache with a TTL.
impl TtlCache<T>
  fn new(ttl) -> Self
  fn get_or_refresh(refresh) -> Result<T, E> — Returns the cached value if still within the TTL, otherwise runs `refresh`, stores and returns it.
  fn invalidate() — Drops the cached value so the next `get_or_refresh` fetches fresh.
struct DockerCache { df, networks } — Caches the slow host-wide Docker inventory calls so the 2s status poll (and every Docker-tab refresh…
impl DockerCache
  fn new() -> Self
struct AppState { db, docker, ingress, bus, secrets, tls, db_path, backup_dir, drain_secs, api, started_at, oauth_states, active_deploys, active_jobs, deploy_queue, docker_cache, registry_storage, registry_internal_token }
impl AppState
  fn new(db, docker, ingress, bus, secrets, tls, db_path, backup_dir, drain_secs, api, registry_storage, registry_internal_token) -> Self
  fn public_base_url() -> String — URL pública do daemon, sem barra final: base do webhook (`{base}/webhook/{service_id}/{token}`) e do…
fn outbound_ip() -> String — Detecta o IP de saída da máquina conectando um socket UDP em 8.8.8.8:80 (sem enviar dados) e lendo o…

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

### routes.rs — `dispatch`: o `match` que manda cada `Command` para o seu handler em `handlers/`.
fn dispatch(state, cmd) -> RpResponse

### web_ui.rs — Servidor de estáticos da web UI/PWA (`crates/daemon/webui/`) — alternativa ao client iced (`rustploy…
type ApiBody = BoxBody<Bytes, std::convert::Infallible>
struct Asset { route, content_type, etag, no_cache, gz } — Um arquivo do app shell, já processado (minificado + gzipado) em tempo de build — ver `Asset` gerado…
fn serve(path) -> Option<Response<ApiBody>> — Serve `path` se casar com algum asset embutido do app shell; `None` se a rota não pertencer à web UI…
const ASSETS
(7 testes)
