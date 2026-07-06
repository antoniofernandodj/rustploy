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

-- ── Helpers ──────────────────────────────────────────────────────────────

-- Normaliza a entrada de URL do login para uma base HTTP(S) sem barra final.
-- Aceita "https://host:porta", "host:porta", "host" e os esquemas legados
-- rwp(s):// (convertidos para https). Default: https.
local function normalize_url(raw)
    local u = (raw or ""):gsub("^%s+", ""):gsub("%s+$", "")
    if u == "" then return nil end
    local scheme, rest = string.match(u, "^([%a][%w+.-]*)://(.*)$")
    if scheme then
        scheme = string.lower(scheme)
        if scheme == "rwp" or scheme == "rwps" then scheme = "https" end
        if scheme ~= "http" and scheme ~= "https" then return nil end
    else
        scheme, rest = "https", u
    end
    -- corta path/query/fragment: só queremos o authority (host[:porta]).
    rest = string.match(rest, "^([^/?#]*)") or rest
    if rest == "" then return nil end
    return scheme .. "://" .. rest
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

function toggle_remember_url(v) ctx.remember_url = v end
function toggle_remember_token(v) ctx.remember_token = v end

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
end

-- Eventos vivos do bus (métricas; logs ficam para a fatia de service detail).
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
    end
end

-- ── Busca ─────────────────────────────────────────────────────────────────

function search_changed(v)
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

-- Abrir o detalhe de um serviço. (Detalhe completo virá na próxima fatia; aqui
-- só navega e registra a seleção para os logs vivos futuros.)
function open_service(id)
    ctx.selected_service = id
    ctx.view = "service"
    ctx.tab = "general"
end

-- ── Ações mutantes ────────────────────────────────────────────────────────

function stop_all()
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
    else
        ctx.new_proj_msg = "erro: " .. res
    end
end

function delete_project(id)
    if not api or not id or id == "" then return end
    ctx.proj_action_msg = "removendo…"
    local ok, res = api:rpc_checked({ ProjectDelete = { id = id } })
    if ok then
        ctx.selected_project_id = ""
        ctx.view = "projects"
    else
        ctx.proj_action_msg = "erro: " .. res
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

function docker_prune_images() prune({ PruneImages = { all = false } }, "removendo imagens sem uso…") end
function docker_prune_volumes() prune({ PruneVolumes = { all = false } }, "removendo volumes sem uso…") end
function docker_prune_networks() prune("PruneNetworks", "removendo redes sem uso…") end

-- Deploy de um serviço (a partir de um card / detalhe).
function start_deploy(id)
    local sid = (id and id ~= "") and id or ctx.selected_service
    if not api or not sid or sid == "" then return end
    ctx.svc_action_msg = "iniciando deploy…"
    local ok, res = api:rpc_checked({ DeployStart = { service_id = sid } })
    if not ok then ctx.svc_action_msg = "erro: " .. res end
end
