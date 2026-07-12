--!strict
--!nolint FunctionUnused
-- new_job_window.luau — script da JANELA "Novo job", um motor Glacier próprio
-- e ISOLADO do app principal (aberto via open_window por
-- handlers/jobs.luau::open_new_job_window). Recebe a conexão (api_url/
-- api_token) e os catálogos de projetos/serviços JÁ BUSCADOS pela janela
-- principal, semeados via open_window({ data = ... }) — um `fetch` rodado no
-- `init()` é descartado pelo motor (só ações despachadas chegam ao
-- executor), mesma razão de `new_service_window.luau`.
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

local function decode(s: string?): { any }
    local ok, v = pcall(json.decode, s or "[]")
    if ok and type(v) == "table" then
        return v
    end
    return {}
end

function init(): ()
    -- ctx.njob_projects/njob_services já chegam como JSON (semeados via data).
    ctx.njob_projects = ctx.njob_projects or "[]"
    ctx.njob_services_all = ctx.njob_services or "[]"
    ctx.njob_services_filtered = "[]"

    ctx.njob_step = "pick_project"
    ctx.njob_project_id = ""
    ctx.njob_project_name = ""
    ctx.njob_service_id = ""
    ctx.njob_service_name = ""
    ctx.njob_name = ""
    ctx.njob_compose = ""
    ctx.njob_main_service = ""
    ctx.njob_kind = "manual"
    ctx.njob_hours = "6"
    ctx.njob_hour = "3"
    ctx.njob_minute = "0"
    ctx.njob_weekday = "0"
    ctx.njob_err = ""
    ctx.njob_msg = ""
end

-- onChange dos inputs (mesma convenção `field:<chave>` do app principal).
function field(key: string?, v: string?): ()
    if key then
        ctx[key] = v or ""
    end
end

function njob_pick_project(id: string): ()
    ctx.njob_project_id = id
    local name = ""
    for _, p in ipairs(decode(ctx.njob_projects)) do
        if p.id == id then
            name = p.name
        end
    end
    ctx.njob_project_name = name

    local filtered = {}
    for _, s in ipairs(decode(ctx.njob_services_all)) do
        if s.project_id == id then
            table.insert(filtered, s)
        end
    end
    ctx.njob_services_filtered = json.encode(filtered)
    ctx.njob_step = "pick_service"
end

function njob_pick_service(id: string): ()
    ctx.njob_service_id = id
    local name = ""
    for _, s in ipairs(decode(ctx.njob_services_filtered)) do
        if s.id == id then
            name = s.name
        end
    end
    ctx.njob_service_name = name
    ctx.njob_step = "form"
end

function njob_back(): ()
    if ctx.njob_step == "form" then
        ctx.njob_step = "pick_service"
    elseif ctx.njob_step == "pick_service" then
        ctx.njob_step = "pick_project"
    end
end

function njob_kind(k: string): ()
    ctx.njob_kind = k
end

function njob_weekday(w: string): ()
    ctx.njob_weekday = w
end

function njob_create(): ()
    local api = client()
    if not api then
        ctx.njob_err = "sem conexão — reabra a janela pelo app"
        return
    end
    local name = Helpers.trim(ctx.njob_name)
    if name == "" then
        ctx.njob_err = "informe um nome"
        return
    end
    local compose = ctx.njob_compose or ""
    if Helpers.trim(compose) == "" then
        ctx.njob_err = "cole o docker-compose.yml do job"
        return
    end
    local main_service = Helpers.trim(ctx.njob_main_service)
    if main_service == "" then
        ctx.njob_err = "informe o main_service (nome do serviço dentro do compose)"
        return
    end
    ctx.njob_err = ""

    local recurrence: any = nil
    if ctx.njob_kind == "interval" then
        recurrence = { IntervalHours = math.max(1, tonumber(ctx.njob_hours) or 1) }
    elseif ctx.njob_kind == "daily" then
        recurrence = {
            Daily = {
                hour = tonumber(ctx.njob_hour) or 0,
                minute = tonumber(ctx.njob_minute) or 0,
            },
        }
    elseif ctx.njob_kind == "weekly" then
        recurrence = {
            Weekly = {
                weekday = tonumber(ctx.njob_weekday) or 0,
                hour = tonumber(ctx.njob_hour) or 0,
                minute = tonumber(ctx.njob_minute) or 0,
            },
        }
    end

    ctx.njob_msg = "criando…"
    local ok, res = api:rpc_checked({
        JobCreate = {
            project_id = ctx.njob_project_id,
            trigger_service_id = ctx.njob_service_id,
            name = name,
            compose = compose,
            main_service = main_service,
            recurrence = recurrence,
        },
    })
    if not ok then
        ctx.njob_msg = ""
        ctx.njob_err = "erro: " .. tostring(res)
        return
    end

    local j = (type(res) == "table" and res.Job) or {}
    broadcast("job_created", { id = j.id, name = j.name or name })
    close_window()
end

function cancel(): ()
    close_window()
end
