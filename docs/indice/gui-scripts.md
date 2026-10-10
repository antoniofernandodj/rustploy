# Índice: rustploy-gui: demais scripts Luau (estado, rede, formatação, janelas)

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Luau: `function x(a)` = global (handler que o `.gv` chama pelo nome), `local x(a)` = privada, `function M.x(a)` = exportada pelo módulo.

## rustploy-gui/views/scripts/

### app.luau — ponto de entrada do `<script>` de app.gvb.

### busy.luau — trava de "ação em andamento" para os handlers de on_click/on_submit.
function M.key(name)
local seed(names) — Semeia `busy_<nome> = "false"` para toda ação instalada.
local owner()
local release(thread)
function M.install(names)

### busy_actions.luau — ações (on_click/on_submit) que ganham a trava de busy.luau.

### format.luau — fachada: reexporta os builders de views/scripts/fmt_*.luau sob um único módulo, para os handlers seg…

### glacier.d.luau — Definições dos globais que o motor glacier-ui injeta no interpretador Luau em runtime (não existem c…
types: FetchResult, StreamHandle, Viewport, StreamOptions, DateDelta

### helpers.luau — utilitários puros compartilhados pelos handlers_*.luau (sem estado, sem I/O).
function M.trim(value)
function M.parse_secret_ref(text) — Referência a secret num valor de env var → nome do secret (ou nil se o valor for texto comum).
function M.parse_dotenv(text) — Parseia um blob .env em (vars, comments).
function M.oauth_redirect_uri(base, provider_segment) — URI de callback OAuth que o usuário registra no app do provedor.
function M.looks_like_git_url(url) — Heurística: a URL denota um repo Git (clonar+buildar) e não uma imagem.
function M.normalize_url(raw) — Normaliza a entrada de URL do login para uma base HTTP(S) sem barra final.

### log_window.luau — script da JANELA de logs (runtime OU build) de um serviço/deployment, um motor Glacier próprio e ISO…
local set_full() — Set COMPLETO do textarea a partir do buf (usado no seed e no corte).
local push_batch(events) — Aplica um lote de eventos novos: appenda no fim (barato) ou, se estourou o teto, corta a cauda e rec…
local log_event_to_row(event) — Um evento do bus: os endpoints já filtram por id; pegamos o LogLine (runtime), BuildLog (build) ou J…
function init()

### new_job_window.luau — script da JANELA "Novo job", um motor Glacier próprio e ISOLADO do app principal (aberto via open_wi…
local client()
local decode(json_text)
local unpack_recurrence(recurrence) — Recorrência (Option<Recurrence>, externally-tagged) → (kind, hours, hour, minute, weekday), pro form…
function init()
function field(key, value) — onChange dos inputs (mesma convenção `field:<chave>` do app principal).
function new_job_source(kind)
function new_job_git_provider_pick(provider_id)
function new_job_git_repo_pick(full_name)
function new_job_pick_project(project_id)
function new_job_pick_service(service_id)
function new_job_pick_no_service() — Job 100% autônomo: sem serviço gatilho, só as env vars do projeto.
function new_job_back()
function new_job_kind(kind_choice)
function new_job_create()
function cancel()

### new_project_window.luau — script da JANELA de "Novo projeto", um motor Glacier próprio e ISOLADO do app principal (aberto via …
local client() — Reconstrói o cliente da API a partir da conexão semeada no contexto.
function init()
function new_project_show_validation_errors(_erros_json) — on_validation_error do form(...): as falhas já vêm prontas em JSON (`[{campo,msg}]`) e os `{erro_<ca…
function submit_project() — on_submit do form(...): só roda quando a validação (rules="required" no campo NOME) passou, então `n…
function cancel() — Botão "Cancelar": fecha ESTA janela (close_window fecha a janela dona deste motor com precisão, sem …

### new_registry_token_window.luau — script da JANELA "Novo token do registry", um motor Glacier próprio e ISOLADO do app principal (aber…
local client()
function init()
function new_registry_token_show_validation_errors(_erros_json) — on_validation_error do form(...) — NOME é `rules="required"`; o motor já publicou {error_new_registr…
function new_registry_token_create()

### new_service_window.luau — script da JANELA do wizard "Novo serviço", um motor Glacier próprio e ISOLADO do app principal (aber…
function init()
function field(key, value) — Setter genérico de campo (on_change="field:<chave>" dos inputs do wizard).

### service_export_window.luau — script da JANELA "Exportar serviço", um motor Glacier próprio e ISOLADO do app principal (aberto via…
local client()
local render() — Reconstrói a lista exibida (☑/☐ por variável do projeto) a partir de `picked`.
local pick(rule)
function init()
function field(key, value)
function export_toggle(key)
function export_suggested()
function export_all()
function export_none()
local generate() — Pede o pacote ao daemon com as escolhas da tela.
function export_save()
function export_show() — Mostra o pacote como texto (com o botão Copiar) para quem não quer arquivo.

### state.luau — estado mutável compartilhado entre todos os handlers/*.luau (mesmo interpretador, mesma tabela: `req…
types: DeployTrack, State

## rustploy-gui/views/scripts/format/

### dashboard.luau — builders de lista do dashboard (deployments/projects/services/docker/ingress/monitoring/deploy engin…
local grid_cols()
function M.deployments(deployments, term) — Deployments (aba Deployments).
local primary_container(service) — Escolhe o container "primário" de um serviço p/ exibir no card: o live, senão o primeiro em execução…
local service_card(service, project_name, metrics) — Card de serviço (com CPU/mem mesclados de `metrics_by_id[svc.id]`).
function M.services(service_pairs, metrics, term) — Array plano de cards de serviço (para seleção/contagem).
function M.service_rows(service_pairs, metrics, term) — Cards de serviço em linhas de N colunas (grid_cols), com filler (glacier não tem grid).
local project_card(project, service_pairs) — Card de projeto, agregando contagem de serviços de `pairs_`.
function M.projects(projects, service_pairs, term)
function M.project_rows(projects, service_pairs, term)
function M.docker_rows(service_pairs, term) — Linhas Docker (containers gerenciados) derivadas dos serviços, ordenadas.
function M.docker_images(images, term, only_used) — `only_used`: o "Somente em uso" da aba — filtrado aqui porque a tabela (tableview) desenha toda linh…
function M.docker_volumes(volumes, term, only_used)
function M.docker_networks(networks, term, only_used)
function M.docker_containers(containers, term) — Containers do host (DockerContainerInfo): todos, rodando + parados.
function M.ingress(service_pairs) — Ingress: uma linha por rota de domínio (não filtrado pela busca).
function M.host_ports(service_pairs) — Portas TCP de host: uma linha por serviço com `host_port` configurado — exposição direta de porta TC…
function M.monitoring(service_pairs, metrics) — Monitoring: uma linha por serviço COM métricas vivas.
local deploy_step_index(state)
local deploy_stepper(info)
local iso_to_epoch_seconds(iso_timestamp)
function M.deploy_engine_detail(info)
function M.deploy_engine_detail_steps(info)
function M.deploy_engine_active(active) — Deploy Engine: "Executando agora".
function M.deploy_engine_queued(queued) — Deploy Engine: "Na fila" (deploys esperando; o primeiro é o próximo a rodar).
function M.deploy_engine_recent(recent) — Deploy Engine: "Histórico 24h".

### docker_cleanup.luau — resumo textual da limpeza automática de Docker (Settings → Manutenção).
function M.resource_label(name)
function M.last_run_summary(last_run) — `lr`: `DockerCleanupLastRun?` (`{ at, results: { { resource, count, reclaimed_bytes, error } } }`).

### git.luau — builders dos provedores/repositórios/branches Git conectados (Gitea/GitHub) — Settings → Git e o pic…
local kind_label(kind) — Rótulo amigável do tipo de provedor a partir do enum do wire (GitProviderKind).
function M.git_providers(providers)
function M.git_repos(repositories)
function M.git_branches(branches)

### jobs.luau — formata Job/JobSummary/JobRun (tarefas one-shot via docker-compose) pra exibição: tela global "Sched…
function M.recurrence_label(recurrence) — Resumo textual de uma `Recurrence?` (serde externally-tagged: `nil` = só manual; `{IntervalHours=6}`…
function M.run_status(run) — Rótulo/cor (mesmo esquema de StateCell) da última execução de um job.
function M.summaries(job_summaries, inflight) — Tela global "Schedules": uma linha por job, de todos os projetos.
function M.pre_deploy_queue(spec, jobs) — Fila de checks de pré-deploy do serviço (aba Advanced), na ORDEM configurada, com o nome resolvido a…
function M.pre_deploy_available(spec, jobs) — Jobs do projeto que AINDA NÃO estão na fila — opções do `<select>` de "adicionar à fila" (evita dupl…
function M.runs(runs) — Histórico de execuções (`Command::JobRunHistory`) — usado pra listar runs e abrir "Ver logs" (log_wi…

### registry.luau — formata repositórios/tags do registry OCI embutido pra exibição na sub-aba Docker > Registry.
function M.repos(repositories, term) — Lista de repositórios (filtrada pela busca global, como docker_images etc.).
function M.tags(tags) — Tags de UM repositório (sem filtro de busca — lista pequena, buscada sob demanda ao abrir o repositó…
function M.tokens(tokens) — Tokens de acesso (Basic auth) — sem filtro de busca (lista tipicamente pequena, buscada uma vez ao a…

### service_detail.luau — builders da tela de detalhe de serviço (source, healthcheck, env vars, logs, deployments, domains, c…
function M.containers(containers) — Lista de containers do serviço (id + nome + estado) → linhas para a UI de detalhe.
local is_terminal(state)
function M.source_summary_parts(source) — ServiceSource → (kind, detail, build_engine) — source_summary().
function M.healthcheck_summary(healthcheck) — Resumo de uma linha do healthcheck.
local env_var_row(env_var) — Uma linha de var de ambiente (secret exibido por referência).
function M.secret_rows(names) — Secrets do projeto como linhas JSON.
function M.env_json_with_comments(vars, comments) — Env vars como linhas JSON, re-interleaving os comentários `# ...` ancorados (linha `__c<idx>`, idx =…
function M.env_dotenv_with_comments(vars, comments) — Env vars como blob `.env` (KEY=VALUE, secrets por referência), com comentários.
local log_rows(log_entries) — Linhas de log (LogEntry/BuildLogLine: {stream, line, timestamp}) coloridas, limpas de ANSI e limitad…
function M.join_log_lines(log_entries) — Junta linhas de log num blob de texto ("HH:MM:SS line") p/ o TextArea/copiar.
function M.deployments_detail(deployments) — Histórico de deployments de um serviço (aba Deployments).
function M.domains_json(spec) — Rotas HTTP de um spec (aba Domains).
function M.safe_name(name) — normalize_name: minúsculas, não-alfanumérico → '_' (colapsado), trim '_'.
local internal_scheme(db_kind)
function M.compose_host(content, ingress_service) — Chave do serviço que recebe o tráfego dentro de um compose: `ingress_service` se declarado, senão a …
function M.internal_url(db_kind, safe, port, compose_host) — `compose_host`: host do compose (ver `compose_host`) — para serviço Compose o hostname interno é a c…
local url_host(api_url) — Extrai só o host (sem esquema/porta) de uma api_url tipo "https://1.2.3.4:8443".
local env_plain(vars, key) — Valor Plain de uma env var pela chave (secrets/ausentes → nil).
local database_credentials(db_kind, vars) — Credenciais (database/user/password) de um serviço de banco/broker, lidas das env vars nas mesmas co…
local with_database_credentials(base, database, user, password) — Anexa `/database?user=…&password=…` a uma URL de banco, no formato pedido (query-params, ex.: postgr…
local percent_encode(text) — Percent-encode de um componente de userinfo (user/password) — protege chars estruturais do URI (`@`,…
local userinfo(user, password) — Monta a parte de userinfo `user:pass@` de uma URI RFC 3986. Casos: user+pass → "user:pass@"  |  só p…
local external_scheme(database_kind) — Esquema de URI externo por banco (userinfo).

### time.luau — timestamps e durações.
function M.time_hms(iso_timestamp) — HH:MM:SS local de um timestamp do daemon.
function M.date_day_month_hour_minute(iso_timestamp) — "dd/mm HH:MM" local.
function M.date_day_month_hour_minute_second(iso_timestamp) — "dd/mm HH:MM:SS" local.
function M.format_seconds(seconds) — Ns ou Mm Ns.
function M.format_uptime(seconds)
function M.format_bytes(bytes_value) — Tamanho de bytes legível ("—" para 0), igual a view::format_bytes.
function M.format_duration(deployment) — Duração de um deployment (finished_at ou agora) − started_at, em format_seconds.
function M.hour_minute_join(hour, minute) — (hour, minute) -> "HH:MM".
function M.hour_minute_split(hour_minute_text) — "HH:MM" -> hour, minute (números).

### types.luau — tipos compartilhados entre os submódulos fmt_*.luau (as formas dos modelos que chegam via `json.deco…
types: EnvValue, EnvVar, EnvComment, DomainRoute, ServiceSpec, ManagedContainer, Service, MetricEntry, ServicePair, GridCard, MetricsMap

### util.luau — busca, codificação de array e mapas de estado (paleta = view.rs).
function M.matches(term, fields) — ── Busca ──────────────────────────────────────────────────────────────── Termo vazio sempre casa (a…
function M.ellipsis(text, max) — ── Texto ──────────────────────────────────────────────────────────────── Trunca `s` para no máximo …
function M.short_reason(message, max) — Motivo de falha vindo do daemon (`DeployStateChanged.message`, `ServiceStatus::Error(...)`) reduzido…
function M.column_budgets()
function M.strip_ansi(text) — Remove sequências de escape ANSI (cor/cursor/erase) das linhas de log.
function M.encode_array(items) — ── Codificação de arrays ──────────────────────────────────────────────── CUIDADO: uma tabela Luau v…
function M.status_label_color(status) — ServiceStatus vem como string ("Running") ou tabela ({Error="..."}).
function M.status_kind(status)
function M.state_label_color(state) — DeployState vem como string ("Live", "Failed", "BuildingImage", ...).
function M.state_kind(state)
function M.in_use_kind(in_use)
function M.container_state(state) — Estado bruto de um container Docker ("running"/"exited"/… ) → (rótulo, kind).
function M.container_stopped(state) — `true` quando o container está PARADO (removível sem force).
function M.source_summary(source) — ServiceSource → texto de "imagem/origem" (source_summary().1).
function M.domain_routes(spec) — domain_routes(): a lista `domains` nova, ou o legado `domain`/`tls_enabled`.
function M.pre_deploy_checks(spec) — Fila efetiva de pré-deploy check: `pre_deploy_job_ids` quando não vazia, senão cai no `pre_deploy_jo…
function M.service_pair_list(services) — `services` do snapshot é uma lista de { project_name, service }.
types: ColBudgets

## rustploy-gui/views/scripts/net/

### api.luau — cliente HTTP/JSON da API do daemon.
function M.auth_headers(token) — - Só o header `Authorization` (tabela vazia quando não há token).
function M:headers()
function M:rpc(command) — - Executa um Command.
function M:rpc_checked(command) — - Como `rpc`, mas trata `Response::Err { code, message }` como falha.
function M.new(base_url, token)
types: Client
