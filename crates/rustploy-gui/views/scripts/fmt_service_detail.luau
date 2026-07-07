--!strict
-- fmt_service_detail.luau — builders da tela de detalhe de serviço (source,
-- healthcheck, env vars, logs, deployments, domains, connection URLs).
-- Equivalente às partes de view.rs usadas por services.rs.

local Types = require("fmt_types")
type EnvVar = Types.EnvVar
type EnvComment = Types.EnvComment
type ServiceSpec = Types.ServiceSpec

local Time = require("fmt_time")
local Util = require("fmt_util")

local M = {}

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
    return Util.enc_array(rows)
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
        table.insert(rows, { time = Time.time_hms(e.timestamp), line = e.line, color = color })
    end
    return Util.enc_array(rows)
end
M.logs_json = log_rows
M.build_logs_json = log_rows

-- Junta linhas de log num blob de texto ("HH:MM:SS line") p/ o TextArea/copiar.
function M.join_log_lines(list: {any}?): string
    local out: {string} = {}
    for _, e in ipairs(list or {}) do
        table.insert(out, Time.time_hms(e.timestamp) .. " " .. e.line)
    end
    return table.concat(out, "\n")
end

-- Histórico de deployments de um serviço (aba Deployments).
function M.deployments_detail(list: {any}?): string
    local rows = {}
    for _, d in ipairs(list or {}) do
        local label = Util.state_label_color(d.state)
        table.insert(rows, {
            id = string.sub(d.id, 1, 12),
            id_full = d.id,
            image = d.image,
            state_label = label,
            state_kind = Util.state_kind(d.state),
            duration = Time.fmt_duration(d),
            start = Time.date_dm_hms(d.started_at),
            can_delete = is_terminal(d.state) and "1" or "0",
        })
    end
    return Util.enc_array(rows)
end

-- Rotas HTTP de um spec (aba Domains).
function M.domains_json(spec: ServiceSpec): string
    local rows = {}
    for _, r in ipairs(Util.domain_routes(spec)) do
        local cport = r.port or spec.port
        table.insert(rows, {
            domain = r.domain,
            port = tostring(cport),
            tls = r.tls and "true" or "false",
            tls_label = r.tls and "TLS" or "—",
        })
    end
    return Util.enc_array(rows)
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

return M
