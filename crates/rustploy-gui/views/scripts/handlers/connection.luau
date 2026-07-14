--!strict
--!nolint FunctionUnused
-- handlers/connection.luau — ciclo de vida da sessão: init (semeia o contexto),
-- login/logout, configurações do daemon buscadas uma vez na conexão.

local State = require("../state")
local Helpers = require("../helpers")
local Api = require("../net/api")
local Stream = require("stream")

-- ── init: semeia o contexto para a UI pintar antes de qualquer dado ─────────
function init(): ()
    if not ctx.screen or ctx.screen == "" then
        ctx.screen = "login"
    end
    if not ctx.view or ctx.view == "" then
        ctx.view = "deployments"
    end
    ctx.connected = ctx.connected or "false"
    ctx.status_line = ctx.status_line or "pronto para conectar"
    ctx.error = ctx.error or ""
    ctx.search = ctx.search or ""
    ctx.data_loading = "true"
    -- Listas vazias como "[]" (não nil) para os ForEach não verem literal.
    for _, k in ipairs({
        "deployments",
        "projects",
        "project_rows",
        "services",
        "service_rows",
        "docker_rows",
        "docker_images",
        "docker_volumes",
        "docker_networks",
        "ingress",
        "host_ports",
        "jobs_summary",
        "proj_jobs",
        "monitoring",
        "eng_active",
        "eng_recent",
        "project_services",
    }) do
        if not ctx[k] or ctx[k] == "" then
            ctx[k] = "[]"
        end
    end
    for _, k in ipairs({
        "deployments_count",
        "projects_count",
        "services_count",
        "ingress_count",
        "host_ports_count",
        "jobs_count",
        "proj_jobs_count",
        "docker_images_count",
        "docker_volumes_count",
        "docker_networks_count",
        "eng_active_count",
        "eng_recent_count",
        "eng_success_24h",
        "eng_failed_24h",
        "eng_total_24h",
        "services_running",
        "services_total",
    }) do
        if not ctx[k] or ctx[k] == "" then
            ctx[k] = "0"
        end
    end
    ctx.remember_url = ctx.remember_url or "false"
    ctx.remember_token = ctx.remember_token or "false"
end

function toggle_remember_url(v: string): ()
    ctx.remember_url = v
end
function toggle_remember_token(v: string): ()
    ctx.remember_token = v
end

-- Configurações do daemon + provedores Git, buscados uma vez na conexão (não
-- entram no snapshot para não sobrescrever campos que o usuário edita).
local function load_settings(): ()
    local api = State.api
    if not api then return end
    local ok, res = api:rpc("GetDaemonSettings")
    if ok and res.DaemonSettings then
        -- Base pública derivada pelo daemon a partir de [api] domain/port — é
        -- só-leitura (não há setter no protocolo), serve para exibir a base do
        -- webhook e montar a redirect URI do OAuth.
        local base = res.DaemonSettings.public_base_url or ""
        ctx.ss_public_base = base
        ctx.ss_email = res.DaemonSettings.acme_email or ""
        ctx.ss_registry_domain = res.DaemonSettings.registry_domain or ""
        ctx.gp_redirect = Helpers.oauth_redirect_uri(base)
    end
    local ok2, res2 = api:rpc("GitProviderList")
    if ok2 and res2.GitProviders then
        ctx.gitea_count = tostring(#res2.GitProviders)
    end
end

-- on_submit/on_click do formulário de login. `fetch` suspende a corrotina, então
-- validamos a conexão com um DaemonStatus antes de abrir o stream.
function connect(): ()
    local base = Helpers.normalize_url(ctx.url)
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

    local client = Api.new(base, token)
    State.api = client
    local ok, res = client:rpc("DaemonStatus")
    if not ok then
        State.api = nil
        ctx.error = res
        ctx.status_line = "falha na conexão"
        ctx.connected = "false"
        return
    end

    ctx.connected = "true"
    ctx.status_line = "conectado"
    ctx.screen = "shell"
    if not ctx.view or ctx.view == "" then
        ctx.view = "deployments"
    end

    load_settings()
    Stream.open_stream()
end

function disconnect(): ()
    if State.state_conn then
        pcall(function()
            State.state_conn:close()
        end)
        State.state_conn = nil
    end
    State.api = nil
    State.snap = nil
    ctx.connected = "false"
    ctx.screen = "login"
    ctx.status_line = "desconectado"
end
