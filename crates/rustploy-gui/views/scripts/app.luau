-- app.luau — cérebro do cliente rustploy-gui em Luau (sucessor de app/root.rs +
-- app/net/*). Hospeda TODA a lógica de rede reativa a cliques: login, navegação,
-- busca, o consumidor SSE (snapshot + eventos vivos) e as ações mutantes.
--
-- Como o contexto do glacier é único e global, este é o único `<script>` ativo:
-- os templates importados (login/shell/home/service) só têm markup; seus
-- onClick/onChange resolvem para as funções globais definidas aqui.
--
-- Rede: `net/api` encapsula POST /api/rpc; `fmt` reimplementa view.rs (as
-- mesmas formas JSON que as listas iteram). O estado vem por SSE em /api/events:
-- um `snapshot` a cada 2s (substitui o antigo poll) + eventos `bus` vivos.

local Api = require("net/api")
local fmt = require("fmt")

-- ── Estado do módulo (persiste entre ações; mesmo interpretador) ──────────
local api = nil              -- cliente Api conectado
local state_conn = nil       -- handle do stream SSE
local snap = nil             -- último snapshot decodificado (para refiltrar)
local last_pairs = {}        -- serviços normalizados { svc, proj } do snapshot
local metrics_by_id = {}     -- service_id → ContainerMetricsPoint (eventos bus)
local search_term = ""       -- termo de busca corrente (minúsculas)
local svc_logs_buf = {}      -- ring de logs vivos do serviço aberto (LogLine)
local blog_buf = {}          -- ring de build logs do deployment selecionado
local deploy_track = { service_id = "", deployment_id = "", running = false }
local ns_templates_all = {}  -- catálogo completo de templates (p/ filtrar a busca)

local LOG_RING = 200
local BUILD_RING = 2000

-- ── Helpers ──────────────────────────────────────────────────────────────

-- Normaliza a entrada de URL do login para uma base HTTP(S) sem barra final.
-- Aceita "https://host:porta", "http://host:porta", "host:porta", "host".
-- Sem esquema → https. Outros esquemas (ex.: ws, ftp) são rejeitados.
local function normalize_url(raw: string): string | nil
    local u = (raw or ""):gsub("^%s+", ""):gsub("%s+$", "")
    if u == "" then return nil end
    local scheme, rest = string.match(u, "^([%a][%w+.-]*)://(.*)$")
    if scheme then
        scheme = string.lower(scheme)
        if scheme ~= "http" and scheme ~= "https" then return nil end
    else
        scheme, rest = "https", u
    end
    -- corta path/query/fragment: só queremos o authority (host[:porta]).
    rest = string.match(rest or "", "^([^/?#]*)") or rest
    if rest == "" then return nil end
    return (scheme or "") .. "://" .. (rest or "")
end

local function trim(s) return (tostring(s or ""):gsub("^%s+", ""):gsub("%s+$", "")) end

-- Toasts (built-in do glacier 0.20): notificação efêmera, complementa o *_msg
-- inline. Usados nos desfechos de mutação (erro/sucesso).
local function notify_err(msg) toast({ message = tostring(msg), kind = "error" }) end
local function notify_ok(msg) toast({ message = tostring(msg), kind = "success" }) end

-- URI de callback OAuth do Gitea que o usuário registra no app do Gitea.
local function oauth_redirect_uri(domain)
    local d = trim(domain):gsub("/+$", "")
    if d == "" then return "<configure o domínio em Web Server>/oauth/gitea/callback" end
    return d .. "/oauth/gitea/callback"
end

-- Heurística: a URL denota um repo Git (clonar+buildar) e não uma imagem.
local function looks_like_git_url(u)
    u = trim(u)
    return u:match("^https://") ~= nil or u:match("^http://") ~= nil
        or u:match("^git@") ~= nil or u:match("^ssh://") ~= nil
        or u:match("^file://") ~= nil or u:match("%.git$") ~= nil
end

-- ── init: semeia o contexto para a UI pintar antes de qualquer dado ───────
function init()
    if not ctx.screen or ctx.screen == "" then ctx.screen = "login" end
    if not ctx.view or ctx.view == "" then ctx.view = "deployments" end
    ctx.connected = ctx.connected or "false"
    ctx.status_line = ctx.status_line or "pronto para conectar"
    ctx.error = ctx.error or ""
    ctx.search = ctx.search or ""
    ctx.data_loading = "true"
    -- Listas vazias como "[]" (não nil) para os ForEach não verem literal.
    for _, k in ipairs({
        "deployments", "projects", "project_rows", "services", "service_rows",
        "docker_rows", "docker_images", "docker_volumes", "docker_networks",
        "ingress", "monitoring", "eng_active", "eng_recent", "project_services",
    }) do
        if not ctx[k] or ctx[k] == "" then ctx[k] = "[]" end
    end
    for _, k in ipairs({
        "deployments_count", "projects_count", "services_count", "ingress_count",
        "docker_images_count", "docker_volumes_count", "docker_networks_count",
        "eng_active_count", "eng_recent_count", "eng_success_24h", "eng_failed_24h",
        "eng_total_24h", "services_running", "services_total",
    }) do
        if not ctx[k] or ctx[k] == "" then ctx[k] = "0" end
    end
    ctx.remember_url = ctx.remember_url or "false"
    ctx.remember_token = ctx.remember_token or "false"
end

-- ── Login / conexão ───────────────────────────────────────────────────────

function toggle_remember_url(v)
    ctx.remember_url = v
end
function toggle_remember_token(v)
    ctx.remember_token = v
end

-- onSubmit/onClick do formulário de login. `fetch` suspende a corrotina, então
-- validamos a conexão com um DaemonStatus antes de abrir o stream.
function connect()
    local base = normalize_url(ctx.url)
    if not base then
        ctx.erro_url = "URL inválida (ex.: https://rustploy.chiquitos.tech)"
        return
    end
    ctx.erro_url = ""
    local token = ctx.token or ""
    ctx.api_url = base
    ctx.api_token = token
    ctx.status_line = "conectando…"
    ctx.error = ""

    api = Api.new(base, token)
    local ok, res = api:rpc("DaemonStatus")
    if not ok then
        api = nil
        ctx.error = res
        ctx.status_line = "falha na conexão"
        ctx.connected = "false"
        return
    end

    ctx.connected = "true"
    ctx.status_line = "conectado"
    ctx.screen = "shell"
    if not ctx.view or ctx.view == "" then ctx.view = "deployments" end

    load_settings()
    open_stream()
end

function disconnect()
    if state_conn then
        pcall(function() state_conn:close() end)
        state_conn = nil
    end
    api = nil
    snap = nil
    ctx.connected = "false"
    ctx.screen = "login"
    ctx.status_line = "desconectado"
end

-- Configurações do daemon + provedores Git, buscados uma vez na conexão (não
-- entram no snapshot para não sobrescrever campos que o usuário edita).
function load_settings()
    if not api then return end
    local ok, res = api:rpc("GetDaemonSettings")
    if ok and res.DaemonSettings then
        local domain = res.DaemonSettings.webhook_base_url or ""
        ctx.ss_domain = domain
        ctx.ss_email = res.DaemonSettings.acme_email or ""
        ctx.gp_redirect = oauth_redirect_uri(domain)
    end
    local ok2, res2 = api:rpc("GitProviderList")
    if ok2 and res2.GitProviders then
        ctx.gitea_count = tostring(#res2.GitProviders)
    end
end

-- ── Stream SSE (snapshot periódico + eventos vivos) ───────────────────────

function open_stream()
    if state_conn then return end
    local opts = { on_message = "on_state", on_error = "on_stream_error", on_close = "on_stream_close" }
    if ctx.api_token and ctx.api_token ~= "" then
        opts.headers = { Authorization = "Bearer " .. ctx.api_token }
    end
    state_conn = sse(ctx.api_url .. "/api/events", opts)
end

function on_stream_error(e)
    ctx.status_line = "stream: " .. tostring(e)
end

function on_stream_close()
    state_conn = nil
    if ctx.connected == "true" then
        ctx.status_line = "conexão encerrada"
        ctx.connected = "false"
        ctx.screen = "login"
    end
end

-- Cada evento SSE traz um `data:` JSON auto-descritivo com `kind`.
function on_state(data)
    local ok, msg = pcall(json.decode, data)
    if not ok or type(msg) ~= "table" then return end
    if msg.kind == "snapshot" then
        apply_snapshot(msg)
    elseif msg.kind == "bus" then
        apply_bus(msg.event)
    end
end

function apply_snapshot(msg)
    snap = msg
    last_pairs = fmt.pair_list(msg.services)

    -- Status do daemon.
    local st = msg.status
    if st then
        ctx.daemon_version = st.version or ""
        ctx.daemon_uptime = fmt.fmt_uptime(st.uptime_secs)
        ctx.services_running = tostring(st.services_running or 0)
        ctx.services_total = tostring(st.services_total or 0)
        ctx.services_label = tostring(st.services_running or 0) .. "/" .. tostring(st.services_total or 0)
    end

    -- Deploy Engine (inline, não filtrado pela busca).
    local eng = msg.engine
    if eng then
        ctx.eng_active_count = tostring(#(eng.active or {}))
        ctx.eng_success_24h = tostring(eng.successful_24h or 0)
        ctx.eng_failed_24h = tostring(eng.failed_24h or 0)
        ctx.eng_total_24h = tostring(eng.total_24h or 0)
        ctx.eng_uptime = fmt.fmt_uptime(eng.uptime_secs)
        ctx.eng_active = fmt.eng_active(eng.active)
        ctx.eng_recent = fmt.eng_recent(eng.recent)
        ctx.eng_recent_count = tostring(#(eng.recent or {}))

        -- Timer decorrido do deploy em voo do serviço aberto (granularidade de
        -- 2s — sem timer no Luau, vem do próprio snapshot; ver plano). O desfecho
        -- terminal vem por DeployStateChanged (apply_bus).
        if deploy_track.running and ctx.selected_service == deploy_track.service_id then
            for _, a in ipairs(eng.active or {}) do
                if a.service_id == deploy_track.service_id then
                    ctx.svc_deploy_running = "true"
                    ctx.svc_deploy_elapsed = fmt.fmt_secs(a.elapsed_secs)
                    break
                end
            end
        end
    end

    -- Contagens.
    ctx.deployments_count = tostring(#(msg.deployments or {}))
    ctx.projects_count = tostring(#(msg.projects or {}))
    ctx.services_count = tostring(#last_pairs)
    ctx.docker_images_count = tostring(#(msg.docker_images or {}))
    ctx.docker_volumes_count = tostring(#(msg.docker_volumes or {}))
    ctx.docker_networks_count = tostring(#(msg.docker_networks or {}))

    -- Ingress + Monitoring (não filtrados pela busca).
    local ingress, icount = fmt.ingress(last_pairs)
    ctx.ingress = ingress
    ctx.ingress_count = tostring(icount)
    ctx.monitoring = fmt.monitoring(last_pairs, metrics_by_id)

    rebuild_lists()
    update_open_project()
    ctx.data_loading = "false"
end

-- Reconstrói as listas filtradas pela busca a partir do último snapshot — sem
-- rede, para a busca filtrar na hora (equivalente ao antigo search_pairs).
function rebuild_lists()
    if not snap then return end
    local term = search_term
    ctx.deployments = fmt.deployments(snap.deployments, term)
    ctx.projects = fmt.projects(snap.projects, last_pairs, term)
    ctx.project_rows = fmt.project_rows(snap.projects, last_pairs, term)
    ctx.services = fmt.services(last_pairs, metrics_by_id, term)
    ctx.service_rows = fmt.service_rows(last_pairs, metrics_by_id, term)
    ctx.docker_rows = fmt.docker_rows(last_pairs, term)
    ctx.docker_images = fmt.docker_images(snap.docker_images, term)
    ctx.docker_volumes = fmt.docker_volumes(snap.docker_volumes, term)
    ctx.docker_networks = fmt.docker_networks(snap.docker_networks, term)
end

-- Cabeçalho + grade de serviços do projeto aberto (view=project_services).
function update_open_project()
    if not snap then return end
    local pid = ctx.selected_project_id or ""
    if pid == "" then return end
    local proj
    for _, p in ipairs(snap.projects or {}) do
        if p.id == pid then proj = p break end
    end
    if not proj then return end
    local proj_pairs = {}
    for _, e in ipairs(last_pairs) do
        if e.svc.spec.project_id == pid then table.insert(proj_pairs, e) end
    end
    ctx.proj_name = proj.name
    ctx.proj_description = proj.description or ""
    ctx.proj_can_delete = (#proj_pairs == 0) and "1" or "0"
    ctx.proj_loading = "false"
    ctx.project_services = fmt.service_rows(proj_pairs, metrics_by_id, search_term)
    -- Variáveis de ambiente do projeto (aba "Variáveis (N)"). Sem isto, a aba
    -- nasce "Variáveis ()" até a primeira edição.
    local penv = proj.env_vars or {}
    ctx.proj_env = fmt.env_json_with_comments(penv, {})
    ctx.proj_env_count = tostring(#penv)
end

-- Refresh imediato após uma mutação: pega o snapshot completo num único RPC (o
-- mesmo builder do SSE) e aplica, para a tela refletir na hora sem esperar o
-- tick de 2s. Chamado pelos handlers de criar/remover projeto/serviço.
function refresh_now()
    if not api then return end
    local ok, res = api:rpc("Snapshot")
    if ok and res.Snapshot then
        local okd, m = pcall(json.decode, res.Snapshot)
        if okd and type(m) == "table" then apply_snapshot(m) end
    end
end

-- Eventos vivos do bus: métricas, logs de container/build e transições de deploy.
function apply_bus(ev)
    if type(ev) ~= "table" then return end
    if ev.ContainerMetrics then
        local p = ev.ContainerMetrics
        if p.service_id then metrics_by_id[p.service_id] = p end
    elseif ev.SystemMetrics then
        local s = ev.SystemMetrics
        ctx.sys_cpu = string.format("%.0f%%", s.cpu_percent or 0)
        ctx.sys_mem = fmt.fmt_bytes(s.mem_used_bytes) .. " / " .. fmt.fmt_bytes(s.mem_total_bytes)
        ctx.sys_disk = fmt.fmt_bytes(s.disk_used_bytes) .. " / " .. fmt.fmt_bytes(s.disk_total_bytes)
        ctx.sys_load = string.format("%.2f %.2f %.2f", s.load_avg_1 or 0, s.load_avg_5 or 0, s.load_avg_15 or 0)
    elseif ev.LogLine then
        local e = ev.LogLine
        if e.service_id == ctx.selected_service then
            table.insert(svc_logs_buf, { stream = e.stream, line = e.line, timestamp = e.timestamp })
            while #svc_logs_buf > LOG_RING do table.remove(svc_logs_buf, 1) end
            ctx.svc_logs = fmt.logs_json(svc_logs_buf)
            ctx.svc_logs_count = tostring(#svc_logs_buf)
            ctx.svc_logs_text = fmt.join_log_lines(svc_logs_buf)
        end
    elseif ev.BuildLog then
        local e = ev.BuildLog
        if e.deployment_id == ctx.dep_selected then
            table.insert(blog_buf, { stream = "Stdout", line = e.line, timestamp = e.timestamp })
            while #blog_buf > BUILD_RING do table.remove(blog_buf, 1) end
            ctx.dep_build_logs = fmt.build_logs_json(blog_buf)
            ctx.dep_build_count = tostring(#blog_buf)
            ctx.dep_build_text = fmt.join_log_lines(blog_buf)
        end
    elseif ev.DeployStateChanged then
        local d = ev.DeployStateChanged
        if d.service_id == deploy_track.service_id then
            local terminal = d.state == "Live" or d.state == "Failed" or d.state == "Stopped"
            if terminal then
                deploy_track.running = false
                local live = d.state == "Live"
                if d.service_id == ctx.selected_service then
                    ctx.svc_deploy_running = "false"
                    ctx.svc_action_msg = live and "deploy concluído" or ("deploy: " .. tostring(d.state))
                    ctx.svc_action_color = live and "#3FB950" or "#F85149"
                    fetch_service_detail(d.service_id)
                end
            elseif d.service_id == ctx.selected_service then
                ctx.svc_deploy_running = "true"
                ctx.svc_action_msg = "deploy · " .. tostring(d.state)
                ctx.svc_action_color = "#58A6FF"
            end
        end
    end
end

-- ── Busca ─────────────────────────────────────────────────────────────────

function search_changed(v: string)
    ctx.search = v or ""
    search_term = string.lower(v or "")
    rebuild_lists()
    update_open_project()
end

-- ── Navegação ─────────────────────────────────────────────────────────────

local function goto_view(v)
    ctx.view = v
    ctx.selected_service = ""
end

function nav_deployments() goto_view("deployments") end
function nav_deploy_engine() goto_view("deploy_engine") end
function nav_monitoring() goto_view("monitoring") end
function nav_schedules() goto_view("schedules") end
function nav_ingress() goto_view("ingress") end
function nav_docker() goto_view("docker") end
function nav_projects()
    ctx.selected_project_id = ""
    goto_view("projects")
end
function nav_settings() goto_view("settings") end
function nav_support() goto_view("support") end

-- Genérico p/ NavItem que usa action="nav" com value=target (se algum usar).
function nav(v) if v then goto_view(v) end end

-- Abas (TabButton dispara `proj_tab:services`, `docker_tab:images`, …).
function proj_tab(t) ctx.proj_tab = t end
function docker_tab(t) ctx.docker_tab = t end
function deploy_tab(t) ctx.deploy_tab = t end

-- Setter genérico de campo (onChange="field:<chave>").
function field(key, v) if key then ctx[key] = v or "" end end

-- Abrir a lista de serviços de um projeto (grade read-only via snapshot).
function open_project(id)
    ctx.selected_project_id = id
    ctx.view = "project_services"
    ctx.proj_tab = "services"
    ctx.editing_project = "false"
    ctx.proj_action_msg = ""
    ctx.proj_loading = "true"
    update_open_project()
end

-- Abas do service detail e do healthcheck kind.
function tab(t) ctx.tab = t end
function prov(t) ctx.prov_tab = t end
function hckind(k) ctx.f_hc_kind = k end

-- Abrir o detalhe de um serviço: navega e dispara o fetch (spec + logs +
-- deployments + provedores Git). O fetch suspende a corrotina (fetch), sem
-- travar a UI; svc_loading=true segura os Selects até tudo chegar.
function open_service(id)
    ctx.selected_service = id
    ctx.view = "service"
    ctx.tab = "general"
    ctx.svc_error = ""
    ctx.dep_selected = ""
    ctx.svc_action_msg = ""
    ctx.svc_action_color = "#8B949E"
    blog_buf = {}
    svc_logs_buf = {}
    if deploy_track.service_id ~= id then
        ctx.svc_deploy_running = "false"
        ctx.svc_deploy_elapsed = ""
    end
    ctx.svc_loading = "true"
    fetch_service_detail(id)
end

-- One-shot: monta TODAS as chaves svc_*/f_* do painel de detalhe (equivalente a
-- net/services.rs::fetch_detail_inner).
function fetch_service_detail(sid)
    if not api or not sid or sid == "" then return end
    local ok, res = api:rpc({ ServiceGet = { id = sid } })
    if not ok or not res.Service then
        ctx.svc_loading = "false"
        ctx.svc_error = ok and "serviço não encontrado" or res
        return
    end
    local svc = res.Service
    local spec = svc.spec

    -- Nome do projeto (ServiceGet só traz o id).
    local project_name = spec.project_id
    if snap then
        for _, p in ipairs(snap.projects or {}) do
            if p.id == spec.project_id then project_name = p.name break end
        end
    end

    -- Logs recentes (seed do ring vivo) + histórico de deployments.
    local logs = {}
    local okl, resl = api:rpc({ LogsGet = { service_id = sid, tail = LOG_RING } })
    if okl and resl.Logs then logs = resl.Logs end
    svc_logs_buf = logs
    local deployments = {}
    local okd, resd = api:rpc({ DeployHistory = { service_id = sid, limit = 30 } })
    if okd and resd.Deployments then deployments = resd.Deployments end

    local status_label, status_color = fmt.status_label_color(svc.status)
    local skind, sdetail, sbuild = fmt.source_summary3(spec.source)

    -- Campos editáveis da aba General (source).
    local g = { repo_url = "", branch = "", username = "", credentials = "",
                build_path = "", watch_paths = "", submodules = false,
                dockerfile = "", context_path = "", build_stage = "" }
    local compose_content = ""
    local prov_tab, gen_provider_id, bound_url = "git", "", ""
    if spec.source.Git then
        local gs = spec.source.Git
        g.repo_url = gs.url or ""
        g.branch = gs.branch or ""
        g.username = gs.username or ""
        g.credentials = gs.credentials or ""
        g.build_path = gs.root_path or ""
        g.watch_paths = table.concat(gs.watch_paths or {}, ", ")
        g.submodules = gs.submodules == true
        g.dockerfile = gs.dockerfile_path or ""
        g.context_path = gs.build_context or ""
        g.build_stage = gs.build_stage or ""
        if gs.provider_id and gs.provider_id ~= "" then
            prov_tab, gen_provider_id, bound_url = "gitea", gs.provider_id, gs.url or ""
        end
    elseif spec.source.Registry then
        g.repo_url = spec.source.Registry.image or ""
    elseif spec.source.Compose then
        compose_content = spec.source.Compose.content or ""
    end

    -- Healthcheck → campos do form.
    local hc = spec.healthcheck
    local hk, hpath, hstatus = "tcp", "", ""
    if type(hc.kind) == "table" and hc.kind.Http then
        hk, hpath, hstatus = "http", hc.kind.Http.path or "", tostring(hc.kind.Http.expected_status)
    elseif hc.kind == "None" then hk = "none"
    elseif hc.kind == "DockerNative" then hk = "docker"
    else hk = "tcp" end

    -- Rota primária (Connection) + provedores Gitea conectados.
    local routes = {}
    if spec.domains and #spec.domains > 0 then
        for _, r in ipairs(spec.domains) do table.insert(routes, r) end
    elseif spec.domain and spec.domain ~= "" then
        routes = { { domain = spec.domain, tls = spec.tls_enabled == true } }
    end
    local pr = routes[1]
    local pr_domain = pr and pr.domain or nil
    local pr_tls = pr and pr.tls == true or false

    ctx.svc_loading = "false"
    ctx.svc_error = ""
    ctx.svc_id = svc.id
    ctx.svc_name = spec.name
    ctx.svc_project = project_name
    ctx.svc_project_id = spec.project_id
    ctx.svc_status_label = status_label
    ctx.svc_status_color = status_color
    ctx.svc_source_kind = skind
    ctx.svc_source_detail = sdetail
    ctx.svc_compose = compose_content
    ctx.svc_compose_orig = compose_content
    ctx.svc_build = sbuild
    ctx.svc_port = tostring(spec.port)
    ctx.svc_host_port = spec.host_port and tostring(spec.host_port) or "—"
    ctx.svc_domain = pr_domain or "—"
    ctx.svc_tls = pr_tls and "enabled" or "disabled"
    ctx.svc_replicas = tostring(spec.replicas)
    ctx.svc_domains = fmt.domains_json(spec)
    ctx.svc_domains_count = tostring(#routes)
    ctx.svc_external_url = fmt.external_url(pr_domain, pr_tls)
    ctx.svc_internal_url = fmt.internal_url(spec.db_kind, fmt.safe_name(spec.name), spec.port)
    ctx.svc_db_kind = spec.db_kind or "—"
    ctx.svc_hc = fmt.healthcheck_summary(spec.healthcheck)
    ctx.svc_run_command = spec.run_command or "—"
    ctx.svc_run_args = (spec.run_args and #spec.run_args > 0) and table.concat(spec.run_args, " ") or "—"
    ctx.svc_env = fmt.env_json_with_comments(spec.env_vars, spec.env_comments)
    ctx.svc_env_count = tostring(#(spec.env_vars or {}))
    local dotenv = fmt.env_dotenv_with_comments(spec.env_vars, spec.env_comments)
    ctx.svc_env_text = dotenv
    ctx.svc_env_text_orig = dotenv
    ctx.svc_logs = fmt.logs_json(logs)
    ctx.svc_logs_count = tostring(#logs)
    ctx.svc_logs_text = fmt.join_log_lines(logs)
    ctx.svc_deployments = fmt.deployments_detail(deployments)
    ctx.svc_deployments_count = tostring(#deployments)
    -- Campos editáveis.
    ctx.f_domain = ""
    ctx.f_port = ""
    ctx.f_tls = "false"
    ctx.f_host_port = spec.host_port and tostring(spec.host_port) or ""
    ctx.f_hc_kind = hk
    ctx.f_hc_path = hpath
    ctx.f_hc_status = hstatus
    ctx.f_hc_interval = tostring(hc.interval_secs)
    ctx.f_hc_timeout = tostring(hc.timeout_secs)
    ctx.f_hc_retries = tostring(hc.retries)
    ctx.f_hc_start = tostring(hc.start_period_secs)
    ctx.f_replicas = tostring(spec.replicas)
    ctx.f_run_command = spec.run_command or ""
    ctx.f_repo_url = g.repo_url
    ctx.f_branch = g.branch
    ctx.f_username = g.username
    ctx.f_credentials = g.credentials
    ctx.f_build_path = g.build_path
    ctx.f_watch_paths = g.watch_paths
    ctx.f_submodules = g.submodules and "true" or "false"
    ctx.f_dockerfile = g.dockerfile
    ctx.f_context_path = g.context_path
    ctx.f_build_stage = g.build_stage
    ctx.f_gen_port = tostring(spec.port)
    ctx.prov_tab = prov_tab
    ctx.gitea_provider_id = gen_provider_id

    -- Provedores Gitea conectados (para o picker de conta).
    local okp, resp = api:rpc("GitProviderList")
    if okp and resp.GitProviders then
        ctx.gitea_providers = fmt.git_providers(resp.GitProviders)
        ctx.gitea_count = tostring(#resp.GitProviders)
        -- Serviço já ligado a um provedor: pré-carrega repos/branches.
        if gen_provider_id ~= "" then
            local okr, resr = api:rpc({ GitRepoList = { provider_id = gen_provider_id } })
            if okr and resr.GitRepos then
                ctx.gitea_repos = fmt.git_repos(resr.GitRepos)
                ctx.gitea_msg = tostring(#resr.GitRepos) .. " repositório(s)"
                local full
                for _, r in ipairs(resr.GitRepos) do
                    if r.clone_url == bound_url then full = r.full_name break end
                end
                if full then
                    ctx.gitea_repo = full
                    local okb, resb = api:rpc({ GitBranchList = { provider_id = gen_provider_id, repo_full_name = full } })
                    if okb and resb.GitBranches then
                        ctx.gitea_branches = fmt.git_branches(resb.GitBranches)
                    end
                end
            end
        end
    end
end

-- ── Service detail: mutações de spec/env (ServiceGet → muta → ServiceUpdate) ─

-- Busca o spec fresco, aplica `mutate(spec)` (retorna erro-string ou nil),
-- envia ServiceUpdate e refaz o fetch do detalhe.
local function with_spec(mutate)
    local sid = ctx.selected_service
    if not api or not sid or sid == "" then return end
    local ok, res = api:rpc({ ServiceGet = { id = sid } })
    if not ok or not res.Service then
        ctx.svc_action_msg = "erro ao carregar spec"
        notify_err("erro ao carregar o serviço")
        return
    end
    local spec = res.Service.spec
    local emsg = mutate(spec)
    if emsg then ctx.svc_action_msg = "erro: " .. emsg; notify_err(emsg); return end
    local ok2, res2 = api:rpc_checked({ ServiceUpdate = { id = sid, spec = spec } })
    if ok2 then
        ctx.svc_action_msg = "salvo"
        fetch_service_detail(sid)
    else
        ctx.svc_action_msg = "erro: " .. res2
        notify_err(res2)
    end
end

-- Move o domínio legado (domain/tls_enabled) para a lista `domains`.
local function materialize_domains(spec)
    if not spec.domains or #spec.domains == 0 then
        local routes = json.array({})
        if spec.domain and spec.domain ~= "" then
            table.insert(routes, { domain = spec.domain, tls = spec.tls_enabled == true })
        end
        spec.domains = routes
    end
    spec.domain = nil
    spec.tls_enabled = false
end

function dom_add()
    local domain = trim(ctx.f_domain)
    if domain == "" then ctx.erro_f_domain = "domínio vazio"; return end
    ctx.erro_f_domain = ""
    with_spec(function(spec)
        materialize_domains(spec)
        local route = { domain = domain, port = tonumber(trim(ctx.f_port)), tls = ctx.f_tls == "true" }
        local found = false
        for i, r in ipairs(spec.domains) do
            if r.domain == domain then spec.domains[i] = route; found = true; break end
        end
        if not found then table.insert(spec.domains, route) end
    end)
end

function dom_del(domain)
    with_spec(function(spec)
        materialize_domains(spec)
        local kept = json.array({})
        for _, r in ipairs(spec.domains) do
            if r.domain ~= domain then table.insert(kept, r) end
        end
        spec.domains = kept
    end)
end

function dom_hostport_save()
    with_spec(function(spec)
        spec.host_port = tonumber(trim(ctx.f_host_port))
    end)
end

function compose_save()
    with_spec(function(spec)
        if spec.source.Compose then
            spec.source = { Compose = { content = ctx.svc_compose or "" } }
        end
    end)
end

function compose_cancel()
    ctx.svc_compose = ctx.svc_compose_orig or ""
end

function hc_save()
    with_spec(function(spec)
        local kind = ctx.f_hc_kind
        local hk
        if kind == "none" then
            hk = "None"
        elseif kind == "http" then
            local path = trim(ctx.f_hc_path); if path == "" then path = "/" end
            hk = {
                Http = {
                    path = path,
                    expected_status = tonumber(trim(ctx.f_hc_status)) or 200
                }
            }
        elseif kind == "docker" then
            hk = "DockerNative"
        else
            hk = "Tcp"
        end
        local cur = spec.healthcheck
        spec.healthcheck = {
            kind = hk,
            interval_secs = tonumber(trim(ctx.f_hc_interval)) or cur.interval_secs,
            timeout_secs = tonumber(trim(ctx.f_hc_timeout)) or cur.timeout_secs,
            retries = tonumber(trim(ctx.f_hc_retries)) or cur.retries,
            start_period_secs = tonumber(trim(ctx.f_hc_start)) or cur.start_period_secs,
        }
    end)
end

function adv_save()
    with_spec(function(spec)
        local r = tonumber(trim(ctx.f_replicas)) or 1
        if r < 1 then r = 1 end
        spec.replicas = math.floor(r)
        local rc = trim(ctx.f_run_command)
        spec.run_command = (rc ~= "") and rc or nil
    end)
end

function gen_save()
    with_spec(function(spec)
        local port = tonumber(trim(ctx.f_gen_port))
        if port then spec.port = math.floor(port) end
        local function ne(v, d) v = trim(v); if v == "" then return d else return v end end
        local function opt(v) v = trim(v); if v == "" then return nil else return v end end
        local repo = trim(ctx.f_repo_url)
        local watch = json.array({})
        for part in string.gmatch(ctx.f_watch_paths or "", "[^,]+") do
            local t = trim(part); if t ~= "" then table.insert(watch, t) end
        end
        local git = { Git = {
            url = repo, branch = ne(ctx.f_branch, "main"), root_path = ne(ctx.f_build_path, "."),
            watch_paths = watch, submodules = ctx.f_submodules == "true",
            dockerfile_path = ne(ctx.f_dockerfile, "Dockerfile"), build_context = ne(ctx.f_context_path, "."),
            build_stage = opt(ctx.f_build_stage), credentials = opt(ctx.f_credentials),
            username = opt(ctx.f_username), provider_id = opt(ctx.gitea_provider_id),
        } }
        if spec.source.Compose then
            -- inalterado
        elseif spec.source.Git or looks_like_git_url(repo) then
            spec.source = git
        else
            spec.source = { Registry = { image = repo } }
        end
    end)
end

-- ── Environment ────────────────────────────────────────────────────────────

function env_add()
    local key = trim(ctx.env_new_key)
    if key == "" then ctx.erro_env_new_key = "chave vazia"; return end
    ctx.erro_env_new_key = ""
    local val = ctx.env_new_val or ""
    with_spec(function(spec)
        local kept = json.array({})
        for _, v in ipairs(spec.env_vars or {}) do
            if v.key ~= key then table.insert(kept, v) end
        end
        table.insert(kept, { key = key, value = { Plain = val } })
        spec.env_vars = kept
    end)
    ctx.env_new_key = ""
    ctx.env_new_val = ""
end

function env_del(key)
    with_spec(function(spec)
        local kept = json.array({})
        for _, v in ipairs(spec.env_vars or {}) do
            if v.key ~= key then table.insert(kept, v) end
        end
        spec.env_vars = kept
        local kc = json.array({})
        for _, c in ipairs(spec.env_comments or {}) do
            if c.before_key ~= key then table.insert(kc, c) end
        end
        spec.env_comments = kc
    end)
end

function env_import()
    local text = ctx.svc_env_text or ""
    with_spec(function(spec)
        local vars, comments = parse_dotenv(text)
        spec.env_vars = vars
        spec.env_comments = comments
    end)
    ctx.env_text_open = "false"
end

function env_export()
    -- Abre o editor .env já preenchido (o texto vem do fetch em svc_env_text).
    ctx.env_text_open = "true"
end

function env_text_toggle()
    ctx.env_text_open = (ctx.env_text_open == "true") and "false" or "true"
end

function env_text_cancel()
    ctx.svc_env_text = ctx.svc_env_text_orig or ""
    ctx.env_text_open = "false"
end

-- Reordena env (arraste): `value` é um JSON array de keys na nova ordem.
function env_reorder(v)
    if not v then return end
    local ok, keys = pcall(json.decode, v)
    if not ok or type(keys) ~= "table" then return end
    with_spec(function(spec)
        local nv, nc = reorder_env(spec.env_vars or {}, spec.env_comments or {}, keys)
        spec.env_vars = nv
        spec.env_comments = nc
    end)
end

-- ── Deployments (build log + delete) ───────────────────────────────────────

function dep_logs(id)
    if not api or not id then return end
    ctx.dep_selected = id
    local ok, res = api:rpc({ GetBuildLogs = { deployment_id = id } })
    local lines = (ok and res.BuildLogs) or {}
    blog_buf = lines
    ctx.dep_build_logs = fmt.build_logs_json(lines)
    ctx.dep_build_count = tostring(#lines)
    ctx.dep_build_text = fmt.join_log_lines(lines)
end

function delete_deployment(id)
    confirm({
        title = "Remover deployment?",
        message = "O registro do deployment e seus build logs serão removidos. Essa ação não pode ser desfeita.",
        confirm_label = "Remover",
        confirm_action = "do_delete_deployment:" .. tostring(id),
        destructive = true,
    })
end

function do_delete_deployment(id)
    if not api or not id then return end
    local ok, res = api:rpc_checked({ DeployDelete = { deployment_id = id } })
    ctx.svc_action_msg = ok and "deployment removido" or ("erro: " .. res)
    if not ok then notify_err(res) end
    fetch_service_detail(ctx.selected_service)
end

-- ── Ações de ciclo de vida do serviço ──────────────────────────────────────

local function svc_lifecycle(cmd, msg)
    local sid = ctx.selected_service
    if not api or not sid or sid == "" then return end
    ctx.svc_action_msg = msg
    local ok, res = api:rpc_checked(cmd)
    if not ok then ctx.svc_action_msg = "erro: " .. res end
    fetch_service_detail(sid)
end

function svc_deploy()
    start_deploy(ctx.selected_service)
end

function svc_rebuild()
    start_deploy(ctx.selected_service)
end

function svc_reload()
    svc_lifecycle({ ServiceReload = { service_id = ctx.selected_service } }, "recarregando…")
end

function svc_stop()
    confirm({
        title = "Parar serviço?",
        message = "O tráfego para este serviço será interrompido até um novo deploy.",
        confirm_label = "Parar",
        confirm_action = "do_svc_stop",
        destructive = true,
    })
end

function do_svc_stop()
    svc_lifecycle({ ServiceStop = { service_id = ctx.selected_service } }, "parando…")
end

-- Ações de serviço a partir de um card do grid do projeto (project_services).
-- O grid se atualiza no próximo snapshot (2s), então só setamos proj_action_msg.
function svc_stop_id(id)
    confirm({
        title = "Parar serviço?",
        message = "O tráfego para este serviço será interrompido até um novo deploy.",
        confirm_label = "Parar",
        confirm_action = "do_svc_stop_id:" .. tostring(id),
        destructive = true,
    })
end

function do_svc_stop_id(id)
    if not api or not id then return end
    ctx.proj_action_msg = "enviando…"
    local ok, res = api:rpc_checked({ ServiceStop = { service_id = id } })
    ctx.proj_action_msg = ok and "serviço parado" or ("erro: " .. res)
    if not ok then notify_err(res) end
    refresh_now()
end

function stop_delete_service(id)
    confirm({
        title = "Parar e remover serviço?",
        message = "O serviço será parado e removido. Essa ação não pode ser desfeita.",
        confirm_label = "Remover",
        confirm_action = "do_stop_delete_service:" .. tostring(id),
        destructive = true,
    })
end

function do_stop_delete_service(id)
    if not api or not id then return end
    ctx.proj_action_msg = "removendo…"
    local ok, res = api:rpc_checked({ ServiceStop = { service_id = id } })
    if not ok then ctx.proj_action_msg = "erro ao parar: " .. res; notify_err(res); return end
    local ok2, res2 = api:rpc_checked({ ServiceDelete = { id = id } })
    ctx.proj_action_msg = ok2 and "serviço parado e removido" or ("erro ao remover: " .. res2)
    if ok2 then notify_ok("Serviço removido") else notify_err(res2) end
    refresh_now()
end

-- ── Picker Gitea (conta → repos → branches) ────────────────────────────────

function gitea_provider_pick(id: string)
    ctx.gitea_provider_id = id or ""
    ctx.gitea_repo = ""
    ctx.gitea_repos = "[]"
    ctx.gitea_branches = "[]"
    if not api or not id or id == "" then return end
    ctx.gitea_msg = "carregando repositórios…"
    local ok, res = api:rpc({ GitRepoList = { provider_id = id } })
    if ok and res.GitRepos then
        ctx.gitea_repos = fmt.git_repos(res.GitRepos)
        ctx.gitea_msg = tostring(#res.GitRepos) .. " repositório(s)"
    else
        ctx.gitea_msg = "erro ao listar repositórios"
    end
end

function gitea_repo_pick(full_name: string)
    if not full_name then return end
    ctx.gitea_repo = full_name
    -- Resolve clone_url/branch a partir da lista já carregada.
    local ok, repos = pcall(json.decode, ctx.gitea_repos or "[]")
    if ok and type(repos) == "table" then
        for _, r in ipairs(repos) do
            if r.full_name == full_name then
                if r.clone_url and r.clone_url ~= "" then ctx.f_repo_url = r.clone_url end
                if r.default_branch and r.default_branch ~= "" then ctx.f_branch = r.default_branch end
                break
            end
        end
    end
    ctx.gitea_branches = "[]"
    local pid = ctx.gitea_provider_id or ""
    if not api or pid == "" then return end
    ctx.gitea_msg = "carregando branches…"
    local okb, resb = api:rpc({ GitBranchList = { provider_id = pid, repo_full_name = full_name } })
    if okb and resb.GitBranches then
        ctx.gitea_branches = fmt.git_branches(resb.GitBranches)
    end
end

-- ── Wizard "Novo serviço" ──────────────────────────────────────────────────
-- A construção do ServiceSpec (incl. compose de banco/broker e blueprints de
-- template) vive no daemon: o cliente só coleta os campos e delega via
-- Command::WizardCatalog (catálogos) + WizardCreate (montagem+criação).

local NS_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
math.randomseed(os.time())
local function token_urlsafe(n)
    local t = {}
    for i = 1, n do
        local idx = math.random(1, #NS_ALPHABET)
        t[i] = string.sub(NS_ALPHABET, idx, idx)
    end
    return table.concat(t)
end

local function ns_is_broker(id)
    return id == "kafka" or id == "rabbitmq" or id == "nats"
end

function new_service_open()
    if (ctx.selected_project_id or "") == "" then return end
    ctx.ns_name = ""
    ctx.ns_app_name = ""
    ctx.ns_msg = ""
    ctx.erro_ns_name = ""
    ctx.ns_tsearch = ""
    ctx.ns_step = "pick_type"
    ctx.view = "new_service"
    if not api then return end
    local ok, res = api:rpc({ WizardCatalog = { search = "" } })
    if ok and res.WizardCatalog then
        ctx.ns_dbs = res.WizardCatalog.dbs
        ctx.ns_brokers = res.WizardCatalog.brokers
        ctx.ns_templates = res.WizardCatalog.templates
        local okt, all = pcall(json.decode, res.WizardCatalog.templates)
        ns_templates_all = (okt and type(all) == "table") and all or {}
    end
end

function ns_cancel()
    ctx.view = "project_services"
    ctx.ns_step = ""
    ctx.ns_msg = ""
end

function ns_back()
    local step = ctx.ns_step
    if step == "pick_db" or step == "pick_broker" or step == "app_form"
        or step == "compose_form" or step == "pick_template" or step == "db_form"
        or step == "template_form" then
        ctx.ns_step = "pick_type"
    else
        ctx.view = "project_services"
        ctx.ns_step = ""
    end
    ctx.ns_msg = ""
end

function ns_kind(k)
    if k == "application" then ctx.ns_step = "app_form"
    elseif k == "compose" then ctx.ns_step = "compose_form"
    elseif k == "database" then ctx.ns_step = "pick_db"
    elseif k == "broker" then ctx.ns_step = "pick_broker"
    elseif k == "template" then ctx.ns_step = "pick_template"
    end
end

-- Banco escolhido: pré-preenche o formulário (senhas geradas no cliente).
function ns_db(id)
    local ok, list = pcall(json.decode, ctx.ns_dbs or "[]")
    if not ok or type(list) ~= "table" then return end
    for _, d in ipairs(list) do
        if d.id == id then
            ctx.ns_db_kind = id
            ctx.ns_db_label = d.label
            ctx.ns_image = d.image
            ctx.ns_db_name = d.has_db_name and "app" or ""
            ctx.ns_db_user = d.user or ""
            ctx.ns_db_password = token_urlsafe(22)
            ctx.ns_db_root_password = token_urlsafe(22)
            ctx.ns_use_replica = "false"
            ctx.ns_db_has_dbname = d.has_db_name and "true" or "false"
            ctx.ns_db_has_user = d.has_user and "true" or "false"
            ctx.ns_db_has_rootpw = d.has_rootpw and "true" or "false"
            ctx.ns_db_has_replica = d.has_replica and "true" or "false"
            ctx.ns_name = ""
            ctx.ns_step = "db_form"
            return
        end
    end
end

-- Broker escolhido: compartilha o passo db_form (mesmas chaves ns_db_*).
function ns_broker(id)
    local ok, list = pcall(json.decode, ctx.ns_brokers or "[]")
    if not ok or type(list) ~= "table" then return end
    for _, b in ipairs(list) do
        if b.id == id then
            ctx.ns_db_kind = id
            ctx.ns_db_label = b.label
            ctx.ns_image = b.image
            ctx.ns_db_name = ""
            ctx.ns_db_user = b.user or ""
            ctx.ns_db_password = token_urlsafe(22)
            ctx.ns_db_root_password = ""
            ctx.ns_use_replica = "false"
            ctx.ns_db_has_dbname = "false"
            ctx.ns_db_has_user = b.has_user and "true" or "false"
            ctx.ns_db_has_rootpw = "false"
            ctx.ns_db_has_replica = "false"
            ctx.ns_name = ""
            ctx.ns_step = "db_form"
            return
        end
    end
end

-- Template escolhido: carrega nome/slug + variáveis editáveis (do catálogo).
function ns_pick_template(id)
    for _, t in ipairs(ns_templates_all) do
        if t.id == id then
            ctx.ns_template_id = id
            ctx.ns_template_name = t.name
            ctx.ns_name = t.slug or ""
            ctx.ns_template_vars = (t.vars and #t.vars > 0) and json.encode(t.vars) or "[]"
            for _, vv in ipairs(t.vars or {}) do
                ctx["ns_tv_" .. vv.idx] = ""
            end
            ctx.ns_step = "template_form"
            return
        end
    end
end

-- Busca do catálogo de templates (filtra o cache no cliente).
function ns_tsearch(v: string)
    ctx.ns_tsearch = v or ""
    local term = string.lower(v or "")
    local rows = {}
    for _, t in ipairs(ns_templates_all) do
        if term == ""
            or string.find(string.lower(t.name or ""), term, 1, true)
            or string.find(string.lower(t.description or ""), term, 1, true) then
            table.insert(rows, t)
        end
    end
    ctx.ns_templates = (#rows > 0) and json.encode(rows) or "[]"
end

function ns_create()
    local pid = ctx.selected_project_id or ""
    if pid == "" or not api then return end
    local step = ctx.ns_step
    local kind, id
    if step == "app_form" then kind, id = "application", ""
    elseif step == "compose_form" then kind, id = "compose", ""
    elseif step == "db_form" then
        id = ctx.ns_db_kind or ""
        kind = ns_is_broker(id) and "broker" or "database"
    elseif step == "template_form" then kind, id = "template", ctx.ns_template_id or ""
    else ctx.ns_msg = "passo inválido"; return end

    -- Nome obrigatório, exceto no template (tem slug default).
    local name = trim(ctx.ns_name)
    if step ~= "template_form" and trim(ctx.ns_app_name) == "" and name == "" then
        ctx.erro_ns_name = "informe um nome"; return
    end
    ctx.erro_ns_name = ""

    -- Valores das variáveis do template, na ordem declarada.
    local tvals = json.array({})
    if step == "template_form" then
        local okv, vars = pcall(json.decode, ctx.ns_template_vars or "[]")
        if okv and type(vars) == "table" then
            for _, vv in ipairs(vars) do
                table.insert(tvals, ctx["ns_tv_" .. vv.idx] or "")
            end
        end
    end

    local req = {
        kind = kind, id = id, project_id = pid,
        name = name, app_name = trim(ctx.ns_app_name),
        db_name = trim(ctx.ns_db_name), user = trim(ctx.ns_db_user),
        password = trim(ctx.ns_db_password), root_password = trim(ctx.ns_db_root_password),
        image = trim(ctx.ns_image), use_replica = (ctx.ns_use_replica == "true"),
        template_values = tvals,
    }
    ctx.ns_msg = "criando…"
    local ok, res = api:rpc_checked({ WizardCreate = req })
    if ok and res.Service then
        ctx.view = "project_services"
        ctx.proj_tab = "services"
        ctx.ns_step = ""
        ctx.ns_msg = ""
        notify_ok("Serviço criado")
        refresh_now()
    else
        local m = ok and "resposta inesperada" or ("erro: " .. res)
        ctx.ns_msg = m
        notify_err(ok and "resposta inesperada do daemon" or res)
    end
end

-- ── Editar projeto (cabeçalho de project_services) ─────────────────────────

function edit_project_toggle()
    ctx.edit_proj_name = ctx.proj_name or ""
    ctx.edit_proj_desc = ctx.proj_description or ""
    ctx.erro_edit_proj_name = ""
    ctx.editing_project = "true"
end

function cancel_project_edit() ctx.editing_project = "false" end

function save_project_edit()
    local pid = ctx.selected_project_id or ""
    if pid == "" or not api then return end
    local name = trim(ctx.edit_proj_name)
    if name == "" then ctx.erro_edit_proj_name = "informe um nome"; return end
    ctx.erro_edit_proj_name = ""
    local desc = trim(ctx.edit_proj_desc)
    local ok, res = api:rpc_checked({ ProjectUpdate = { id = pid, name = name,
        description = (desc ~= "" and desc or nil) } })
    if ok then
        ctx.editing_project = "false"
        ctx.proj_action_msg = "projeto atualizado"
        ctx.proj_name = name
        ctx.proj_description = desc
    else
        ctx.proj_action_msg = "erro: " .. res
    end
end

-- ── Variáveis de ambiente do projeto (aba env de project_services) ─────────

-- Busca o projeto fresco, aplica `mutate(env_vars)` (devolve o novo array),
-- envia ProjectEnvSet e reescreve proj_env.
local function with_project_env(mutate)
    local pid = ctx.selected_project_id or ""
    if pid == "" or not api then return end
    local ok, res = api:rpc("ProjectList")
    if not ok or not res.Projects then return end
    local env
    for _, p in ipairs(res.Projects) do
        if p.id == pid then env = p.env_vars or json.array({}); break end
    end
    if env == nil then return end
    local newenv = mutate(env)
    local ok2, res2 = api:rpc_checked({ ProjectEnvSet = { project_id = pid, env_vars = newenv } })
    if ok2 then
        ctx.proj_env = fmt.env_json_with_comments(newenv, {})
        ctx.proj_env_count = tostring(#newenv)
    else
        notify_err(res2)
    end
end

function penv_add()
    local key = trim(ctx.penv_new_key)
    if key == "" then ctx.erro_penv_new_key = "chave vazia"; return end
    ctx.erro_penv_new_key = ""
    local val = ctx.penv_new_val or ""
    with_project_env(function(env)
        local kept = json.array({})
        for _, v in ipairs(env) do if v.key ~= key then table.insert(kept, v) end end
        table.insert(kept, { key = key, value = { Plain = val } })
        return kept
    end)
    ctx.penv_new_key = ""
    ctx.penv_new_val = ""
end

function penv_del(key)
    with_project_env(function(env)
        local kept = json.array({})
        for _, v in ipairs(env) do if v.key ~= key then table.insert(kept, v) end end
        return kept
    end)
end

function penv_reorder(v)
    if not v then return end
    local ok, keys = pcall(json.decode, v)
    if not ok or type(keys) ~= "table" then return end
    with_project_env(function(env)
        local by = {}
        for _, x in ipairs(env) do by[x.key] = x end
        local out = json.array({})
        for _, k in ipairs(keys) do
            if by[k] then table.insert(out, by[k]); by[k] = nil end
        end
        for _, x in ipairs(env) do if by[x.key] then table.insert(out, x) end end
        return out
    end)
end

-- ── Settings (Web Server) ──────────────────────────────────────────────────

function ss_domain(v)
    ctx.ss_domain = v or ""
    ctx.gp_redirect = oauth_redirect_uri(v or "")
end

function settings_tab(t) ctx.settings_tab = t end

function settings_save()
    if not api then return end
    local domain = trim(ctx.ss_domain)
    local email = trim(ctx.ss_email)
    ctx.settings_msg = "salvando…"
    local ok, res = api:rpc_checked({ SetDaemonSettings = {
        webhook_base_url = (domain ~= "" and domain or nil),
        acme_email = (email ~= "" and email or nil),
    } })
    ctx.settings_msg = ok and "configurações salvas" or ("erro: " .. res)
    if ok then notify_ok("Configurações salvas") else notify_err(res) end
end

-- ── Settings → Git (provedores Gitea) ──────────────────────────────────────

function gp_mode(m) ctx.gp_mode = m end

local function gp_refresh_list()
    local ok, res = api:rpc("GitProviderList")
    if ok and res.GitProviders then
        ctx.gitea_providers = fmt.git_providers(res.GitProviders)
        ctx.gitea_count = tostring(#res.GitProviders)
    end
end

function gp_refresh()
    if not api then return end
    gp_refresh_list()
end

function gp_connect()
    if not api then return end
    ctx.gp_oauth_url = ""
    local base = trim(ctx.gp_base_url)
    if base == "" then ctx.gp_msg = "informe a Base URL do Gitea"; return end
    local name = trim(ctx.gp_name); if name == "" then name = "Gitea" end
    local is_oauth = (ctx.gp_mode or "oauth") ~= "pat"
    local cmd
    if is_oauth then
        local cid = trim(ctx.gp_client_id)
        local csec = ctx.gp_client_secret or ""
        if cid == "" or trim(csec) == "" then
            ctx.gp_msg = "Client ID e Client Secret são obrigatórios"; return
        end
        cmd = { GitProviderCreate = { kind = "Gitea", name = name, base_url = base,
            auth_mode = "OAuth", oauth_client_id = cid, oauth_client_secret = csec, pat = nil } }
    else
        local pat = ctx.gp_pat or ""
        if trim(pat) == "" then ctx.gp_msg = "informe o Personal Access Token"; return end
        cmd = { GitProviderCreate = { kind = "Gitea", name = name, base_url = base,
            auth_mode = "Pat", oauth_client_id = nil, oauth_client_secret = nil, pat = pat } }
    end
    ctx.gp_msg = "conectando…"
    local ok, res = api:rpc(cmd)
    if not ok or not res.GitProviderInfo then
        ctx.gp_msg = "erro: " .. (ok and "resposta inesperada" or res); return
    end
    local pid = res.GitProviderInfo.id
    if is_oauth then
        -- OAuth precisa de round-trip no navegador. O Luau não abre o browser;
        -- surface a URL para o usuário abrir e depois clicar em "Atualizar lista".
        local oko, reso = api:rpc({ GitOAuthStart = { provider_id = pid } })
        if oko and reso.OAuthUrl then
            ctx.gp_oauth_url = reso.OAuthUrl
            ctx.gp_msg = "abra para autorizar: " .. reso.OAuthUrl
        else
            ctx.gp_msg = "provider criado; inicie o OAuth manualmente"
        end
    else
        ctx.gp_msg = "conta Gitea conectada ✓"
    end
    ctx.gp_name = ""
    ctx.gp_base_url = ""
    ctx.gp_client_id = ""
    ctx.gp_client_secret = ""
    ctx.gp_pat = ""
    gp_refresh_list()
end

function gp_delete(id)
    if not api or not id then return end
    local ok, res = api:rpc_checked({ GitProviderDelete = { id = id } })
    ctx.gp_msg = ok and "provider removido" or ("erro: " .. res)
    if not ok then notify_err(res) end
    gp_refresh_list()
end

-- ── Ações mutantes ────────────────────────────────────────────────────────

function stop_all()
    confirm({
        title = "Parar todos os serviços?",
        message = "Todos os serviços em execução serão parados agora. Você pode reiniciá-los depois com um novo deploy.",
        confirm_label = "Parar todos",
        confirm_action = "do_stop_all",
        destructive = true,
    })
end

function do_stop_all()
    if not api then return end
    ctx.status_line = "parando todos…"
    local ok, res = api:rpc_checked("StopAllManaged")
    if ok then
        local n = res.StopAllResult and res.StopAllResult.count or 0
        ctx.status_line = "parados: " .. tostring(n)
    else
        ctx.status_line = "erro: " .. res
    end
end

function create_project()
    if not api then return end
    local name = (ctx.new_proj_name or ""):gsub("^%s+", ""):gsub("%s+$", "")
    if name == "" then
        ctx.erro_new_proj_name = "informe um nome"
        return
    end
    ctx.erro_new_proj_name = ""
    local desc = (ctx.new_proj_desc or ""):gsub("^%s+", ""):gsub("%s+$", "")
    local cmd = { ProjectCreate = { name = name, description = (desc ~= "" and desc or nil) } }
    ctx.new_proj_msg = "criando…"
    local ok, res = api:rpc_checked(cmd)
    if ok then
        ctx.new_proj_name = ""
        ctx.new_proj_desc = ""
        ctx.new_proj_msg = "projeto criado"
        notify_ok("Projeto \"" .. name .. "\" criado")
        refresh_now()
    else
        ctx.new_proj_msg = "erro: " .. res
        notify_err(res)
    end
end

function delete_project(id: string)
    if not id or id == "" then return end
    confirm({
        title = "Remover projeto?",
        message = "O projeto será removido. Só é possível remover projetos sem serviços.",
        confirm_label = "Remover projeto",
        confirm_action = "do_delete_project:" .. tostring(id),
        destructive = true,
    })
end

function do_delete_project(id)
    if not api or not id or id == "" then return end
    ctx.proj_action_msg = "removendo…"
    local ok, res = api:rpc_checked({ ProjectDelete = { id = id } })
    if ok then
        ctx.selected_project_id = ""
        ctx.view = "projects"
        notify_ok("Projeto removido")
        refresh_now()
    else
        ctx.proj_action_msg = "erro: " .. res
        notify_err(res)
    end
end

-- Docker: limpar recursos sem uso (a aba se atualiza no próximo snapshot).
local function prune(cmd, label)
    if not api then return end
    ctx.docker_msg = label
    local ok, res = api:rpc_checked(cmd)
    if ok and res.PruneResult then
        ctx.docker_msg = string.format("removidos: %d · %s liberados",
            res.PruneResult.count or 0, fmt.fmt_bytes(res.PruneResult.reclaimed_bytes))
    elseif ok then
        ctx.docker_msg = "limpeza concluída"
    else
        ctx.docker_msg = "erro: " .. res
    end
end

function docker_prune_images()
    confirm({
        title = "Limpar imagens sem uso?",
        message = "As imagens Docker sem uso serão removidas agora. Essa limpeza não pode ser desfeita.",
        confirm_label = "Limpar",
        confirm_action = "do_docker_prune_images",
        destructive = true,
    })
end

function do_docker_prune_images()
    local all = ctx.docker_prune_all_images == "true"
    prune({ PruneImages = { all = all } },
        all and "removendo TODAS as imagens sem uso…" or "removendo imagens dangling…")
end

function docker_prune_volumes()
    confirm({
        title = "Limpar volumes sem uso?",
        message = "Os volumes Docker sem uso serão removidos agora. Essa limpeza não pode ser desfeita.",
        confirm_label = "Limpar",
        confirm_action = "do_docker_prune_volumes",
        destructive = true,
    })
end

function do_docker_prune_volumes()
    local all = ctx.docker_prune_all_volumes == "true"
    prune({ PruneVolumes = { all = all } }, "removendo volumes sem uso…")
end

function docker_prune_networks()
    confirm({
        title = "Limpar redes sem uso?",
        message = "As redes Docker sem uso serão removidas agora. Essa limpeza não pode ser desfeita.",
        confirm_label = "Limpar",
        confirm_action = "do_docker_prune_networks",
        destructive = true,
    })
end

function do_docker_prune_networks()
    prune("PruneNetworks", "removendo redes sem uso…")
end

-- Deploy de um serviço (a partir de um card / detalhe). Arma deploy_track para
-- o timer decorrido (atualizado pelo snapshot de 2s, via engine.active) e o
-- desfecho (via evento bus DeployStateChanged).
function start_deploy(id)
    local sid = (id and id ~= "") and id or ctx.selected_service
    if not api or not sid or sid == "" then return end
    ctx.svc_action_msg = "iniciando deploy…"
    ctx.svc_action_color = "#58A6FF"
    local ok, res = api:rpc_checked({ DeployStart = { service_id = sid } })
    if ok and res.Deployment then
        deploy_track = { service_id = sid, deployment_id = res.Deployment.id, running = true }
        ctx.svc_deploy_running = "true"
        ctx.svc_deploy_elapsed = "0s"
        ctx.svc_action_msg = "deploy iniciado · " .. tostring(res.Deployment.state)
        notify_ok("Deploy iniciado")
    else
        ctx.svc_action_msg = ok and "resposta inesperada" or ("erro: " .. res)
        ctx.svc_action_color = "#F85149"
        notify_err(ok and "resposta inesperada do daemon" or res)
    end
    if ctx.selected_service == sid then fetch_service_detail(sid) end
end

-- ── Parse/reorder de .env (equivalente a services.rs) ─────────────────────

-- Parseia um blob .env em (vars, comments). Comentários ancoram na var seguinte
-- (before_key), ou nil no fim. `<secret:NAME>` volta a ser referência de secret.
function parse_dotenv(text)
    local vars = json.array({})
    local comments = json.array({})
    local pending = {}
    for line in string.gmatch((text or "") .. "\n", "([^\n]*)\n") do
        local l = trim(line)
        if l == "" then
            -- ignora
        elseif l:sub(1, 1) == "#" then
            table.insert(pending, l)
        else
            local k, v = l:match("^([^=]+)=(.*)$")
            if k then
                local key = trim(k)
                if key ~= "" then
                    for _, c in ipairs(pending) do
                        table.insert(comments, { text = c, before_key = key })
                    end
                    pending = {}
                    v = trim(v)
                    local secret = v:match("^<secret:(.+)>$")
                    local value = secret and { Secret = secret } or { Plain = v }
                    table.insert(vars, { key = key, value = value })
                end
            end
        end
    end
    for _, c in ipairs(pending) do
        table.insert(comments, { text = c, before_key = nil })
    end
    return vars, comments
end

-- Aplica uma ordem de keys (vars + linhas `__c<idx>`) a vars/comments: vars
-- reordenadas; cada comentário reancora na primeira var real que o segue.
function reorder_env(vars, comments, keys)
    local var_keys = {}
    local by_key = {}
    for _, v in ipairs(vars) do var_keys[v.key] = true; by_key[v.key] = v end
    local seen = {}
    local new_vars = json.array({})
    local new_comments = json.array({})
    for pos, k in ipairs(keys) do
        local ci = k:match("^__c(%d+)$")
        if ci then
            local idx = tonumber(ci) + 1  -- __c é 0-based
            local c = comments[idx]
            if c then
                seen[idx] = true
                local anchor = nil
                for j = pos + 1, #keys do
                    if var_keys[keys[j]] then anchor = keys[j]; break end
                end
                table.insert(new_comments, { text = c.text, before_key = anchor })
            end
        elseif by_key[k] then
            table.insert(new_vars, by_key[k])
            by_key[k] = nil
        end
    end
    for _, v in ipairs(vars) do
        if by_key[v.key] then table.insert(new_vars, v) end
    end
    for idx, c in ipairs(comments) do
        if not seen[idx] then table.insert(new_comments, c) end
    end
    return new_vars, new_comments
end
