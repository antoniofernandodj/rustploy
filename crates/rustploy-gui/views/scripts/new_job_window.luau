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
-- require resolve relativo a ESTE arquivo (scripts/): "net/api", "helpers" e
-- "fmt" (só os builders de git_providers/git_repos/git_branches, reusados da
-- aba Gitea de serviço — ver handlers/services.luau).

local Api = require("net/api")
local Helpers = require("helpers")
local fmt = require("fmt")

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

    -- Fonte do compose: "compose" (colado, default) | "git" (clona um repo
    -- a cada execução — mesmo picker conta→repo→branch da aba Gitea de
    -- serviço). Ver njob_source/njob_git_provider_pick/njob_git_repo_pick.
    ctx.njob_source = "compose"
    ctx.njob_git_providers = "[]"
    ctx.njob_git_provider_id = ""
    ctx.njob_git_repos = "[]"
    ctx.njob_git_repo = ""
    ctx.njob_git_branches = "[]"
    ctx.njob_git_branch = ""
    ctx.njob_compose_path = "docker-compose.yml"
    ctx.njob_git_msg = ""

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

-- ── Fonte do compose: Compose (colado) x Git (picker conta→repo→branch) ──
-- Idêntico a gitea_provider_pick/gitea_repo_pick de handlers/services.luau,
-- só que dentro desta janela isolada (não pode buscar no init(), mas ações
-- despachadas por clique/on_change chegam ao executor normalmente).

function njob_source(kind: string): ()
    ctx.njob_source = kind
    if kind == "git" and ctx.njob_git_providers == "[]" then
        local api = client()
        if not api then
            return
        end
        ctx.njob_git_msg = "carregando contas…"
        local ok, res = api:rpc("GitProviderList")
        if ok and res.GitProviders then
            ctx.njob_git_providers = fmt.git_providers(res.GitProviders)
            ctx.njob_git_msg = ""
        else
            ctx.njob_git_msg = "erro ao listar contas conectadas"
        end
    end
end

function njob_git_provider_pick(id: string): ()
    ctx.njob_git_provider_id = id or ""
    ctx.njob_git_repo = ""
    ctx.njob_git_repos = "[]"
    ctx.njob_git_branches = "[]"
    local api = client()
    if not api or ctx.njob_git_provider_id == "" then
        return
    end
    ctx.njob_git_msg = "carregando repositórios…"
    local ok, res = api:rpc{ GitRepoList = { provider_id = ctx.njob_git_provider_id } }
    if ok and res.GitRepos then
        ctx.njob_git_repos = fmt.git_repos(res.GitRepos)
        ctx.njob_git_msg = tostring(#res.GitRepos) .. " repositório(s)"
    else
        ctx.njob_git_msg = "erro ao listar repositórios"
    end
end

function njob_git_repo_pick(full_name: string): ()
    if not full_name then
        return
    end
    ctx.njob_git_repo = full_name
    local ok, repos = pcall(json.decode, ctx.njob_git_repos or "[]")
    if ok and type(repos) == "table" then
        for _, r in ipairs(repos) do
            if r.full_name == full_name and r.default_branch and r.default_branch ~= "" then
                ctx.njob_git_branch = r.default_branch
                break
            end
        end
    end
    ctx.njob_git_branches = "[]"
    local pid = ctx.njob_git_provider_id or ""
    local api = client()
    if not api or pid == "" then
        return
    end
    ctx.njob_git_msg = "carregando branches…"
    local okb, resb = api:rpc{ GitBranchList = { provider_id = pid, repo_full_name = full_name } }
    if okb and resb.GitBranches then
        ctx.njob_git_branches = fmt.git_branches(resb.GitBranches)
        ctx.njob_git_msg = ""
    else
        ctx.njob_git_msg = "erro ao listar branches"
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

-- Job 100% autônomo: sem serviço gatilho, só as env vars do projeto.
function njob_pick_no_service(): ()
    ctx.njob_service_id = ""
    ctx.njob_service_name = "nenhum (autônomo)"
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
    local main_service = Helpers.trim(ctx.njob_main_service)
    if main_service == "" then
        ctx.njob_err = "informe o main_service (nome do serviço dentro do compose)"
        return
    end

    local compose = ""
    local git_source: any = nil
    if ctx.njob_source == "git" then
        local branch = Helpers.trim(ctx.njob_git_branch)
        if ctx.njob_git_repo == "" or branch == "" then
            ctx.njob_err = "selecione repositório e branch"
            return
        end
        local compose_path = Helpers.trim(ctx.njob_compose_path)
        if compose_path == "" then
            compose_path = "docker-compose.yml"
        end
        -- URL de clone vem da lista de repos já carregada (mesma resolução
        -- de gitea_repo_pick em handlers/services.luau).
        local url = ""
        local ok, repos = pcall(json.decode, ctx.njob_git_repos or "[]")
        if ok and type(repos) == "table" then
            for _, r in ipairs(repos) do
                if r.full_name == ctx.njob_git_repo then
                    url = r.clone_url or ""
                    break
                end
            end
        end
        if url == "" then
            ctx.njob_err = "não foi possível resolver a URL do repositório — selecione de novo"
            return
        end
        git_source = {
            provider_id = ctx.njob_git_provider_id ~= "" and ctx.njob_git_provider_id or nil,
            url = url,
            branch = branch,
            username = nil,
            credentials = nil,
            compose_path = compose_path,
        }
    else
        compose = ctx.njob_compose or ""
        if Helpers.trim(compose) == "" then
            ctx.njob_err = "cole o docker-compose.yml do job"
            return
        end
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
    local ok, res = api:rpc_checked{
        JobCreate = {
            project_id = ctx.njob_project_id,
            trigger_service_id = ctx.njob_service_id,
            name = name,
            compose = compose,
            git_source = git_source,
            main_service = main_service,
            recurrence = recurrence,
        },
    }
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
