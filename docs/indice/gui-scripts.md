# Índice: rustploy-gui: demais scripts Luau (estado, rede, formatação, janelas)

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Luau: `function x(a)` = global (handler que o `.gv` chama pelo nome), `local x(a)` = privada, `function M.x(a)` = exportada pelo módulo.

## rustploy-gui/views/scripts/

### app.luau — ponto de entrada do `<script>` de app.gvb.

### busy.luau — trava de "ação em andamento" para os handlers de on_click/on_submit.
function M.key(name)
local seed(names) — Semeia `busy_<nome> = "false"` para toda ação instalada.
function M.install(names)

### busy_actions.luau — ações (on_click/on_submit) que ganham a trava de busy.luau.

### fmt.luau — fachada: reexporta os builders de views/scripts/fmt_*.luau sob um único módulo, para os handlers seg…

### glacier.d.luau — Definições dos globais que o motor glacier-ui injeta no interpretador Luau em runtime (não existem c…
types: FetchResult, StreamHandle, Viewport, StreamOptions, DateDelta

### helpers.luau — utilitários puros compartilhados pelos handlers_*.luau (sem estado, sem I/O).
function M.trim(s)
function M.parse_secret_ref(v) — Referência a secret num valor de env var → nome do secret (ou nil se o valor for texto comum).
function M.parse_dotenv(text) — Parseia um blob .env em (vars, comments).
function M.oauth_redirect_uri(base, seg) — URI de callback OAuth que o usuário registra no app do provedor.
function M.looks_like_git_url(u) — Heurística: a URL denota um repo Git (clonar+buildar) e não uma imagem.
function M.normalize_url(raw) — Normaliza a entrada de URL do login para uma base HTTP(S) sem barra final.

### log_window.luau — script da JANELA de logs (runtime OU build) de um serviço/deployment, um motor Glacier próprio e ISO…
local set_full() — Set COMPLETO do textarea a partir do buf (usado no seed e no corte).
local push_batch(events) — Aplica um lote de eventos novos: appenda no fim (barato) ou, se estourou o teto, corta a cauda e rec…
local to_row(ev) — Um evento do bus: os endpoints já filtram por id; pegamos o LogLine (runtime), BuildLog (build) ou J…
function init()

### new_job_window.luau — script da JANELA "Novo job", um motor Glacier próprio e ISOLADO do app principal (aberto via open_wi…
local client()
local decode(s)
local unpack_recurrence(r) — Recorrência (Option<Recurrence>, externally-tagged) → (kind, hours, hour, minute, weekday), pro form…
function init()
function field(key, v) — onChange dos inputs (mesma convenção `field:<chave>` do app principal).
function njob_source(kind)
function njob_git_provider_pick(id)
function njob_git_repo_pick(full_name)
function njob_pick_project(id)
function njob_pick_service(id)
function njob_pick_no_service() — Job 100% autônomo: sem serviço gatilho, só as env vars do projeto.
function njob_back()
function njob_kind(k)
function njob_create()
function cancel()

### new_project_window.luau — script da JANELA de "Novo projeto", um motor Glacier próprio e ISOLADO do app principal (aberto via …
local client() — Reconstrói o cliente da API a partir da conexão semeada no contexto.
function init()
function np_apontar(_erros_json) — on_validation_error do form(...): as falhas já vêm prontas em JSON (`[{campo,msg}]`) e os `{erro_<ca…
function submit_project() — on_submit do form(...): só roda quando a validação (rules="required" no campo NOME) passou, então `n…
function cancel() — Botão "Cancelar": fecha ESTA janela (close_window fecha a janela dona deste motor com precisão, sem …

### new_registry_token_window.luau — script da JANELA "Novo token do registry", um motor Glacier próprio e ISOLADO do app principal (aber…
local client()
function init()
function ntok_apontar(_erros_json) — on_validation_error do form(...) — NOME é `rules="required"`; o motor já publicou {erro_ntok_name} e…
function ntok_create()

### new_service_window.luau — script da JANELA do wizard "Novo serviço", um motor Glacier próprio e ISOLADO do app principal (aber…
function init()
function field(key, v) — Setter genérico de campo (on_change="field:<chave>" dos inputs do wizard).

### state.luau — estado mutável compartilhado entre todos os handlers/*.luau (mesmo interpretador, mesma tabela: `req…
types: DeployTrack, State

## rustploy-gui/views/scripts/fmt/

### dashboard.luau — builders de lista do dashboard (deployments/projects/services/docker/ingress/monitoring/deploy engin…
local grid_cols()
function M.deployments(list, term) — Deployments (aba Deployments).
local primary_container(svc) — Escolhe o container "primário" de um serviço p/ exibir no card: o live, senão o primeiro em execução…
local service_card(svc, proj, metrics) — Card de serviço (com CPU/mem mesclados de `metrics_by_id[svc.id]`).
function M.services(pairs_, metrics, term) — Array plano de cards de serviço (para seleção/contagem).
function M.service_rows(pairs_, metrics, term) — Cards de serviço em linhas de N colunas (grid_cols), com filler (glacier não tem grid).
local project_card(p, pairs_) — Card de projeto, agregando contagem de serviços de `pairs_`.
function M.projects(list, pairs_, term)
function M.project_rows(list, pairs_, term)
function M.docker_rows(pairs_, term) — Linhas Docker (containers gerenciados) derivadas dos serviços, ordenadas.
function M.docker_images(list, term, only_used) — `only_used`: o "Somente em uso" da aba — filtrado aqui porque a tabela (tableview) desenha toda linh…
function M.docker_volumes(list, term, only_used)
function M.docker_networks(list, term, only_used)
function M.docker_containers(list, term) — Containers do host (DockerContainerInfo): todos, rodando + parados.
function M.ingress(pairs_) — Ingress: uma linha por rota de domínio (não filtrado pela busca).
function M.host_ports(pairs_) — Portas TCP de host: uma linha por serviço com `host_port` configurado — exposição direta de porta TC…
function M.monitoring(pairs_, metrics) — Monitoring: uma linha por serviço COM métricas vivas.
local step_of(state)
local deploy_stepper(info)
local iso_epoch(iso)
function M.eng_detail(info)
function M.eng_detail_steps(info)
function M.eng_active(active) — Deploy Engine: "Executando agora".
function M.eng_queued(queued) — Deploy Engine: "Na fila" (deploys esperando; o primeiro é o próximo a rodar).
function M.eng_recent(recent) — Deploy Engine: "Histórico 24h".

### docker_cleanup.luau — resumo textual da limpeza automática de Docker (Settings → Manutenção).
function M.resource_label(name)
function M.last_run_summary(lr) — `lr`: `DockerCleanupLastRun?` (`{ at, results: { { resource, count, reclaimed_bytes, error } } }`).

### git.luau — builders dos provedores/repositórios/branches Git conectados (Gitea/GitHub) — Settings → Git e o pic…
local kind_label(kind) — Rótulo amigável do tipo de provedor a partir do enum do wire (GitProviderKind).
function M.git_providers(list)
function M.git_repos(list)
function M.git_branches(list)

### jobs.luau — formata Job/JobSummary/JobRun (tarefas one-shot via docker-compose) pra exibição: tela global "Sched…
function M.recurrence_label(r) — Resumo textual de uma `Recurrence?` (serde externally-tagged: `nil` = só manual; `{IntervalHours=6}`…
function M.run_status(run) — Rótulo/cor (mesmo esquema de StateCell) da última execução de um job.
function M.summaries(list, inflight) — Tela global "Schedules": uma linha por job, de todos os projetos.
function M.pre_deploy_queue(spec, list) — Fila de checks de pré-deploy do serviço (aba Advanced), na ORDEM configurada, com o nome resolvido a…
function M.pre_deploy_available(spec, list) — Jobs do projeto que AINDA NÃO estão na fila — opções do `<select>` de "adicionar à fila" (evita dupl…
function M.runs(list) — Histórico de execuções (`Command::JobRunHistory`) — usado pra listar runs e abrir "Ver logs" (log_wi…

### registry.luau — formata repositórios/tags do registry OCI embutido pra exibição na sub-aba Docker > Registry.
function M.repos(list, term) — Lista de repositórios (filtrada pela busca global, como docker_images etc.).
function M.tags(list) — Tags de UM repositório (sem filtro de busca — lista pequena, buscada sob demanda ao abrir o repositó…
function M.tokens(list) — Tokens de acesso (Basic auth) — sem filtro de busca (lista tipicamente pequena, buscada uma vez ao a…

### service_detail.luau — builders da tela de detalhe de serviço (source, healthcheck, env vars, logs, deployments, domains, c…
function M.containers(list) — Lista de containers do serviço (id + nome + estado) → linhas para a UI de detalhe.
local is_terminal(state)
function M.source_summary3(source) — ServiceSource → (kind, detail, build_engine) — source_summary().
function M.healthcheck_summary(hc) — Resumo de uma linha do healthcheck.
local env_var_row(v) — Uma linha de var de ambiente (secret exibido por referência).
function M.secret_rows(names) — Secrets do projeto como linhas JSON.
function M.env_json_with_comments(vars, comments) — Env vars como linhas JSON, re-interleaving os comentários `# ...` ancorados (linha `__c<idx>`, idx =…
function M.env_dotenv_with_comments(vars, comments) — Env vars como blob `.env` (KEY=VALUE, secrets por referência), com comentários.
local log_rows(list) — Linhas de log (LogEntry/BuildLogLine: {stream, line, timestamp}) coloridas, limpas de ANSI e limitad…
function M.join_log_lines(list) — Junta linhas de log num blob de texto ("HH:MM:SS line") p/ o TextArea/copiar.
function M.deployments_detail(list) — Histórico de deployments de um serviço (aba Deployments).
function M.domains_json(spec) — Rotas HTTP de um spec (aba Domains).
function M.safe_name(name) — normalize_name: minúsculas, não-alfanumérico → '_' (colapsado), trim '_'.
local internal_scheme(db_kind)
function M.compose_host(content, ingress_service) — Chave do serviço que recebe o tráfego dentro de um compose: `ingress_service` se declarado, senão a …
function M.internal_url(db_kind, safe, port, compose_host) — `compose_host`: host do compose (ver `compose_host`) — para serviço Compose o hostname interno é a c…
local url_host(api_url) — Extrai só o host (sem esquema/porta) de uma api_url tipo "https://1.2.3.4:8443".
local env_plain(vars, key) — Valor Plain de uma env var pela chave (secrets/ausentes → nil).
local db_credentials(db_kind, vars) — Credenciais (database/user/password) de um serviço de banco/broker, lidas das env vars nas mesmas co…
local with_db_credentials(base, database, user, password) — Anexa `/database?user=…&password=…` a uma URL de banco, no formato pedido (query-params, ex.: postgr…
local pct(s) — Percent-encode de um componente de userinfo (user/password) — protege chars estruturais do URI (`@`,…
local userinfo(user, password) — Monta a parte de userinfo `user:pass@` de uma URI RFC 3986. Casos: user+pass → "user:pass@"  |  só p…
local external_scheme(k) — Esquema de URI externo por banco (userinfo).

### time.luau — timestamps e durações.
function M.time_hms(iso) — HH:MM:SS local de um timestamp do daemon.
function M.date_dm_hm(iso) — "dd/mm HH:MM" local.
function M.date_dm_hms(iso) — "dd/mm HH:MM:SS" local.
function M.fmt_secs(secs) — Ns ou Mm Ns.
function M.fmt_uptime(secs)
function M.fmt_bytes(b) — Tamanho de bytes legível ("—" para 0), igual a view::fmt_bytes.
function M.fmt_duration(dep) — Duração de um deployment (finished_at ou agora) − started_at, em fmt_secs.
function M.hm_join(hour, minute) — (hour, minute) -> "HH:MM".
function M.hm_split(hm) — "HH:MM" -> hour, minute (números).

### types.luau — tipos compartilhados entre os submódulos fmt_*.luau (as formas dos modelos que chegam via `json.deco…
types: EnvValue, EnvVar, EnvComment, DomainRoute, ServiceSpec, ManagedContainer, Service, MetricEntry, ServicePair, GridCard, MetricsMap

### util.luau — busca, codificação de array e mapas de estado (paleta = view.rs).
function M.matches(term, fields) — ── Busca ──────────────────────────────────────────────────────────────── Termo vazio sempre casa (a…
function M.ellipsis(s, max) — ── Texto ──────────────────────────────────────────────────────────────── Trunca `s` para no máximo …
function M.short_reason(msg, max) — Motivo de falha vindo do daemon (`DeployStateChanged.message`, `ServiceStatus::Error(...)`) reduzido…
function M.col_budgets()
function M.strip_ansi(s) — Remove sequências de escape ANSI (cor/cursor/erase) das linhas de log.
function M.enc_array(t) — ── Codificação de arrays ──────────────────────────────────────────────── CUIDADO: uma tabela Luau v…
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
function M.pair_list(services) — `services` do snapshot é uma lista de { project_name, service }.
types: ColBudgets

## rustploy-gui/views/scripts/net/

### api.luau — cliente HTTP/JSON da API do daemon.
function M.auth_headers(token) — - Só o header `Authorization` (tabela vazia quando não há token).
function M:headers()
function M:rpc(cmd) — - Executa um Command.
function M:rpc_checked(cmd) — - Como `rpc`, mas trata `Response::Err { code, message }` como falha.
function M:upload_archive(service_id, filename, body_base64)
function M.new(base_url, token)
types: Client
