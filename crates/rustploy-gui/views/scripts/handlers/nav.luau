--!strict
--!nolint FunctionUnused
-- handlers/nav.luau — navegação da sidebar/tabs e busca do topbar.

local State = require("../state")
local Stream = require("stream")
local Registry = require("registry")

local function goto_view(v: string): ()
    ctx.view = v
    ctx.selected_service = ""
end

function nav_deployments(): ()
    goto_view("deployments")
end

function nav_deploy_engine(): ()
    goto_view("deploy_engine")
end

function nav_monitoring(): ()
    goto_view("monitoring")
end

function nav_schedules(): ()
    goto_view("schedules")
end

function nav_ingress(): ()
    goto_view("ingress")
end

function nav_docker(): ()
    goto_view("docker")
end

function nav_projects(): ()
    ctx.selected_project_id = ""
    goto_view("projects")
end

function nav_settings(): ()
    goto_view("settings")
    ctx.settings_tab = "web"
    -- Defaults do form de conexão Git (destaca a aba certa e o método OAuth).
    if (ctx.gp_kind or "") == "" then
        ctx.gp_kind = "gitea"
    end
    if (ctx.gp_mode or "") == "" then
        ctx.gp_mode = "oauth"
    end
end

function nav_support(): ()
    goto_view("support")
end


-- Genérico p/ NavItem que usa action="nav" com value=target (se algum usar).
function nav(v: string?): () 
    if v then
        goto_view(v)
    end
end


-- Abas (TabButton dispara `proj_tab:services`, `docker_tab:images`, …).
function proj_tab(t: string): ()
    ctx.proj_tab = t
end

function docker_tab(t: string): ()
    ctx.docker_tab = t
    if t == "registry" then
        Registry.refresh_tokens()
    end
end

function deploy_tab(t: string): ()
    ctx.deploy_tab = t
end


-- Abas do service detail e do healthcheck kind.
function tab(t: string): ()
    ctx.tab = t
end

function prov(t: string): ()
    ctx.prov_tab = t
end

function hckind(k: string): ()
    ctx.f_hc_kind = k
end


-- Setter genérico de campo (on_change="field:<chave>").
function field(key: string?, v: string?): ()
    if key then
        ctx[key] = v or ""
    end
end


function search_changed(v: string): ()
    ctx.search = v or ""
    State.search_term = string.lower(v or "")
    Stream.rebuild_lists()
    Stream.update_open_project()
end
