--!strict
--!nolint FunctionUnused
-- new_project_window.luau — script da JANELA de "Novo projeto", um motor Glacier
-- próprio e ISOLADO do app principal (aberto via open_window). Recebe a conexão
-- (api_url/api_token) semeada no contexto por open_window({ data = ... }); ao
-- criar o projeto emite broadcast("project_created") para a janela principal
-- (recebido por on_broadcast em handlers/projects.luau) e fecha a si mesma.
--
-- require resolve relativo a ESTE arquivo (scripts/): "net/api" e "helpers".

local Api = require("net/api")
local Helpers = require("helpers")

-- Reconstrói o cliente da API a partir da conexão semeada no contexto.
local function client(): Api.Client?
    local url = ctx.api_url or ""
    if url == "" then
        return nil
    end
    return Api.new(url, ctx.api_token)
end

function init(): ()
    ctx.np_name = ctx.np_name or ""
    ctx.np_desc = ctx.np_desc or ""
    ctx.np_err = ""
    ctx.np_msg = ""
end

-- onChange dos inputs (mesma convenção `field:<chave>` do app principal).
function field(key: string, value: string): ()
    ctx[key] = value
end

-- onSubmit/onClick do formulário. `fetch` (dentro de rpc_checked) SUSPENDE a
-- corrotina; ao voltar, em sucesso avisa a janela principal e se fecha.
function submit_project(): ()
    local api = client()
    if not api then
        ctx.np_err = "sem conexão — reabra a janela pelo app"
        return
    end
    local name = Helpers.trim(ctx.np_name)
    if name == "" then
        ctx.np_err = "informe um nome"
        return
    end
    ctx.np_err = ""
    local desc = Helpers.trim(ctx.np_desc)
    ctx.np_msg = "criando…"

    local ok, res = api:rpc_checked{
        ProjectCreate = {
            name = name,
            description = (desc ~= "" and desc or nil),
        },
    }
    if not ok then
        ctx.np_msg = ""
        ctx.np_err = "erro: " .. tostring(res)
        return
    end

    -- Sucesso: envia os dados do projeto criado à janela inicial e fecha esta.
    local p = (type(res) == "table" and res.Project) or {}
    broadcast("project_created", {
        id = p.id,
        name = p.name or name,
        description = p.description or desc,
    })
    close_window()
end

-- Botão "Cancelar": fecha ESTA janela (close_window fecha a janela dona deste
-- motor com precisão, sem depender de qual janela está em foco).
function cancel(): ()
    close_window()
end
