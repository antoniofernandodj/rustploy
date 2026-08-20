--!strict
--!nolint FunctionUnused
-- handlers/connection.luau — ciclo de vida da sessão: init (semeia o contexto),
-- login/logout, configurações do daemon buscadas uma vez na conexão.

local State = require("../state")
local Helpers = require("../helpers")
local Api = require("../net/api")
local Stream = require("stream")
local Settings = require("settings")

-- ── login lembrado: lista de servidor/token (via o `storage` do glacier) ────
-- Antes era um único par com dois checkboxes "lembrar"; agora toda conexão
-- bem-sucedida salva/atualiza sozinha (ver `connect()`), sem checkbox nenhum,
-- e quem administra mais de um daemon tem a lista inteira num
-- `<ComboEdit>` (login.gv): escolher um servidor já conhecido preenche o
-- token sozinho; digitar um novo deixa o campo livre pra um par ainda não
-- salvo. `storage` grava num arquivo local que sobrevive a restart (fora do
-- `ctx`, que é só memória), em
-- `~/.local/share/rustploy/.glacier-storage/app.json` (raiz definida por
-- `.storage_dir(...)` no Rust).
local saved_servers: { { url: string, token: string } } = {}

-- Publica só as URLs (o `<ComboEdit>` não precisa do token pra montar a
-- lista — o token vem de `token_for` quando um item é escolhido).
local function publish_saved_servers(): ()
    local urls = json.array{}
    for _, s in ipairs(saved_servers) do
        table.insert(urls, s.url)
    end
    ctx.saved_servers = json.encode(urls)
end

local function token_for(url: string): string?
    for _, s in ipairs(saved_servers) do
        if s.url == url then
            return s.token
        end
    end
    return nil
end

-- Salva/atualiza um par e o move pro topo da lista — o mais usado por último
-- fica em primeiro, e é também o que `init()` pré-preenche no próximo boot.
local function remember_server(url: string, token: string): ()
    for i = #saved_servers, 1, -1 do
        if saved_servers[i].url == url then
            table.remove(saved_servers, i)
        end
    end
    table.insert(saved_servers, 1, { url = url, token = token })
    storage.set("saved_servers", saved_servers)
    publish_saved_servers()
end

-- Handler do `<ComboEdit>` de servidor: dispara a cada tecla (`onChange`) e
-- ao escolher um item já salvo (`onSelect`). Se a URL atual bate com um
-- servidor conhecido, preenche o token junto; se não bate com nenhum,
-- LIMPA o token — sem isso, trocar de um servidor salvo pra um digitado na
-- mão mantinha o token do servidor anterior no campo, e um Connect apressado
-- mandava o token errado pro servidor novo.
function servidor_escolhido(value: string): ()
    ctx.token = token_for(value) or ""
end

-- Botão "Esquecer" ao lado do combo: remove o servidor atual da lista salva
-- e limpa os campos — sem isso a tela continuava mostrando o par recém
-- esquecido até a próxima troca de servidor ou reabertura do app.
function esquecer_servidor(): ()
    local url = ctx.url or ""
    for i = #saved_servers, 1, -1 do
        if saved_servers[i].url == url then
            table.remove(saved_servers, i)
        end
    end
    storage.set("saved_servers", saved_servers)
    publish_saved_servers()
    ctx.url = ""
    ctx.token = ""
end

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
    for _, k in ipairs{
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
    } do
        if not ctx[k] or ctx[k] == "" then
            ctx[k] = "[]"
        end
    end
    -- Contadores nascem "…", não "0": antes do primeiro snapshot não sabemos
    -- quantos são, e "0" AFIRMA vazio — a UI piscava "Projects (0)" → "Projects
    -- (2)", e mensagens como "Nenhum job cadastrado ainda" apareciam durante o
    -- carregamento. O snapshot troca todos por número (apply_snapshot em
    -- handlers/stream.luau, junto de `data_loading = "false"`). Os `equals="0"`
    -- dos templates continuam valendo: com "…" não casam, então o estado vazio
    -- só pinta quando o dado realmente chegou e é zero.
    for _, k in ipairs{
        "deployments_count",
        "projects_count",
        "services_count",
        "ingress_count",
        "host_ports_count",
        "jobs_count",
        "proj_env_count",
        "proj_jobs_count",
        "proj_secrets_count",
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
    } do
        if not ctx[k] or ctx[k] == "" then
            ctx[k] = "…"
        end
    end
    -- Semeia o formulário de login com a lista de servidores salvos, e
    -- pré-preenche url/token com o mais usado por último (topo da lista) —
    -- para nascer pronto pra conectar de novo sem escolher nada.
    local stored = storage.get("saved_servers")
    if type(stored) == "table" then
        saved_servers = stored
    end
    publish_saved_servers()
    if #saved_servers > 0 and (not ctx.url or ctx.url == "") then
        ctx.url = saved_servers[1].url
        ctx.token = saved_servers[1].token
    end
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
    Settings.dc_load()
end

-- on_submit/on_click do formulário de login. `fetch` suspende a corrotina, então
-- validamos a conexão com um DaemonStatus antes de abrir o stream.
function connect(): ()
    local base = Helpers.normalize_url(ctx.url)
    if not base then
        ctx.erro_url = "URL inválida (ex.: https://rustploy.domain.tech)"
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

    -- Só salva na lista quando a conexão realmente deu certo — uma tentativa
    -- com URL/token errados não deve virar entrada salva.
    remember_server(base, token)

    ctx.connected = "true"
    ctx.status_line = "conectado"
    ctx.screen = "shell"
    if not ctx.view or ctx.view == "" then
        ctx.view = "deployments"
    end

    load_settings()
    Stream.open_stream()
end

-- Logout: nada da sessão anterior pode sobreviver em RAM — nem no `ctx` (o
-- espelho do contexto do motor, de onde a UI pinta tudo: nomes de projeto,
-- linhas de log, api_token…) nem no `State` (cache do snapshot/métricas).
--
-- Em vez de enumerar as chaves a limpar — a lista cresce a cada tela nova e
-- sempre esquece alguma — apagamos o `ctx` INTEIRO e deixamos `init()` semear
-- os defaults como no boot. O login lembrado volta do `storage` (disco), não da
-- memória, então conectar de novo continua nascendo preenchido.
--
-- `ctx[k] = nil` é remoção de verdade, não só sumiço do lado Luau: o glacier
-- espelha o contexto na tabela no início da execução e, no fim, remove do motor
-- toda chave ausente nela (sync_to_luau/sync_from_luau).
function disconnect(): ()
    if State.state_conn then
        pcall(function()
            State.state_conn:close()
        end)
        State.state_conn = nil
    end
    State.api = nil
    State.snap = nil
    State.last_pairs = {}
    State.metrics_by_id = {}
    State.jobs_by_id = {}
    State.ns_templates_all = {}
    State.search_term = ""
    State.deploy_track =
        { service_id = "", deployment_id = "", running = false, name = "" }

    -- Coleta as chaves antes de apagar: mutar a tabela durante o próprio
    -- `pairs` sobre ela é comportamento indefinido.
    local data = ctx :: { [string]: any }
    local keys: { string } = {}
    for k in pairs(data) do
        table.insert(keys, k)
    end
    for _, k in ipairs(keys) do
        data[k] = nil
    end

    init()
    ctx.connected = "false"
    ctx.screen = "login"
    ctx.status_line = "desconectado"
end
