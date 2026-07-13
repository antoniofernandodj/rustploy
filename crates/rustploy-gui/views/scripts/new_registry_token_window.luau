--!strict
--!nolint FunctionUnused
-- new_registry_token_window.luau — script da JANELA "Novo token do registry",
-- um motor Glacier próprio e ISOLADO do app principal (aberto via open_window
-- por handlers/registry.luau::registry_open_token_window). Recebe a conexão
-- (api_url/api_token) e o host do registry (registry_host) já conhecidos pela
-- janela principal, semeados via open_window({ data = ... }).
--
-- require resolve relativo a ESTE arquivo (scripts/): "net/api" e "helpers".

local Api = require("net/api")
local Helpers = require("helpers")

local function client(): Api.Client?
    local url = ctx.api_url or ""
    if url == "" then
        return nil
    end
    return Api.new(url, ctx.api_token)
end

function init(): ()
    -- ctx.registry_host já chega pronto (semeado via data).
    ctx.registry_host = ctx.registry_host or "127.0.0.1:5100"

    ctx.ntok_step = "form"
    ctx.ntok_name = ""
    ctx.ntok_scope = "pull"
    ctx.ntok_err = ""
    ctx.ntok_msg = ""
    ctx.ntok_login_cmd = ""
end

-- onChange dos inputs (mesma convenção `field:<chave>` do app principal).
function field(key: string?, v: string?): ()
    if key then
        ctx[key] = v or ""
    end
end

function ntok_scope(s: string): ()
    ctx.ntok_scope = s
end

function ntok_create(): ()
    local api = client()
    if not api then
        ctx.ntok_err = "sem conexão — reabra a janela pelo app"
        return
    end
    local name = Helpers.trim(ctx.ntok_name)
    if name == "" then
        ctx.ntok_err = "informe um nome"
        return
    end
    ctx.ntok_err = ""
    ctx.ntok_msg = "criando…"

    local ok, res = api:rpc_checked({
        RegistryTokenCreate = { name = name, scope = ctx.ntok_scope },
    })
    if not ok then
        ctx.ntok_msg = ""
        ctx.ntok_err = "erro: " .. tostring(res)
        return
    end

    local created = (type(res) == "table" and res.RegistryTokenCreated) or {}
    local secret = created.secret or ""
    ctx.ntok_login_cmd = string.format(
        "docker login %s -u %s -p %s",
        ctx.registry_host,
        created.name or name,
        secret
    )
    ctx.ntok_step = "reveal"

    -- Avisa a janela principal já — não espera o "Fechar" (o usuário pode
    -- demorar pra copiar o segredo, ou nunca fechar explicitamente).
    broadcast("registry_token_created", { name = created.name or name })
end
