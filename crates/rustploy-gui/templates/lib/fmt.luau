--!strict
-- fmt.luau — Reimplementação Luau de `src/app/net/view.rs`.
--
-- Funções puras: modelo (tabela vinda de `json.decode` do snapshot da API) →
-- string/JSON que os templates iteram. Nenhuma faz I/O. As formas de saída são
-- IDÊNTICAS às de `view.rs` para não mexer na markup das listas (mesmos nomes
-- de campo por linha). O filtro de busca (`matches_search`) vive aqui, como o
-- antigo `search_pairs`.

-- ── Definições de Tipos Customizados ──────────────────────────────────────

type EnvValue = {
    Secret: string?,
    Plain: string?,
}

type EnvVar = {
    key: string,
    value: EnvValue,
}

type EnvComment = {
    before_key: string?,
    text: string,
}

type DomainRoute = {
    domain: string,
    port: number?,
    tls: boolean,
}

type ServiceSpec = {
    name: string,
    port: number,
    source: any,
    domain: string?,
    tls_enabled: boolean?,
    domains: { { domain: string, port: number?, tls: boolean? } }?,
    project_id: string,
}

type Service = {
    id: string,
    status: string | { Error: string? },
    spec: ServiceSpec,
    live_container_id: string?,
}

type MetricEntry = {
    cpu_percent: number?,
    mem_used_bytes: number?,
    net_rx_bytes: number?,
    net_tx_bytes: number?,
}

export type ServicePair = {
    svc: Service,
    proj: string,
}

type GridCard = { filler: string, [string]: any }

export type MetricsMap = { [string]: MetricEntry }

local M = {}

-- `json` é um global injetado pelo motor glacier-ui em runtime (encode/decode/
-- array) e declarado em `.luaurc` para o LSP. NÃO redefinir localmente: um
-- fallback aqui (ex.: `shared.json`) quebra o carregamento (o `shared` não
-- existe no Luau do mlua) ou anula o encoder real. Usa-se o global direto.

-- ── Tempo ────────────────────────────────────────────────────────────────
-- chrono (Rust) formatava timestamps em horário LOCAL. Aqui parseamos o
-- RFC3339 (UTC, com "Z") e convertemos para epoch verdadeiro somando o offset
-- local, para então formatar com os.date (que usa horário local).

local TZ_OFFSET: number = 0
do
    local ok, off = pcall(function()
        local now = os.time()
        return os.difftime(now, os.time(os.date("!*t", now)))
    end)
    if ok and type(off) == "number" then
        TZ_OFFSET = off
    end
end

-- RFC3339 "2026-07-06T12:34:56(.789)?(Z|±hh:mm)?" → epoch (segundos). Os
-- componentes são tratados como UTC; +TZ_OFFSET recupera o epoch verdadeiro.
local function iso_to_epoch(iso: string?): number?
    if type(iso) ~= "string" then return nil end
    local y, mo, d, h, mi, s = string.match(iso, "(%d+)-(%d+)-(%d+)T(%d+):(%d+):(%d+)")
    if not y then return nil end
    local utc = os.time({
        year = tonumber(y) :: number, month = tonumber(mo) :: number, day = tonumber(d) :: number,
        hour = tonumber(h) :: number, min = tonumber(mi) :: number, sec = tonumber(s) :: number,
    })
    if not utc then return nil end
    return utc + TZ_OFFSET
end

-- HH:MM:SS local de um timestamp ISO (ou "" se inválido).
function M.time_hms(iso: string?): string
    local ep = iso_to_epoch(iso)
    if not ep then return "" end
    return os.date("%H:%M:%S", ep)
end

-- "dd/mm HH:MM" local.
function M.date_dm_hm(iso: string?): string
    local ep = iso_to_epoch(iso)
    if not ep then return "" end
    return os.date("%d/%m %H:%M", ep)
end

-- "dd/mm HH:MM:SS" local.
function M.date_dm_hms(iso: string?): string
    local ep = iso_to_epoch(iso)
    if not ep then return "" end
    return os.date("%d/%m %H:%M:%S", ep)
end

-- ── Números / durações ───────────────────────────────────────────────────

-- Ns ou Mm Ns.
function M.fmt_secs(secs: any): string
    local num_secs = math.floor(tonumber(secs) or 0)
    local m = math.floor(num_secs / 60)
    local s = num_secs % 60
    if m > 0 then return string.format("%dm %ds", m, s) end
    return string.format("%ds", s)
end

function M.fmt_uptime(secs: any): string
    local num_secs = math.floor(tonumber(secs) or 0)
    local d = math.floor(num_secs / 86400)
    local h = math.floor((num_secs % 86400) / 3600)
    local m = math.floor((num_secs % 3600) / 60)
    local s = num_secs % 60
    if d > 0 then return string.format("%dd %dh %dm", d, h, m) end
    if h > 0 then return string.format("%dh %dm", h, m) end
    return string.format("%dm %ds", m, s)
end

-- Tamanho de bytes legível ("—" para 0), igual a view::fmt_bytes.
function M.fmt_bytes(b: any): string
    local num_b = tonumber(b) or 0
    local KB = 1024.0
    local MB = KB * 1024.0
    local GB = MB * 1024.0
    if num_b == 0 then return "—" end
    local f = num_b + 0.0
    if f >= GB then return string.format("%.1f GB", f / GB) end
    if f >= MB then return string.format("%.0f MB", f / MB) end
    return string.format("%.0f KB", f / KB)
end

-- Duração de um deployment (finished_at ou agora) − started_at, em fmt_secs.
local function fmt_duration(dep: any): string
    local start = iso_to_epoch(dep.started_at)
    if not start then return "0s" end
    local finish = dep.finished_at and iso_to_epoch(dep.finished_at) or os.time()
    local secs = finish - start
    if secs < 0 then secs = 0 end
    return M.fmt_secs(secs)
end

-- Barra de progresso em blocos.
function M.progress_bar(percent: any, width: number?): string
    local p = tonumber(percent) or 0
    local w = width or 12
    local filled = math.floor(p * w / 100)
    if filled > w then filled = w end
    if filled < 0 then filled = 0 end
    return string.rep("█", filled) .. string.rep("░", w - filled)
end

-- ── Busca ────────────────────────────────────────────────────────────────
-- Termo vazio sempre casa (a busca é filtro, não requisito). `term` já deve
-- vir em minúsculas.
function M.matches(term: string?, fields: {any}): boolean
    if not term or term == "" then return true end
    for _, f in ipairs(fields) do
        if type(f) == "string" and string.find(string.lower(f), term, 1, true) then
            return true
        end
    end
    return false
end

-- ── Codificação de arrays ────────────────────────────────────────────────
-- CUIDADO: uma tabela Luau vazia vira `{}` (objeto) no json.encode, não `[]`.
-- Os templates iteram arrays, então forçamos "[]" quando vazio.
local function enc_array(t: {any}): string
    if #t == 0 then return "[]" end
    return json.encode(t)
end
M.enc_array = enc_array

-- ── Mapas de estado (paleta = view.rs) ───────────────────────────────────

-- ServiceStatus vem como string ("Running") ou tabela ({Error="..."}).
local function status_label_color(status: any): (string, string)
    if type(status) == "table" then
        if status.Error ~= nil then return "Error", "#F85149" end
    end
    if status == "Running" then return "Running", "#3FB950"
    elseif status == "Deploying" then return "Deploying", "#58A6FF"
    elseif status == "Degraded" then return "Degraded", "#D29922"
    elseif status == "Stopping" then return "Stopping", "#8B949E"
    elseif status == "Stopped" then return "Stopped", "#8B949E"
    end
    return "Error", "#F85149"
end

local function status_kind(status: any): string
    if type(status) == "table" and status.Error ~= nil then return "bad" end
    if status == "Running" then return "ok"
    elseif status == "Deploying" then return "info"
    elseif status == "Degraded" then return "warn"
    elseif status == "Stopping" or status == "Stopped" then return "muted"
    end
    return "bad"
end

-- DeployState vem como string ("Live", "Failed", "BuildingImage", ...).
local function state_label_color(state: string?): (string, string)
    if state == "Live" then return "LIVE", "#3FB950"
    elseif state == "Stopped" then return "STOPPED", "#8B949E"
    elseif state == "Failed" then return "FAILED", "#F85149"
    end
    return "BUILDING", "#58A6FF"
end

local function state_kind(state: string?): string
    if state == "Live" then return "ok"
    elseif state == "Stopped" then return "muted"
    elseif state == "Failed" then return "bad"
    end
    return "info"
end

local function in_use_kind(in_use: boolean): string
    if in_use then return "ok" end
    return "muted"
end

-- ServiceSource → texto de "imagem/origem" (source_summary().1).
local function source_summary(source: any): string
    if type(source) ~= "table" then return "—" end
    if source.Registry then return source.Registry.image or "—"
    elseif source.Git then
        local g = source.Git
        return (g.url or "") .. " @ " .. (g.branch or "")
    elseif source.Compose then return "docker-compose"
    end
    return "—"
end

-- domain_routes(): a lista `domains` nova, ou o legado `domain`/`tls_enabled`.
local function domain_routes(spec: ServiceSpec): {DomainRoute}
    local routes: {DomainRoute} = {}
    if spec.domains and #spec.domains > 0 then
        for _, r in ipairs(spec.domains) do
            table.insert(routes, { domain = r.domain, port = r.port, tls = r.tls == true })
        end
    elseif spec.domain and spec.domain ~= "" then
        table.insert(routes, { domain = spec.domain :: string, port = nil, tls = spec.tls_enabled == true })
    end
    return routes
end

-- ── Builders de lista (mesma forma JSON de view.rs) ──────────────────────

local GRID_COLS = 3

-- `services` do snapshot é uma lista de { project_name, service }.
-- Normaliza para pares { svc = Service, proj = "nome" } e mescla métricas.
local function pair_list(services: {any}?): {ServicePair}
    local out: {ServicePair} = {}
    for _, e in ipairs(services or {}) do
        table.insert(out, { svc = e.service, proj = e.project_name or "" })
    end
    return out
end
M.pair_list = pair_list

-- Deployments (aba Deployments). `list` = Vec<DeploymentSummary>.
function M.deployments(list: {any}?, term: string?): string
    local rows = {}
    for _, s in ipairs(list or {}) do
        if M.matches(term, { s.service_name, s.project_name }) then
            local d = s.deployment
            local label, _ = state_label_color(d.state)
            table.insert(rows, {
                service = s.service_name,
                project = s.project_name,
                state_label = label,
                state_kind = state_kind(d.state),
                duration = fmt_duration(d),
                start = M.time_hms(d.started_at),
            })
        end
    end
    return enc_array(rows)
end

-- Card de serviço (com CPU/mem mesclados de `metrics_by_id[svc.id]`).
local function service_card(svc: Service, proj: string, metrics: MetricsMap?)
    local status_label, status_color = status_label_color(svc.status)
    local cpu, mem = "—", "—"
    local m = metrics and metrics[svc.id]
    if m then
        cpu = string.format("%.1f%%", m.cpu_percent or 0)
        mem = M.fmt_bytes(m.mem_used_bytes)
    end
    return {
        filler = "0",
        id = svc.id,
        name = svc.spec.name,
        project = proj,
        port = tostring(svc.spec.port),
        status_label = status_label,
        status_color = status_color,
        cpu = cpu,
        mem = mem,
    }
end

-- Array plano de cards de serviço (para seleção/contagem).
function M.services(pairs_: {ServicePair}, metrics: MetricsMap?, term: string?): string
    local rows = {}
    for _, p in ipairs(pairs_) do
        if M.matches(term, { p.svc.spec.name, p.proj }) then
            table.insert(rows, service_card(p.svc, p.proj, metrics))
        end
    end
    return enc_array(rows)
end




local FILLER: GridCard = { 
    filler = "1",
    id = nil,
    name = nil,
    project = nil,
    port = nil,
    status_label = nil,
    status_color = nil,
    cpu = nil,
    mem = nil
}


-- Cards de serviço em linhas de GRID_COLS, com filler (glacier não tem grid).

function M.service_rows(pairs_: {ServicePair}, metrics: MetricsMap?, term: string?): string
    local filtered: {GridCard} = {}
    for _, p in ipairs(pairs_) do
        if M.matches(term, { p.svc.spec.name, p.proj }) then
            table.insert(filtered, service_card(p.svc, p.proj, metrics))
        end
    end
    local rows = {}
    local i = 1
    while i <= #filtered do
        local cards = {}
        for j = i, math.min(i + GRID_COLS - 1, #filtered) do
            table.insert(cards, filtered[j])
        end
        while #cards < GRID_COLS do table.insert(cards, FILLER) end
        table.insert(rows, { cards = cards })
        i = i + GRID_COLS
    end
    return enc_array(rows)
end

-- Card de projeto, agregando contagem de serviços de `pairs_`.
local function project_card(p: any, pairs_: {ServicePair})
    local count, running = 0, 0
    for _, e in ipairs(pairs_) do
        if e.svc.spec.project_id == p.id then
            count = count + 1
            if e.svc.status == "Running" then running = running + 1 end
        end
    end
    return {
        filler = "0",
        id = p.id,
        name = p.name,
        description = p.description or "",
        service_count = tostring(count),
        running_count = tostring(running),
        can_delete = (count == 0) and "1" or "0",
    }
end

function M.projects(list: {any}?, pairs_: {ServicePair}, term: string?): string
    local rows = {}
    for _, p in ipairs(list or {}) do
        if M.matches(term, { p.name, p.description or "" }) then
            table.insert(rows, project_card(p, pairs_))
        end
    end
    return enc_array(rows)
end

function M.project_rows(list: {any}?, pairs_: {ServicePair}, term: string?): string
    local filtered: {GridCard} = {}
    for _, p in ipairs(list or {}) do
        if M.matches(term, { p.name, p.description or "" }) then
            table.insert(filtered, project_card(p, pairs_))
        end
    end
    local rows = {}
    local i = 1
    while i <= #filtered do
        local cards = {}
        for j = i, math.min(i + GRID_COLS - 1, #filtered) do
            table.insert(cards, filtered[j])
        end
        while #cards < GRID_COLS do table.insert(cards, FILLER) end
        table.insert(rows, { cards = cards })
        i = i + GRID_COLS
    end
    return enc_array(rows)
end

-- Linhas Docker (containers gerenciados) derivadas dos serviços, ordenadas.
function M.docker_rows(pairs_: {ServicePair}, term: string?): string
    local sorted: {ServicePair} = {}
    for _, p in ipairs(pairs_) do table.insert(sorted, p) end
    table.sort(sorted, function(a: ServicePair, b: ServicePair) 
        return a.svc.spec.name < b.svc.spec.name 
    end)
    local rows = {}
    for _, p in ipairs(sorted) do
        if M.matches(term, { p.svc.spec.name, p.proj }) then
            local label, _ = status_label_color(p.svc.status)
            local cid = "—"
            if p.svc.live_container_id and p.svc.live_container_id ~= "" then
                cid = string.sub(p.svc.live_container_id :: string, 1, 12)
            end
            table.insert(rows, {
                name = p.svc.spec.name,
                project = p.proj,
                image = source_summary(p.svc.spec.source),
                container = cid,
                status_label = label,
                status_kind = status_kind(p.svc.status),
            })
        end
    end
    return enc_array(rows)
end

function M.docker_images(list: {any}?, term: string?): string
    local sorted = {}
    for _, img in ipairs(list or {}) do table.insert(sorted, img) end
    table.sort(sorted, function(a: any, b: any)
        return table.concat(a.tags or {}, ",") < table.concat(b.tags or {}, ",")
    end)
    local rows = {}
    for _, img in ipairs(sorted) do
        local tagstr = table.concat(img.tags or {}, " ")
        if M.matches(term, { tagstr, img.project or "", img.service or "" }) then
            local in_use = (img.containers or 0) > 0
            local tags = (#(img.tags or {}) == 0) and "<none>" or table.concat(img.tags, ", ")
            local id = img.id or ""
            id = string.gsub(id, "^sha256:", "")
            table.insert(rows, {
                id = string.sub(id, 1, 12),
                tags = tags,
                size = M.fmt_bytes(img.size_bytes),
                created = M.date_dm_hm(img.created),
                project = img.project or "—",
                service = img.service or "—",
                in_use_label = in_use and "EM USO" or "SEM USO",
                in_use_kind = in_use_kind(in_use),
            })
        end
    end
    return enc_array(rows)
end

function M.docker_volumes(list: {any}?, term: string?): string
    local sorted = {}
    for _, v in ipairs(list or {}) do table.insert(sorted, v) end
    table.sort(sorted, function(a: any, b: any) return a.name < b.name end)
    local rows = {}
    for _, v in ipairs(sorted) do
        if M.matches(term, { v.name, v.driver }) then
            table.insert(rows, {
                name = v.name,
                driver = v.driver,
                mountpoint = v.mountpoint,
                size = (v.size_bytes and v.size_bytes >= 0) and M.fmt_bytes(v.size_bytes) or "—",
                in_use_label = v.in_use and "EM USO" or "SEM USO",
                in_use_kind = in_use_kind(v.in_use),
            })
        end
    end
    return enc_array(rows)
end

function M.docker_networks(list: {any}?, term: string?): string
    local sorted = {}
    for _, n in ipairs(list or {}) do table.insert(sorted, n) end
    table.sort(sorted, function(a: any, b: any) return a.name < b.name end)
    local rows = {}
    for _, n in ipairs(sorted) do
        if M.matches(term, { n.name, n.project or "" }) then
            table.insert(rows, {
                name = n.name,
                driver = n.driver,
                scope = n.scope,
                project = n.project or "—",
                containers = tostring(n.container_count or 0),
                in_use_label = n.in_use and "EM USO" or "SEM USO",
                in_use_kind = in_use_kind(n.in_use),
            })
        end
    end
    return enc_array(rows)
end

-- Ingress: uma linha por rota de domínio (não filtrado pela busca). Retorna (json, count).
function M.ingress(pairs_: {ServicePair}): (string, number)
    local rows = {}
    for _, p in ipairs(pairs_) do
        local spec = p.svc.spec
        local port = spec.port
        for _, r in ipairs(domain_routes(spec)) do
            if r.domain and string.gsub(r.domain, "%s", "") ~= "" then
                local scheme = r.tls and "https" or "http"
                local cport = r.port or port
                table.insert(rows, {
                    domain = r.domain,
                    url = scheme .. "://" .. r.domain,
                    service = spec.name,
                    project = p.proj,
                    upstream = ":" .. tostring(cport),
                    tls = r.tls and "TLS" or "—",
                })
            end
        end
    end
    return enc_array(rows), #rows
end

-- Monitoring: uma linha por serviço COM métricas vivas.
function M.monitoring(pairs_: {ServicePair}, metrics: MetricsMap?): string
    local rows = {}
    for _, p in ipairs(pairs_) do
        local m = metrics and metrics[p.svc.id]
        if m then
            table.insert(rows, {
                name = p.svc.spec.name,
                project = p.proj,
                cpu = string.format("%.1f%%", m.cpu_percent or 0),
                mem = M.fmt_bytes(m.mem_used_bytes),
                rx = M.fmt_bytes(m.net_rx_bytes),
                tx = M.fmt_bytes(m.net_tx_bytes),
            })
        end
    end
    return enc_array(rows)
end

-- Deploy Engine: "Executando agora".
function M.eng_active(active: {any}?): string
    local rows = {}
    for _, info in ipairs(active or {}) do
        local label, _ = state_label_color(info.state)
        table.insert(rows, {
            service = info.service_name,
            project = info.project_name,
            state_label = label,
            state_kind = state_kind(info.state),
            percent = tostring(info.percent) .. "%",
            bar = M.progress_bar(info.percent, 12),
            total = M.fmt_secs(info.elapsed_secs),
            phase = M.fmt_secs(info.current_state_secs),
            service_id = info.service_id,
        })
    end
    return enc_array(rows)
end

-- ── Service detail (equivalente às partes de view.rs usadas por services.rs) ──

M.status_label_color = status_label_color

local function is_terminal(state: string?): boolean
    return state == "Live" or state == "Stopped" or state == "Failed" or state == "Pruning"
end

-- ServiceSource → (kind, detail, build_engine) — source_summary().
function M.source_summary3(source: any): (string, string, string)
    if type(source) ~= "table" then return "—", "—", "—" end
    if source.Registry then return "Registry", source.Registry.image or "—", "—"
    elseif source.Git then
        local g = source.Git
        return "Git", (g.url or "") .. " @ " .. (g.branch or ""), g.dockerfile_path or "Dockerfile"
    elseif source.Compose then return "Compose", "docker-compose", "Compose"
    end
    return "—", "—", "—"
end

-- Resumo de uma linha do healthcheck.
function M.healthcheck_summary(hc: any): string
    local k = hc.kind
    local kind: string
    if type(k) == "table" and k.Http then
        kind = "HTTP " .. (k.Http.path or "") .. " → " .. tostring(k.Http.expected_status)
    elseif k == "None" then
        return "disabled"
    elseif k == "Tcp" then kind = "TCP"
    elseif k == "DockerNative" then kind = "Docker"
    else kind = tostring(k) end
    return string.format("%s · every %ss · timeout %ss · %s retries",
        kind, tostring(hc.interval_secs), tostring(hc.timeout_secs), tostring(hc.retries))
end

-- Uma linha de var de ambiente (secret exibido por referência).
local function env_var_row(v: EnvVar)
    local value: string
    local kind: string
    if v.value and v.value.Secret ~= nil then
        value, kind = "secret:" .. (v.value.Secret :: string), "secret"
    else
        value, kind = (v.value and v.value.Plain) or "", "plain"
    end
    return { key = v.key, value = value, kind = kind }
end

-- Env vars como linhas JSON, re-interleaving os comentários `# ...` ancorados
-- (linha `__c<idx>`, idx = índice 0-based em env_comments).
function M.env_json_with_comments(vars: {EnvVar}?, comments: {EnvComment}?): string
    local active_vars = vars or {}
    local active_comments = comments or {}
    local rows = {}
    for _, v in ipairs(active_vars) do
        for i, c in ipairs(active_comments) do
            if c.before_key == v.key then
                table.insert(rows, { key = "__c" .. (i - 1), value = c.text, kind = "comment" })
            end
        end
        table.insert(rows, env_var_row(v))
    end
    for i, c in ipairs(active_comments) do
        if c.before_key == nil then
            table.insert(rows, { key = "__c" .. (i - 1), value = c.text, kind = "comment" })
        end
    end
    return enc_array(rows)
end

-- Env vars como blob `.env` (KEY=VALUE, secrets por referência), com comentários.
function M.env_dotenv_with_comments(vars: {EnvVar}?, comments: {EnvComment}?): string
    local active_vars = vars or {}
    local active_comments = comments or {}
    local lines: {string} = {}
    for _, v in ipairs(active_vars) do
        for _, c in ipairs(active_comments) do
            if c.before_key == v.key then table.insert(lines, c.text) end
        end
        local val: string
        if v.value and v.value.Secret ~= nil then
            val = "<secret:" .. (v.value.Secret :: string) .. ">"
        else
            val = (v.value and v.value.Plain) or ""
        end
        table.insert(lines, tostring(v.key) .. "=" .. val)
    end
    for _, c in ipairs(active_comments) do
        if c.before_key == nil then table.insert(lines, c.text) end
    end
    return table.concat(lines, "\n")
end

-- Linhas de log (LogEntry/BuildLogLine: {stream, line, timestamp}) coloridas.
local function log_rows(list: {any}?): string
    local rows = {}
    for _, e in ipairs(list or {}) do
        local color = (e.stream == "Stderr") and "#F85149" or "#9DA7B3"
        table.insert(rows, { time = M.time_hms(e.timestamp), line = e.line, color = color })
    end
    return enc_array(rows)
end
M.logs_json = log_rows
M.build_logs_json = log_rows

-- Junta linhas de log num blob de texto ("HH:MM:SS line") p/ o TextArea/copiar.
function M.join_log_lines(list: {any}?): string
    local out: {string} = {}
    for _, e in ipairs(list or {}) do
        table.insert(out, M.time_hms(e.timestamp) .. " " .. e.line)
    end
    return table.concat(out, "\n")
end

-- Histórico de deployments de um serviço (aba Deployments).
function M.deployments_detail(list: {any}?): string
    local rows = {}
    for _, d in ipairs(list or {}) do
        local label = state_label_color(d.state)
        table.insert(rows, {
            id = string.sub(d.id, 1, 12),
            id_full = d.id,
            image = d.image,
            state_label = label,
            state_kind = state_kind(d.state),
            duration = fmt_duration(d),
            start = M.date_dm_hms(d.started_at),
            can_delete = is_terminal(d.state) and "1" or "0",
        })
    end
    return enc_array(rows)
end

-- Rotas HTTP de um spec (aba Domains).
function M.domains_json(spec: ServiceSpec): string
    local rows = {}
    for _, r in ipairs(domain_routes(spec)) do
        local cport = r.port or spec.port
        table.insert(rows, {
            domain = r.domain,
            port = tostring(cport),
            tls = r.tls and "true" or "false",
            tls_label = r.tls and "TLS" or "—",
        })
    end
    return enc_array(rows)
end

-- normalize_name: minúsculas, não-alfanumérico → '_' (colapsado), trim '_'.
function M.safe_name(name: string): string
    local out: {string} = {}
    local last_dash = true
    for i = 1, #name do
        local ch = string.sub(name, i, i)
        if string.match(ch, "%w") then
            table.insert(out, string.lower(ch))
            last_dash = false
        elseif not last_dash then
            table.insert(out, "_")
            last_dash = true
        end
    end
    local s = table.concat(out)
    return (s:gsub("^_+", ""):gsub("_+$", ""))
end

local function internal_scheme(db_kind: string?): string?
    local k = db_kind and string.lower(db_kind) or nil
    if k == "postgres" or k == "postgresql" then return "postgres"
    elseif k == "mysql" or k == "mariadb" then return "mysql"
    elseif k == "redis" then return "redis"
    elseif k == "mongodb" or k == "mongo" then return "mongodb"
    elseif k == "rabbitmq" then return "amqp"
    elseif k == "nats" then return "nats"
    elseif k == "kafka" then return nil
    else return "http" end
end

function M.internal_url(db_kind: string?, safe: string, port: any): string
    local host = "rp_" .. safe .. ":" .. tostring(port)
    local scheme = internal_scheme(db_kind)
    if scheme then return scheme .. "://" .. host end
    return host
end

function M.external_url(domain: string?, tls: boolean): string
    if not domain or domain:gsub("%s", "") == "" then return "—" end
    local clean_domain = domain:gsub("/+$", "")
    return (tls and "https" or "http") .. "://" .. clean_domain
end

-- Deploy Engine: "Histórico 24h".
function M.eng_recent(recent: {any}?): string
    local rows = {}
    for _, info in ipairs(recent or {}) do
        local label, _ = state_label_color(info.state)
        local icon = "○"
        if info.state == "Live" then icon = "✓"
        elseif info.state == "Failed" then icon = "✕" end
        table.insert(rows, {
            icon = icon,
            service = info.service_name,
            project = info.project_name,
            state_label = label,
            state_kind = state_kind(info.state),
            duration = M.fmt_secs(info.elapsed_secs),
            start = M.time_hms(info.started_at),
        })
    end
    return enc_array(rows)
end

-- ── Git providers / repos / branches ──────────────────────────────────────

function M.git_providers(list: {any}?): string
    local rows = {}
    for _, p in ipairs(list or {}) do
        local login = p.account and p.account.login or nil
        local connected = login ~= nil
        local display: string
        local account_lbl: string
        local account_login: string
        if connected then
            display = tostring(p.name or "") .. " (@" .. tostring(login or "") .. ")"
            account_lbl = "@" .. tostring(login or "")
            account_login = tostring(login or "")
        else
            display = tostring(p.name or "") .. " — não conectado"
            account_lbl = "(pendente — autorize no navegador)"
            account_login = "não conectado"
        end
        local auth_mode = (p.auth_mode == "OAuth") and "OAuth2" or "PAT"
        table.insert(rows, {
            id = p.id,
            name = p.name,
            display = display,
            base_url = p.base_url,
            auth_mode = auth_mode,
            account = account_lbl,
            account_login = account_login,
            connected = connected and "true" or "false",
        })
    end
    return enc_array(rows)
end

function M.git_repos(list: {any}?): string
    local rows = {}
    for _, r in ipairs(list or {}) do
        table.insert(rows, {
            full_name = r.full_name,
            clone_url = r.clone_url,
            default_branch = r.default_branch,
            private = r.private and "private" or "public",
        })
    end
    return enc_array(rows)
end

function M.git_branches(list: {any}?): string
    local rows = {}
    for _, b in ipairs(list or {}) do
        table.insert(rows, { name = b.name })
    end
    return enc_array(rows)
end

return M
