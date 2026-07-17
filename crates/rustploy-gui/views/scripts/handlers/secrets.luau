--!strict
--!nolint FunctionUnused
-- handlers/secrets.luau — aba "Secrets" do projeto: criar/sobrescrever e apagar
-- valores cifrados, e o atalho "usar secret" da aba Variáveis.
--
-- A UI é write-only DE PROPÓSITO: o daemon só devolve os NOMES (SecretList →
-- Response::SecretNames), nunca o blob cifrado, então não há o que exibir nem
-- como "editar" um valor — sobrescrever é salvar outro com o mesmo nome
-- (o SecretSet do daemon faz ON CONFLICT DO UPDATE). Ver docs/secrets.md.
--
-- A lista em si não é buscada aqui: vem no snapshot (`projects[].secrets`),
-- montada por handlers/stream.luau em ctx.proj_secrets/proj_secrets_count.

local State = require("../state")
local Helpers = require("../helpers")
local Stream = require("stream")

-- Nomes atualmente conhecidos (do snapshot já renderizado), para saber se um
-- SecretSet cria ou substitui — o daemon não distingue os dois casos.
local function known_names(): { [string]: boolean }
    local set = {}
    local ok, rows = pcall(json.decode, ctx.proj_secrets or "[]")
    if ok and type(rows) == "table" then
        for _, r in ipairs(rows) do
            set[tostring(r.name)] = true
        end
    end
    return set
end

function secret_add(): ()
    local pid = ctx.selected_project_id or ""
    local api = State.api
    if pid == "" or not api then
        return
    end
    local name = Helpers.trim(ctx.sec_new_name)
    if name == "" then
        ctx.erro_sec_new_name = "informe um nome"
        return
    end
    ctx.erro_sec_new_name = ""
    local value = ctx.sec_new_val or ""
    if value == "" then
        ctx.erro_sec_new_name = "informe um valor"
        return
    end
    local replacing = known_names()[name] == true
    local ok, res = api:rpc_checked{
        SecretSet = { project_id = pid, name = name, value = value }
    }
    if ok then
        ctx.sec_new_name = ""
        ctx.sec_new_val = ""
        toast{
            message = tostring(
                replacing
                    and ("Secret \"" .. name .. "\" substituído — o novo valor vale no próximo deploy")
                    or ("Secret \"" .. name .. "\" criado")
            ),
            kind = "success"
        }
        Stream.refresh_now()
    else
        toast{ message = tostring(res), kind = "error" }
    end
end

function secret_del(name: string): ()
    local pid = ctx.selected_project_id or ""
    if pid == "" or name == "" then
        return
    end
    local ok_confirm = confirm{
        title = "Remover secret?",
        message = "O valor cifrado será apagado e não há como recuperá-lo. "
            .. "Variáveis que referenciam \"" .. name .. "\" passarão a receber "
            .. "valor vazio no próximo deploy.",
        confirm_label = "Remover secret",
        destructive = true,
    }
    if not ok_confirm then
        return
    end
    local api = State.api
    if not api then
        return
    end
    local ok, res = api:rpc_checked{ SecretDelete = { project_id = pid, name = name } }
    if ok then
        toast{ message = tostring("Secret removido"), kind = "success" }
        Stream.refresh_now()
    else
        toast{ message = tostring(res), kind = "error" }
    end
end

-- Chip "usar secret" (aba Variáveis): arma o form de adicionar var em modo
-- secret com o nome escolhido. Quem grava é penv_add (handlers/projects.luau),
-- que nesse modo monta { Secret = nome } em vez de { Plain = valor }.
function secret_use(name: string): ()
    ctx.penv_new_is_secret = "true"
    ctx.penv_new_val = name
    if Helpers.trim(ctx.penv_new_key) == "" then
        ctx.penv_new_key = name
    end
end

function penv_secret_toggle(): ()
    local on = ctx.penv_new_is_secret == "true"
    ctx.penv_new_is_secret = on and "false" or "true"
    ctx.penv_new_val = ""
end
