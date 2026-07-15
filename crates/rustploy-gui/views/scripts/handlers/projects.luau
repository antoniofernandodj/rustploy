--!strict
--!nolint FunctionUnused
-- handlers/projects.luau — grade de projetos: criar/editar/remover projeto,
-- variáveis de ambiente de projeto, ações de serviço a partir do card do grid
-- (parar/remover), e o "Stop All" de todo o cluster (topbar).

local State = require("../state")
local Helpers = require("../helpers")
local fmt = require("../fmt")
local Stream = require("stream")
local Registry = require("registry")

-- Abrir a lista de serviços de um projeto (grade read-only via snapshot).
function open_project(id: string): ()
    ctx.selected_project_id = id
    ctx.view = "project_services"
    ctx.proj_tab = "services"
    ctx.editing_project = "false"
    ctx.proj_action_msg = ""
    ctx.penv_text_open = "false"
    ctx.proj_loading = "true"
    Stream.update_open_project()
end

-- ── Criar projeto (agora em uma JANELA à parte) ──────────────────────────

-- Abre a janela de cadastro de projeto (motor Glacier próprio, isolado). Passa a
-- conexão atual (api_url/api_token) via `data` para que a janela chame
-- ProjectCreate por conta própria; ao concluir, ela emite broadcast(
-- "project_created"), recebido por `on_broadcast` abaixo. Ver
-- views/new_project_form.gv + scripts/new_project_window.luau.
function open_new_project_window(): ()
    open_window{
        file = "crates/rustploy-gui/views/new_project_form.gv",
        title = "Novo projeto — Rustploy",
        width = 460,
        height = 340,
        resizable = false,  -- diálogo fixo (borderless, sem alças de resize)
        data = {
            api_url = ctx.api_url or "",
            api_token = ctx.api_token or "",
        },
    }
end

-- Recebe mensagens de OUTRAS janelas (glacier broadcast). Em ambos os casos
-- força um refresh imediato do snapshot (o card novo aparece na hora, sem
-- esperar o próximo tick do SSE) e confirma num toast:
--   "project_created" — a janela de cadastro de projeto concluiu.
--   "service_created" — a janela do wizard de serviço concluiu (a grade do
--                       projeto aberto é reconstruída pelo update_open_project).
function on_broadcast(event: string, payload: any): ()
    local name = (type(payload) == "table" and payload.name) or ""
    if event == "project_created" then
        toast{ message = tostring("Projeto \"" .. (name ~= "" and name or "projeto") .. "\" criado"), kind = "success" }
        Stream.refresh_now()
    elseif event == "service_created" then
        toast{ message = tostring("Serviço \"" .. (name ~= "" and name or "serviço") .. "\" criado"), kind = "success" }
        Stream.refresh_now()
    elseif event == "job_created" then
        toast{ message = tostring("Job \"" .. (name ~= "" and name or "job") .. "\" criado"), kind = "success" }
        Stream.refresh_now()
    elseif event == "registry_token_created" then
        toast{ message = tostring("Token \"" .. (name ~= "" and name or "token") .. "\" criado"), kind = "success" }
        Registry.refresh_tokens()
    end
end

-- ── Criar / remover projeto ──────────────────────────────────────────────

-- NOTA: create_project (formulário inline, removido da UI em favor da janela
-- acima) é mantido por ser reutilizável — um fluxo de criação embutido poderia
-- voltar a chamá-lo. Hoje não há call-site na UI.
function create_project(): ()
    local api = State.api
    if not api then
        return
    end
    local name = Helpers.trim(ctx.new_proj_name)
    if name == "" then
        ctx.erro_new_proj_name = "informe um nome"
        return
    end
    ctx.erro_new_proj_name = ""
    local desc = Helpers.trim(ctx.new_proj_desc)
    local cmd = {
        ProjectCreate = {
            name = name,
            description = (desc ~= "" and desc or nil)
        }
    }
    ctx.new_proj_msg = "criando…"
    local ok, res = api:rpc_checked(cmd)
    if ok then
        ctx.new_proj_name = ""
        ctx.new_proj_desc = ""
        ctx.new_proj_msg = "projeto criado"
        toast{ message = tostring("Projeto \"" .. name .. "\" criado"), kind = "success" }
        Stream.refresh_now()
    else
        ctx.new_proj_msg = "erro: " .. res
        toast{ message = tostring(res), kind = "error" }
    end
end

function delete_project(id: string): ()
    if id == "" then
        return
    end
    local ok_confirm = confirm{
        title = "Remover projeto?",
        message = "O projeto será removido. Só é possível remover projetos sem serviços.",
        confirm_label = "Remover projeto",
        destructive = true,
    }

    if not ok_confirm then
        return
    end

    local api = State.api
    if not api or id == "" then
        return
    end
    ctx.proj_action_msg = "removendo…"
    local ok, res = api:rpc_checked{ ProjectDelete = { id = id } }
    if ok then
        ctx.selected_project_id = ""
        ctx.view = "projects"
        toast{ message = tostring("Projeto removido"), kind = "success" }
        Stream.refresh_now()
    else
        ctx.proj_action_msg = "erro: " .. res
        toast{ message = tostring(res), kind = "error" }
    end
end



-- ── Editar projeto (cabeçalho de project_services) ──────────────────────

function edit_project_toggle(): ()
    ctx.edit_proj_name = ctx.proj_name or ""
    ctx.edit_proj_desc = ctx.proj_description or ""
    ctx.erro_edit_proj_name = ""
    ctx.editing_project = "true"
end

function cancel_project_edit(): ()
    ctx.editing_project = "false"
end

function save_project_edit(): ()
    local pid = ctx.selected_project_id or ""
    local api = State.api
    if pid == "" or not api then
        return
    end
    local name = Helpers.trim(ctx.edit_proj_name)
    if name == "" then
        ctx.erro_edit_proj_name = "informe um nome";
        return
    end
    ctx.erro_edit_proj_name = ""
    local desc = Helpers.trim(ctx.edit_proj_desc)
    local ok, res = api:rpc_checked{
        ProjectUpdate = {
            id = pid,
            name = name,
            description = (desc ~= "" and desc or nil)
        }
    }
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
-- Fluxo idêntico ao do serviço: env_vars + env_comments (`# ...` ancorados por
-- key). O `mutate` recebe uma tabela `pe = { env_vars, env_comments }` e a
-- altera in place; enviamos as duas listas no ProjectEnvSet.

local function with_project_env(mutate: (pe: any) -> ()): ()
    local pid = ctx.selected_project_id or ""
    local api = State.api
    if pid == "" or not api then
        return
    end
    local ok, res = api:rpc("ProjectList")
    if not ok or not res.Projects then
        return
    end
    local proj
    for _, p in ipairs(res.Projects) do
        if p.id == pid then
            proj = p;
            break
        end
    end
    if proj == nil then
        return
    end
    local pe = {
        env_vars = proj.env_vars or json.array{},
        env_comments = proj.env_comments or json.array{},
    }
    mutate(pe)
    local ok2, res2 = api:rpc_checked{
        ProjectEnvSet = {
            project_id = pid,
            env_vars = pe.env_vars,
            env_comments = pe.env_comments,
        }
    }
    if ok2 then
        ctx.proj_env = fmt.env_json_with_comments(pe.env_vars, pe.env_comments)
        ctx.proj_env_count = tostring(#pe.env_vars)
        -- Não sobrescrever o textarea enquanto o usuário edita (ver stream.luau).
        if ctx.penv_text_open ~= "true" then
            local dotenv = fmt.env_dotenv_with_comments(pe.env_vars, pe.env_comments)
            ctx.proj_env_text = dotenv
            ctx.proj_env_text_orig = dotenv
        end
    else
        toast{ message = tostring(res2), kind = "error" }
    end
end

-- Parser .env (KEY=VALUE, # comentário, <secret:nome>) → (env_vars, comments).
-- Comentários pendentes ancoram na próxima var real (before_key); os do fim
-- ficam sem âncora (before_key = nil).
local function parse_project_dotenv(text: string?): ({any}, {any})
    local vars = json.array{}
    local comments = json.array{}
    local pending = {}
    for line in string.gmatch((text or "") .. "\n", "([^\n]*)\n") do
        local l = Helpers.trim(line)
        if l == "" then
            -- ignora
        elseif l:sub(1, 1) == "#" then
            table.insert(pending, l)
        else
            local k, v = l:match("^([^=]+)=(.*)$")
            if k and v then
                local key = Helpers.trim(k)
                if key ~= "" then
                    for _, c in ipairs(pending) do
                        table.insert(comments, { text = c, before_key = key })
                    end
                    pending = {}
                    v = Helpers.trim(v)
                    local secret = v:match("^<secret:(.+)>$")
                    local value = secret and { Secret = secret } or { Plain = v }
                    table.insert(vars, { key = key, value = value })
                end
            end
        end
    end
    for _, c in ipairs(pending) do
        table.insert(comments, { text = c, before_key = nil })
    end
    return vars, comments
end

-- Reordena vars/comments a partir de uma ordem de keys (vars + linhas
-- `__c<idx>`): cada comentário reancora na primeira var real que o segue.
local function reorder_project_env(vars: {any}, comments: {any}, keys: {any}): ({any}, {any})
    local var_keys = {}
    local by_key = {}
    for _, v in ipairs(vars) do var_keys[v.key] = true; by_key[v.key] = v end
    local seen = {}
    local new_vars = json.array{}
    local new_comments = json.array{}
    for pos, k in ipairs(keys) do
        local ci = k:match("^__c(%d+)$")
        if ci then
            local idx = (tonumber(ci) or 0) + 1 -- __c é 0-based
            local c = comments[idx]
            if c then
                seen[idx] = true
                local anchor = nil
                for j = pos + 1, #keys do
                    if var_keys[keys[j]] then anchor = keys[j]; break end
                end
                table.insert(new_comments, { text = c.text, before_key = anchor })
            end
        elseif by_key[k] then
            table.insert(new_vars, by_key[k])
            by_key[k] = nil
        end
    end
    for _, v in ipairs(vars) do
        if by_key[v.key] then
            table.insert(new_vars, v)
        end
    end
    for idx, c in ipairs(comments) do
        if not seen[idx] then
            table.insert(new_comments, c)
        end
    end
    return new_vars, new_comments
end

function penv_add(): ()
    local key = Helpers.trim(ctx.penv_new_key)
    if key == "" then
        ctx.erro_penv_new_key = "chave vazia";
        return
    end
    ctx.erro_penv_new_key = ""
    local val = ctx.penv_new_val or ""
    with_project_env(function(pe: any): ()
        local kept = json.array{}
        for _, v in ipairs(pe.env_vars) do
            if v.key ~= key then
                table.insert(kept, v)
            end
        end
        table.insert(kept, { key = key, value = { Plain = val } })
        pe.env_vars = kept
    end)
    ctx.penv_new_key = ""
    ctx.penv_new_val = ""
end

function penv_del(key: string): ()
    with_project_env(function(pe: any): ()
        local kept = json.array{}
        for _, v in ipairs(pe.env_vars) do
            if v.key ~= key then
                table.insert(kept, v)
            end
        end
        pe.env_vars = kept
        local kc = json.array{}
        for _, c in ipairs(pe.env_comments) do
            if c.before_key ~= key then
                table.insert(kc, c)
            end
        end
        pe.env_comments = kc
    end)
end

function penv_reorder(v: string?): ()
    if not v then
        return
    end
    local ok, keys = pcall(json.decode, v)
    if not ok or type(keys) ~= "table" then
        return
    end
    with_project_env(function(pe: any): ()
        local nv, nc = reorder_project_env(pe.env_vars, pe.env_comments, keys)
        pe.env_vars = nv
        pe.env_comments = nc
    end)
end

-- ── Editor .env do projeto (mesmo fluxo do serviço) ─────────────────────

function penv_text_toggle(): ()
    ctx.penv_text_open = (ctx.penv_text_open == "true") and "false" or "true"
end

function penv_export(): ()
    -- Abre o editor .env já preenchido (o texto vem do snapshot em proj_env_text).
    ctx.penv_text_open = "true"
end

function penv_text_cancel(): ()
    ctx.proj_env_text = ctx.proj_env_text_orig or ""
    ctx.penv_text_open = "false"
end

function penv_import(): ()
    local vars, comments = parse_project_dotenv(ctx.proj_env_text)
    with_project_env(function(pe: any): ()
        pe.env_vars = vars
        pe.env_comments = comments
    end)
    ctx.penv_text_open = "false"
end

-- ── Ações de serviço a partir de um card do grid do projeto ─────────────
-- O grid se atualiza no próximo snapshot; refresh_now traz isso pra hora.

function svc_stop_id(id: string): ()
    local ok_confirm = confirm{
        title = "Parar serviço?",
        message = "O tráfego para este serviço será interrompido até um novo deploy.",
        confirm_label = "Parar",
        destructive = true,
    }

    if not ok_confirm then
        return
    end

    local api = State.api
    if not api then
        return
    end
    ctx.proj_action_msg = "enviando…"
    local ok, res = api:rpc_checked{ ServiceStop = { service_id = id } }
    ctx.proj_action_msg = ok and "serviço parado" or ("erro: " .. res)
    if not ok then
        toast{ message = tostring(res), kind = "error" }
    end
    Stream.refresh_now()
end


function stop_delete_service(id: string): ()
    -- Jobs de Schedules gatilhados por este serviço são removidos em cascata
    -- pelo daemon — avisa quais no confirm (jobs_by_id vem do snapshot).
    local dep_names = {}
    for _, job in State.jobs_by_id do
        if job.trigger_service_id == id then
            table.insert(dep_names, tostring(job.name))
        end
    end
    local message = "O serviço será parado e removido."
    if #dep_names > 0 then
        table.sort(dep_names)
        message = message
            .. " Os jobs de Schedules que dependem dele também serão removidos, com histórico: "
            .. table.concat(dep_names, ", ")
            .. "."
    end
    message = message .. " Essa ação não pode ser desfeita."
    local ok_confirm = confirm{
        title = "Parar e remover serviço?",
        message = message,
        confirm_label = "Remover",
        destructive = true,
    }

    if not ok_confirm then
        return
    end


    local api = State.api
    if not api then
        return
    end
    ctx.proj_action_msg = "removendo…"
    local ok, res = api:rpc_checked{ ServiceStop = { service_id = id } }
    if not ok then
        ctx.proj_action_msg = "erro ao parar: " .. res;
        toast{ message = tostring(res), kind = "error" }; return
    end
    local ok2, res2 = api:rpc_checked{ ServiceDelete = { id = id } }
    ctx.proj_action_msg = ok2 and "serviço parado e removido" or ("erro ao remover: " .. res2)
    if ok2 then
        toast{ message = tostring("Serviço removido"), kind = "success" }
    else
        toast{ message = tostring(res2), kind = "error" }
    end
    Stream.refresh_now()
end

-- ── Stop All (topbar) ────────────────────────────────────────────────────

function stop_all(): ()
    local ok_confirm = confirm{
        title = "Parar todos os serviços?",
        message = "Todos os serviços em execução serão parados agora. Você pode reiniciá-los depois com um novo deploy.",
        confirm_label = "Parar todos",
        destructive = true,
    }

    if not ok_confirm then
        return
    end

    local api = State.api
    if not api then
        return
    end
    ctx.status_line = "parando todos…"
    local ok, res = api:rpc_checked("StopAllManaged")
    if ok then
        local n = res.StopAllResult and res.StopAllResult.count or 0
        ctx.status_line = "parados: " .. tostring(n)
    else
        ctx.status_line = "erro: " .. res
    end
end
