--!strict
--!nolint FunctionUnused
-- handlers/projects.luau — grade de projetos: criar/editar/remover projeto,
-- variáveis de ambiente de projeto, ações de serviço a partir do card do grid
-- (parar/remover), e o "Stop All" de todo o cluster (topbar).

local State = require("../state")
local Helpers = require("../helpers")
local fmt = require("../fmt")
local Stream = require("stream")

-- Abrir a lista de serviços de um projeto (grade read-only via snapshot).
function open_project(id: string): ()
    ctx.selected_project_id = id
    ctx.view = "project_services"
    ctx.proj_tab = "services"
    ctx.editing_project = "false"
    ctx.proj_action_msg = ""
    ctx.proj_loading = "true"
    Stream.update_open_project()
end

-- ── Criar / remover projeto ──────────────────────────────────────────────

function create_project(): ()
    local api = State.api
    if not api then return end
    local name = Helpers.trim(ctx.new_proj_name)
    if name == "" then
        ctx.erro_new_proj_name = "informe um nome"
        return
    end
    ctx.erro_new_proj_name = ""
    local desc = Helpers.trim(ctx.new_proj_desc)
    local cmd = { ProjectCreate = { name = name, description = (desc ~= "" and desc or nil) } }
    ctx.new_proj_msg = "criando…"
    local ok, res = api:rpc_checked(cmd)
    if ok then
        ctx.new_proj_name = ""
        ctx.new_proj_desc = ""
        ctx.new_proj_msg = "projeto criado"
        Helpers.notify_ok("Projeto \"" .. name .. "\" criado")
        Stream.refresh_now()
    else
        ctx.new_proj_msg = "erro: " .. res
        Helpers.notify_err(res)
    end
end

function delete_project(id: string): ()
    if id == "" then return end
    confirm({
        title = "Remover projeto?",
        message = "O projeto será removido. Só é possível remover projetos sem serviços.",
        confirm_label = "Remover projeto",
        confirm_action = "do_delete_project:" .. tostring(id),
        destructive = true,
    })
end

function do_delete_project(id: string): ()
    local api = State.api
    if not api or id == "" then return end
    ctx.proj_action_msg = "removendo…"
    local ok, res = api:rpc_checked({ ProjectDelete = { id = id } })
    if ok then
        ctx.selected_project_id = ""
        ctx.view = "projects"
        Helpers.notify_ok("Projeto removido")
        Stream.refresh_now()
    else
        ctx.proj_action_msg = "erro: " .. res
        Helpers.notify_err(res)
    end
end

-- ── Editar projeto (cabeçalho de project_services) ──────────────────────

function edit_project_toggle(): ()
    ctx.edit_proj_name = ctx.proj_name or ""
    ctx.edit_proj_desc = ctx.proj_description or ""
    ctx.erro_edit_proj_name = ""
    ctx.editing_project = "true"
end

function cancel_project_edit(): () ctx.editing_project = "false" end

function save_project_edit(): ()
    local pid = ctx.selected_project_id or ""
    local api = State.api
    if pid == "" or not api then return end
    local name = Helpers.trim(ctx.edit_proj_name)
    if name == "" then ctx.erro_edit_proj_name = "informe um nome"; return end
    ctx.erro_edit_proj_name = ""
    local desc = Helpers.trim(ctx.edit_proj_desc)
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

-- ── Variáveis de ambiente do projeto (aba "Variáveis") ──────────────────

-- Busca o projeto fresco, aplica `mutate(env_vars)` (devolve o novo array),
-- envia ProjectEnvSet e reescreve proj_env.
local function with_project_env(mutate: (env: any) -> any): ()
    local pid = ctx.selected_project_id or ""
    local api = State.api
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
        Helpers.notify_err(res2)
    end
end

function penv_add(): ()
    local key = Helpers.trim(ctx.penv_new_key)
    if key == "" then ctx.erro_penv_new_key = "chave vazia"; return end
    ctx.erro_penv_new_key = ""
    local val = ctx.penv_new_val or ""
    with_project_env(function(env: any): any
        local kept = json.array({})
        for _, v in ipairs(env) do if v.key ~= key then table.insert(kept, v) end end
        table.insert(kept, { key = key, value = { Plain = val } })
        return kept
    end)
    ctx.penv_new_key = ""
    ctx.penv_new_val = ""
end

function penv_del(key: string): ()
    with_project_env(function(env: any): any
        local kept = json.array({})
        for _, v in ipairs(env) do if v.key ~= key then table.insert(kept, v) end end
        return kept
    end)
end

function penv_reorder(v: string?): ()
    if not v then return end
    local ok, keys = pcall(json.decode, v)
    if not ok or type(keys) ~= "table" then return end
    with_project_env(function(env: any): any
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

-- ── Ações de serviço a partir de um card do grid do projeto ─────────────
-- O grid se atualiza no próximo snapshot; refresh_now traz isso pra hora.

function svc_stop_id(id: string): ()
    confirm({
        title = "Parar serviço?",
        message = "O tráfego para este serviço será interrompido até um novo deploy.",
        confirm_label = "Parar",
        confirm_action = "do_svc_stop_id:" .. tostring(id),
        destructive = true,
    })
end

function do_svc_stop_id(id: string): ()
    local api = State.api
    if not api then return end
    ctx.proj_action_msg = "enviando…"
    local ok, res = api:rpc_checked({ ServiceStop = { service_id = id } })
    ctx.proj_action_msg = ok and "serviço parado" or ("erro: " .. res)
    if not ok then Helpers.notify_err(res) end
    Stream.refresh_now()
end

function stop_delete_service(id: string): ()
    confirm({
        title = "Parar e remover serviço?",
        message = "O serviço será parado e removido. Essa ação não pode ser desfeita.",
        confirm_label = "Remover",
        confirm_action = "do_stop_delete_service:" .. tostring(id),
        destructive = true,
    })
end

function do_stop_delete_service(id: string): ()
    local api = State.api
    if not api then return end
    ctx.proj_action_msg = "removendo…"
    local ok, res = api:rpc_checked({ ServiceStop = { service_id = id } })
    if not ok then ctx.proj_action_msg = "erro ao parar: " .. res; Helpers.notify_err(res); return end
    local ok2, res2 = api:rpc_checked({ ServiceDelete = { id = id } })
    ctx.proj_action_msg = ok2 and "serviço parado e removido" or ("erro ao remover: " .. res2)
    if ok2 then Helpers.notify_ok("Serviço removido") else Helpers.notify_err(res2) end
    Stream.refresh_now()
end

-- ── Stop All (topbar) ────────────────────────────────────────────────────

function stop_all(): ()
    confirm({
        title = "Parar todos os serviços?",
        message = "Todos os serviços em execução serão parados agora. Você pode reiniciá-los depois com um novo deploy.",
        confirm_label = "Parar todos",
        confirm_action = "do_stop_all",
        destructive = true,
    })
end

function do_stop_all(): ()
    local api = State.api
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
