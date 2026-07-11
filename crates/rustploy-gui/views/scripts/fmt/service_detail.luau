--!strict
-- fmt/service_detail.luau — builders da tela de detalhe de serviço (source,
-- healthcheck, env vars, logs, deployments, domains, connection URLs).
-- Equivalente às partes de view.rs usadas por services.rs.

local Types = require("types")
type EnvVar = Types.EnvVar
type EnvComment = Types.EnvComment
type ServiceSpec = Types.ServiceSpec

local Time = require("time")
local Util = require("util")

local M = {}

-- Lista de containers do serviço (id + nome + estado) → linhas para a UI de
-- detalhe. `list` = Vec<ManagedContainer> anexado ao serviço no snapshot.
function M.containers(list: { Types.ManagedContainer }?): string
    local rows = {}
    for _, c in ipairs(list or {}) do
        local color = "#8B949E"
        if c.state == "running" then color = "#3FB950"
        elseif c.state == "restarting" or c.state == "created" or c.state == "paused" then
            color = "#D29922"
        end
        table.insert(rows, {
            id = c.id or "",
            id_short = string.sub(c.id or "", 1, 12),
            name = c.name or "—",
            state = c.state or "—",
            state_color = color,
        })
    end
    return Util.enc_array(rows)
end

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

-- Nº máx. de linhas materializadas no `for-each` de log. O `<for-each>` do
-- glacier não é virtualizado: cada linha vira widgets reconstruídos a CADA
-- re-render (por evento de log). Um build log histórico com milhares de linhas
-- reconstruído por evento congela a UI, então o for-each mostra só a cauda; o
-- texto completo continua em join_log_lines (aba Logs "Copiar tudo" / textão).
local RENDER_ROWS = 500
-- Teto de comprimento por linha (defensivo contra linhas absurdas que sozinhas
-- estouram o text-shaping). O texto de "Copiar tudo" preserva a linha inteira.
local LINE_MAX = 2000

-- Linhas de log (LogEntry/BuildLogLine: {stream, line, timestamp}) coloridas,
-- limpas de ANSI e limitadas à cauda (RENDER_ROWS) p/ o custo de render ficar
-- limitado independentemente do tamanho do histórico.
local function log_rows(list: {any}?): string
    local src = list or {}
    local start = math.max(1, #src - RENDER_ROWS + 1)
    local rows = {}
    for i = start, #src do
        local e = src[i]
        local color = (e.stream == "Stderr") and "#F85149" or "#9DA7B3"
        local line = Util.ellipsis(Util.strip_ansi(e.line), LINE_MAX)
        table.insert(rows, { time = Time.time_hms(e.timestamp), line = line, color = color })
    end
    return Util.enc_array(rows)
end
M.logs_json = log_rows
M.build_logs_json = log_rows

-- Junta linhas de log num blob de texto ("HH:MM:SS line") p/ o TextArea/copiar.
-- Sem ANSI (mas sem cap de linhas: é o conteúdo completo p/ copiar).
function M.join_log_lines(list: {any}?): string
    local out: {string} = {}
    for _, e in ipairs(list or {}) do
        table.insert(out, Time.time_hms(e.timestamp) .. " " .. Util.strip_ansi(e.line))
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
    if k == "postgres" or k == "postgresql" then return "postgresql"
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

-- Extrai só o host (sem esquema/porta) de uma api_url tipo "https://1.2.3.4:8443".
local function url_host(api_url: string?): string?
    local u = api_url and api_url:gsub("%s", "") or ""
    if u == "" then return nil end
    local rest = string.match(u, "^[%a][%w+.-]*://(.*)$") or u
    local host = string.match(rest, "^([^:/]+)")
    return host and host ~= "" and host or nil
end

-- Valor Plain de uma env var pela chave (secrets/ausentes → nil).
local function env_plain(vars: {EnvVar}?, key: string): string?
    for _, v in ipairs(vars or {}) do
        if v.key == key and v.value and v.value.Plain ~= nil and v.value.Plain ~= "" then
            return v.value.Plain
        end
    end
    return nil
end

-- Credenciais (database/user/password) de um serviço de banco/broker, lidas das
-- env vars nas mesmas convenções que o wizard grava (ver shared::wizard).
-- Retorna nil em cada campo que não se aplica ao db_kind.
local function db_credentials(db_kind: string?, vars: {EnvVar}?): (string?, string?, string?)
    local k = db_kind and string.lower(db_kind) or nil
    if k == "postgres" or k == "postgresql" then
        return env_plain(vars, "POSTGRES_DB"), env_plain(vars, "POSTGRES_USER"), env_plain(vars, "POSTGRES_PASSWORD")
    elseif k == "mysql" or k == "mariadb" then
        return env_plain(vars, "MYSQL_DATABASE"), env_plain(vars, "MYSQL_USER"), env_plain(vars, "MYSQL_PASSWORD")
    elseif k == "mongodb" or k == "mongo" then
        return nil, env_plain(vars, "MONGO_INITDB_ROOT_USERNAME"), env_plain(vars, "MONGO_INITDB_ROOT_PASSWORD")
    elseif k == "redis" then
        return nil, nil, env_plain(vars, "REDIS_PASSWORD")
    elseif k == "rabbitmq" then
        return nil, env_plain(vars, "RABBITMQ_DEFAULT_USER"), env_plain(vars, "RABBITMQ_DEFAULT_PASS")
    end
    return nil, nil, nil
end

-- Anexa `/database?user=…&password=…` a uma URL de banco, no formato pedido
-- (query-params, ex.: postgresql://host:5432/app?user=app&password=…).
-- Só inclui os campos presentes; sem credenciais, retorna a base intacta.
local function with_db_credentials(base: string, database: string?, user: string?, password: string?): string
    local url = base
    if database and database ~= "" then
        url = url .. "/" .. database
    end
    local params: {string} = {}
    if user and user ~= "" then table.insert(params, "user=" .. user) end
    if password and password ~= "" then table.insert(params, "password=" .. password) end
    if #params > 0 then
        url = url .. "?" .. table.concat(params, "&")
    end
    return url
end

-- Percent-encode de um componente de userinfo (user/password) — protege chars
-- estruturais do URI (`@`, `:`, `/`, `?`, `#`, …). Senhas do wizard usam base64
-- url-safe (só passa reto), mas senhas custom podem ter reservados.
local function pct(s: string): string
    return (s:gsub("[^%w%-._~]", function(c: string): string
        return string.format("%%%02X", string.byte(c))
    end))
end

-- Monta a parte de userinfo `user:pass@` de uma URI RFC 3986. Casos:
--   user+pass → "user:pass@"  |  só pass (ex.: Redis) → ":pass@"
--   só user   → "user@"       |  nenhum → "" (sem `@`)
local function userinfo(user: string?, password: string?): string
    local u = (user and user ~= "") and pct(user) or ""
    local p = (password and password ~= "") and pct(password) or ""
    if u == "" and p == "" then return "" end
    if p == "" then return u .. "@" end
    return u .. ":" .. p .. "@"
end

-- Esquema de URI externo por banco (userinfo). Difere do `internal_scheme` em
-- dois pontos: MariaDB ganha `mariadb://` próprio (mais preciso que `mysql://`),
-- e Postgres NÃO entra aqui (é tratado à parte como string JDBC). `nil` = sem
-- esquema de banco (Kafka/serviço comum) → passthrough TCP puro `host:porta`.
local function external_scheme(k: string?): string?
    if k == "mysql" then return "mysql"
    elseif k == "mariadb" then return "mariadb"
    elseif k == "redis" then return "redis"
    elseif k == "mongodb" or k == "mongo" then return "mongodb"
    elseif k == "rabbitmq" then return "amqp"
    elseif k == "nats" then return "nats"
    end
    return nil
end

-- URL de conexão externa por tipo de banco, sobre o host:porta do daemon.
--   • Postgres → string JDBC (jdbc:postgresql://host:porta/db?user=&password=),
--     reaproveitando `with_db_credentials` (formato que os drivers JDBC/libpq
--     aceitam — confirmado em uso).
--   • Demais → URI padrão RFC com credenciais no userinfo
--     (scheme://user:pass@host:porta/db). Redis omite user (":pass@"); Mongo põe
--     barra final + `?authSource=admin` (o root do MONGO_INITDB mora no admin).
--   • Sem esquema de banco (Kafka/serviço comum) → só `host:porta`.
local function db_connection_url(
    db_kind: string?,
    host: string,
    port: string,
    database: string?,
    user: string?,
    password: string?
): string
    local k = db_kind and string.lower(db_kind) or nil
    local hp = host .. ":" .. port

    if k == "postgres" or k == "postgresql" then
        return "jdbc:" .. with_db_credentials("postgresql://" .. hp, database, user, password)
    end

    local scheme = external_scheme(k)
    if not scheme then
        return hp
    end

    local url = scheme .. "://" .. userinfo(user, password) .. hp
    if database and database ~= "" then
        url = url .. "/" .. database
    elseif k == "mongodb" or k == "mongo" then
        url = url .. "/"
    end
    if (k == "mongodb" or k == "mongo") and user and user ~= "" then
        url = url .. "?authSource=admin"
    end
    return url
end

-- Domínio HTTP tem prioridade (rota de ingress); na ausência dele, cai para o
-- passthrough TCP (`host_port`) usando o host do daemon (`api_url` da sessão) e a
-- URL de conexão idiomática do `db_kind` (ver `db_connection_url`), com database
-- + credenciais lidas das env vars.
function M.external_url(
    domain: string?,
    tls: boolean,
    host_port: any,
    db_kind: string?,
    api_url: string?,
    env_vars: {EnvVar}?
): string
    local database, user, password = db_credentials(db_kind, env_vars)
    if domain and domain:gsub("%s", "") ~= "" then
        local clean_domain = domain:gsub("/+$", "")
        return (tls and "https" or "http") .. "://" .. clean_domain
    end
    if host_port and tostring(host_port) ~= "" then
        local host = url_host(api_url) or "<host>"
        return db_connection_url(db_kind, host, tostring(host_port), database, user, password)
    end
    return "—"
end

return M
