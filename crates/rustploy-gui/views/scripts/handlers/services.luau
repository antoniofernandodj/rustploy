--!strict
--!nolint FunctionUnused
-- handlers/services.luau — detalhe do serviço (service.xml): fetch completo,
-- mutações de spec/env, ciclo de vida (deploy/reload/stop), build logs de
-- deployments, picker Gitea. Exporta `fetch_service_detail` para
-- handlers/stream.luau reusar no desfecho de um deploy (evento bus
-- DeployStateChanged).

local State = require("../state")
local Helpers = require("../helpers")
local fmt = require("../fmt")

local M = {}

-- ── Fetch completo do painel de detalhe ─────────────────────────────────────

-- Abrir o detalhe de um serviço: navega e dispara o fetch (spec + logs +
-- deployments + provedores Git). O fetch suspende a corrotina, sem travar a
-- UI; svc_loading=true segura os Selects até tudo chegar.
function open_service(id: string): ()
    ctx.selected_service = id
    ctx.view = "service"
    ctx.tab = "general"
    ctx.svc_error = ""
    ctx.dep_selected = ""
    ctx.svc_action_msg = ""
    ctx.svc_action_color = "#8B949E"
    if State.deploy_track.service_id ~= id then
        ctx.svc_deploy_running = "false"
        ctx.svc_deploy_elapsed = ""
    end
    ctx.svc_loading = "true"
    M.fetch_service_detail(id)
end

-- One-shot: monta TODAS as chaves svc_*/f_* do painel de detalhe (equivalente
-- a net/services.rs::fetch_detail_inner).
function M.fetch_service_detail(sid: string): ()
    local api = State.api
    if not api or sid == "" then return end
    local ok, res = api:rpc({ ServiceGet = { id = sid } })
    if not ok or not res.Service then
        ctx.svc_loading = "false"
        ctx.svc_error = ok and "serviço não encontrado" or res
        return
    end
    local svc = res.Service
    local spec = svc.spec

    -- Nome + env vars do projeto (ServiceGet só traz o id). As env vars do
    -- projeto são a base herdada por todos os serviços; usamos elas junto do
    -- serviço para montar a URL externa (ver merged_env abaixo).
    local project_name = spec.project_id
    local project_env = {}
    local snap = State.snap
    if snap then
        for _, p in ipairs(snap.projects or {}) do
            if p.id == spec.project_id then
                project_name = p.name
                project_env = p.env_vars or {}
                break
            end
        end
    end

    -- Runtime logs NÃO são carregados aqui: vivem só na janela isolada de logs
    -- (open_logs_window, que busca o próprio tail). A janela principal não os
    -- renderiza nem os transmite, para não pagar reavaliação por linha.
    local deployments = {}
    local okd, resd = api:rpc({ DeployHistory = { service_id = sid, limit = 30 } })
    if okd and resd.Deployments then deployments = resd.Deployments end

    local status_label, status_color = fmt.status_label_color(svc.status)
    local skind, sdetail, sbuild = fmt.source_summary3(spec.source)

    -- Campos editáveis da aba General (source).
    local g = {
        repo_url = "",
        branch = "",
        username = "",
        credentials = "",
        build_path = "",
        watch_paths = "",
        submodules = false,
        dockerfile = "",
        context_path = "",
        build_stage = ""
    }
    local compose_content = ""
    local prov_tab, gen_provider_id, bound_url = "git", "", ""
    if spec.source.Git then
        local gs = spec.source.Git
        g.repo_url = gs.url or ""
        g.branch = gs.branch or ""
        g.username = gs.username or ""
        g.credentials = gs.credentials or ""
        g.build_path = gs.root_path or ""
        g.watch_paths = table.concat(gs.watch_paths or {}, ", ")
        g.submodules = gs.submodules == true
        g.dockerfile = gs.dockerfile_path or ""
        g.context_path = gs.build_context or ""
        g.build_stage = gs.build_stage or ""
        if gs.provider_id and gs.provider_id ~= "" then
            prov_tab, gen_provider_id, bound_url = "gitea", gs.provider_id, gs.url or ""
        end
    elseif spec.source.Registry then
        g.repo_url = spec.source.Registry.image or ""
    elseif spec.source.Compose then
        compose_content = spec.source.Compose.content or ""
    end

    -- Healthcheck → campos do form.
    local hc = spec.healthcheck
    local hk, hpath, hstatus = "tcp", "", ""
    if type(hc.kind) == "table" and hc.kind.Http then
        hk, hpath, hstatus = "http", hc.kind.Http.path or "", tostring(hc.kind.Http.expected_status)
    elseif hc.kind == "None" then
        hk = "none"
    elseif hc.kind == "DockerNative" then
        hk = "docker"
    else
        hk = "tcp"
    end

    -- Rota primária (Connection) + provedores Gitea conectados.
    local routes = {}
    if spec.domains and #spec.domains > 0 then
        for _, r in ipairs(spec.domains) do table.insert(routes, r) end
    elseif spec.domain and spec.domain ~= "" then
        routes = { { domain = spec.domain, tls = spec.tls_enabled == true } }
    end
    local pr = routes[1]
    local pr_domain = pr and pr.domain or nil
    local pr_tls = pr and pr.tls == true or false

    ctx.svc_loading = "false"
    ctx.svc_error = ""
    ctx.svc_id = svc.id
    ctx.svc_name = spec.name
    ctx.svc_project = project_name
    ctx.svc_project_id = spec.project_id
    ctx.svc_status_label = status_label
    ctx.svc_status_color = status_color
    -- Containers reais em execução: o snapshot (State.snap) anexa a lista a cada
    -- serviço; buscamos pelo id (ServiceGet não carrega essa informação).
    local svc_containers = {}
    if snap then
        for _, e in ipairs(snap.services or {}) do
            if e.service and e.service.id == sid then
                svc_containers = e.service.containers or {}
                break
            end
        end
    end
    ctx.svc_containers = fmt.containers(svc_containers)
    ctx.svc_containers_count = tostring(#svc_containers)
    ctx.svc_source_kind = skind
    ctx.svc_source_detail = sdetail
    ctx.svc_compose = compose_content
    ctx.svc_compose_orig = compose_content
    ctx.svc_build = sbuild
    ctx.svc_port = tostring(spec.port)
    ctx.svc_host_port = spec.host_port and tostring(spec.host_port) or "—"
    ctx.svc_domain = pr_domain or "—"
    ctx.svc_tls = pr_tls and "enabled" or "disabled"
    ctx.svc_replicas = tostring(spec.replicas)
    ctx.svc_domains = fmt.domains_json(spec)
    ctx.svc_domains_count = tostring(#routes)
    -- Credenciais para a URL externa: mesma fusão que o daemon faz em
    -- resolve_env (projeto = base, serviço sobrescreve). Serviços Compose de
    -- banco têm spec.env_vars vazio — os ${POSTGRES_*} vêm do projeto. Como
    -- env_plain retorna o primeiro match por chave, o serviço vem primeiro.
    local merged_env = {}
    for _, v in ipairs(spec.env_vars or {}) do table.insert(merged_env, v) end
    for _, v in ipairs(project_env) do table.insert(merged_env, v) end
    -- Valor completo (`*_url`) alimenta o clipboard; `*_url_short` é o que a
    -- markup exibe, truncado para não empurrar o botão "Copiar" para fora.
    ctx.svc_external_url = fmt.external_url(pr_domain, pr_tls, spec.host_port, spec.db_kind, ctx.api_url, merged_env)
    ctx.svc_external_url_short = fmt.ellipsis(ctx.svc_external_url, 20)
    ctx.svc_internal_url = fmt.internal_url(spec.db_kind, fmt.safe_name(spec.name), spec.port)
    ctx.svc_internal_url_short = fmt.ellipsis(ctx.svc_internal_url, 20)
    ctx.svc_db_kind = spec.db_kind or "—"
    ctx.svc_hc = fmt.healthcheck_summary(spec.healthcheck)
    ctx.svc_run_command = spec.run_command or "—"
    ctx.svc_run_args = (spec.run_args and #spec.run_args > 0) and table.concat(spec.run_args, " ") or "—"
    ctx.svc_env = fmt.env_json_with_comments(spec.env_vars, spec.env_comments)
    ctx.svc_env_count = tostring(#(spec.env_vars or {}))
    local dotenv = fmt.env_dotenv_with_comments(spec.env_vars, spec.env_comments)
    ctx.svc_env_text = dotenv
    ctx.svc_env_text_orig = dotenv
    ctx.svc_deployments = fmt.deployments_detail(deployments)
    ctx.svc_deployments_count = tostring(#deployments)
    -- Webhook de deploy (aba Deployments). Só serviços Application (Git ou
    -- Registry) têm token, e ele nasce no PRIMEIRO deploy — antes disso o
    -- daemon devolve WebhookUrl=null e a UI mostra o aviso em vez da URL.
    ctx.svc_webhook_supported = spec.source.Compose and "false" or "true"
    ctx.svc_webhook_url = ""
    if not spec.source.Compose then
        local okw, resw = api:rpc({ GetWebhookUrl = { service_id = sid } })
        if okw and type(resw) == "table" and resw.WebhookUrl then
            ctx.svc_webhook_url = resw.WebhookUrl
        end
    end
    -- Campos editáveis.
    ctx.f_domain = ""
    ctx.f_port = ""
    ctx.f_tls = "false"
    ctx.f_host_port = spec.host_port and tostring(spec.host_port) or ""
    ctx.f_hc_kind = hk
    ctx.f_hc_path = hpath
    ctx.f_hc_status = hstatus
    ctx.f_hc_interval = tostring(hc.interval_secs)
    ctx.f_hc_timeout = tostring(hc.timeout_secs)
    ctx.f_hc_retries = tostring(hc.retries)
    ctx.f_hc_start = tostring(hc.start_period_secs)
    ctx.f_replicas = tostring(spec.replicas)
    ctx.f_run_command = spec.run_command or ""
    ctx.f_repo_url = g.repo_url
    ctx.f_branch = g.branch
    ctx.f_username = g.username
    ctx.f_credentials = g.credentials
    ctx.f_build_path = g.build_path
    ctx.f_watch_paths = g.watch_paths
    ctx.f_submodules = g.submodules and "true" or "false"
    ctx.f_dockerfile = g.dockerfile
    ctx.f_context_path = g.context_path
    ctx.f_build_stage = g.build_stage
    ctx.f_gen_port = tostring(spec.port)
    ctx.prov_tab = prov_tab
    ctx.gitea_provider_id = gen_provider_id

    -- Provedores Gitea conectados (para o picker de conta).
    local okp, resp = api:rpc("GitProviderList")
    if okp and resp.GitProviders then
        ctx.gitea_providers = fmt.git_providers(resp.GitProviders)
        ctx.gitea_count = tostring(#resp.GitProviders)
        -- Serviço já ligado a um provedor: pré-carrega repos/branches.
        if gen_provider_id ~= "" then
            local okr, resr = api:rpc({ GitRepoList = { provider_id = gen_provider_id } })
            if okr and resr.GitRepos then
                ctx.gitea_repos = fmt.git_repos(resr.GitRepos)
                ctx.gitea_msg = tostring(#resr.GitRepos) .. " repositório(s)"
                local full
                for _, r in ipairs(resr.GitRepos) do
                    if r.clone_url == bound_url then
                        full = r.full_name;
                        break
                    end
                end
                if full then
                    ctx.gitea_repo = full
                    local okb, resb = api:rpc({
                        GitBranchList = {
                            provider_id = gen_provider_id,
                            repo_full_name = full
                        }
                    })
                    if okb and resb.GitBranches then
                        ctx.gitea_branches = fmt.git_branches(resb.GitBranches)
                    end
                end
            end
        end
    end
end

-- ── Mutações de spec/env (ServiceGet → muta → ServiceUpdate) ────────────────

-- Busca o spec fresco, aplica `mutate(spec)`, envia ServiceUpdate e refaz o
-- fetch do detalhe. As validações (campo vazio etc.) vivem nos handlers, antes
-- de chamar with_spec; os closures só mutam o spec em lugar, sem retornar erro.
local function with_spec(mutate: (spec: any) -> ()): ()
    local sid = ctx.selected_service
    local api = State.api
    if not api or sid == "" then
        return
    end
    local ok, res = api:rpc({ ServiceGet = { id = sid } })
    if not ok or not res.Service then
        ctx.svc_action_msg = "erro ao carregar spec"
        Helpers.notify_err("erro ao carregar o serviço")
        return
    end
    local spec = res.Service.spec
    mutate(spec)
    local ok2, res2 = api:rpc_checked({ ServiceUpdate = { id = sid, spec = spec } })
    if ok2 then
        ctx.svc_action_msg = "salvo"
        M.fetch_service_detail(sid)
    else
        ctx.svc_action_msg = "erro: " .. res2
        Helpers.notify_err(res2)
    end
end

-- Move o domínio legado (domain/tls_enabled) para a lista `domains`.
local function materialize_domains(spec: any): ()
    if not spec.domains or #spec.domains == 0 then
        local routes = json.array({})
        if spec.domain and spec.domain ~= "" then
            table.insert(routes, { domain = spec.domain, tls = spec.tls_enabled == true })
        end
        spec.domains = routes
    end
    spec.domain = nil
    spec.tls_enabled = false
end

function dom_add(): ()
    local domain = Helpers.trim(ctx.f_domain)
    if domain == "" then ctx.erro_f_domain = "domínio vazio"; return end
    ctx.erro_f_domain = ""
    with_spec(function(spec: any): ()
        materialize_domains(spec)
        local route = { domain = domain, port = tonumber(Helpers.trim(ctx.f_port)), tls = ctx.f_tls == "true" }
        local found = false
        for i, r in ipairs(spec.domains) do
            if r.domain == domain then spec.domains[i] = route; found = true; break end
        end
        if not found then table.insert(spec.domains, route) end
    end)
end

function dom_del(domain: string): ()
    with_spec(function(spec: any): ()
        materialize_domains(spec)
        local kept = json.array({})
        for _, r in ipairs(spec.domains) do
            if r.domain ~= domain then table.insert(kept, r) end
        end
        spec.domains = kept
    end)
end

function dom_hostport_save(): ()
    with_spec(function(spec: any): ()
        spec.host_port = tonumber(Helpers.trim(ctx.f_host_port))
    end)
end

-- Sentinela 0 = "aloque para mim": o daemon troca por uma porta livre da faixa
-- [external_ports] e a libera no firewall (rustployd-fw). O fetch pós-update do
-- with_spec já re-renderiza svc_host_port/svc_external_url com a porta real.
function dom_hostport_auto(): ()
    with_spec(function(spec: any): ()
        spec.host_port = 0
    end)
end

function compose_save(): ()
    with_spec(function(spec: any): ()
        if spec.source.Compose then
            spec.source = { Compose = { content = ctx.svc_compose or "" } }
        end
    end)
end

function compose_cancel(): ()
    ctx.svc_compose = ctx.svc_compose_orig or ""
end

function hc_save(): ()
    with_spec(function(spec: any): ()
        local kind = ctx.f_hc_kind
        local hk: any
        if kind == "none" then
            hk = "None"
        elseif kind == "http" then
            local path = Helpers.trim(ctx.f_hc_path); if path == "" then path = "/" end
            hk = {
                Http = {
                    path = path,
                    expected_status = tonumber(Helpers.trim(ctx.f_hc_status)) or 200,
                },
            }
        elseif kind == "docker" then
            hk = "DockerNative"
        else
            hk = "Tcp"
        end
        local cur = spec.healthcheck
        spec.healthcheck = {
            kind = hk,
            interval_secs = tonumber(Helpers.trim(ctx.f_hc_interval)) or cur.interval_secs,
            timeout_secs = tonumber(Helpers.trim(ctx.f_hc_timeout)) or cur.timeout_secs,
            retries = tonumber(Helpers.trim(ctx.f_hc_retries)) or cur.retries,
            start_period_secs = tonumber(Helpers.trim(ctx.f_hc_start)) or cur.start_period_secs,
        }
    end)
end

function adv_save(): ()
    with_spec(function(spec: any): ()
        local r = tonumber(Helpers.trim(ctx.f_replicas)) or 1
        if r < 1 then r = 1 end
        spec.replicas = math.floor(r)
        local rc = Helpers.trim(ctx.f_run_command)
        spec.run_command = (rc ~= "") and rc or nil
    end)
end

function gen_save(): ()
    with_spec(function(spec: any): ()
        local port = tonumber(Helpers.trim(ctx.f_gen_port))
        if port then spec.port = math.floor(port) end
        local function ne(v: any, d: string): string
            local t = Helpers.trim(v)
            if t == "" then return d else return t end
        end
        local function opt(v: any): string?
            local t = Helpers.trim(v)
            if t == "" then return nil else return t end
        end
        local repo = Helpers.trim(ctx.f_repo_url)
        local watch = json.array({})
        for part in string.gmatch(ctx.f_watch_paths or "", "[^,]+") do
            local t = Helpers.trim(part); if t ~= "" then table.insert(watch, t) end
        end
        local git = { Git = {
            url = repo, branch = ne(ctx.f_branch, "main"), root_path = ne(ctx.f_build_path, "."),
            watch_paths = watch, submodules = ctx.f_submodules == "true",
            dockerfile_path = ne(ctx.f_dockerfile, "Dockerfile"), build_context = ne(ctx.f_context_path, "."),
            build_stage = opt(ctx.f_build_stage), credentials = opt(ctx.f_credentials),
            username = opt(ctx.f_username), provider_id = opt(ctx.gitea_provider_id),
        } }
        if spec.source.Compose then
            -- inalterado
        elseif spec.source.Git or Helpers.looks_like_git_url(repo) then
            spec.source = git
        else
            spec.source = { Registry = { image = repo } }
        end
    end)
end

-- ── Environment ──────────────────────────────────────────────────────────

-- Parseia um blob .env em (vars, comments). Comentários ancoram na var
-- seguinte (before_key), ou nil no fim. `<secret:NAME>` volta a ser referência
-- de secret.
local function parse_dotenv(text: string?): ({any}, {any})
    local vars = json.array({})
    local comments = json.array({})
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

-- Aplica uma ordem de keys (vars + linhas `__c<idx>`) a vars/comments: vars
-- reordenadas; cada comentário reancora na primeira var real que o segue.
local function reorder_env(vars: {any}, comments: {any}, keys: {any}): ({any}, {any})
    local var_keys = {}
    local by_key = {}
    for _, v in ipairs(vars) do var_keys[v.key] = true; by_key[v.key] = v end
    local seen = {}
    local new_vars = json.array({})
    local new_comments = json.array({})
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

function env_add(): ()
    local key = Helpers.trim(ctx.env_new_key)
    if key == "" then ctx.erro_env_new_key = "chave vazia"; return end
    ctx.erro_env_new_key = ""
    local val = ctx.env_new_val or ""
    with_spec(function(spec: any): ()
        local kept = json.array({})
        for _, v in ipairs(spec.env_vars or {}) do
            if v.key ~= key then table.insert(kept, v) end
        end
        table.insert(kept, { key = key, value = { Plain = val } })
        spec.env_vars = kept
    end)
    ctx.env_new_key = ""
    ctx.env_new_val = ""
end

function env_del(key: string): ()
    with_spec(function(spec: any): ()
        local kept = json.array({})
        for _, v in ipairs(spec.env_vars or {}) do
            if v.key ~= key then
                table.insert(kept, v)
            end
        end
        spec.env_vars = kept
        local kc = json.array({})
        for _, c in ipairs(spec.env_comments or {}) do
            if c.before_key ~= key then
                table.insert(kc, c)
            end
        end
        spec.env_comments = kc
    end)
end

function env_import(): ()
    local text = ctx.svc_env_text or ""
    with_spec(function(spec: any): ()
        local vars, comments = parse_dotenv(text)
        spec.env_vars = vars
        spec.env_comments = comments
    end)
    ctx.env_text_open = "false"
end

function env_export(): ()
    -- Abre o editor .env já preenchido (o texto vem do fetch em svc_env_text).
    ctx.env_text_open = "true"
end

function env_text_toggle(): ()
    ctx.env_text_open = (ctx.env_text_open == "true") and "false" or "true"
end

function env_text_cancel(): ()
    ctx.svc_env_text = ctx.svc_env_text_orig or ""
    ctx.env_text_open = "false"
end

-- Reordena env (arraste): `value` é um JSON array de keys na nova ordem.
function env_reorder(v: string?): ()
    if not v then return end
    local ok, keys = pcall(json.decode, v)
    if not ok or type(keys) ~= "table" then
        return
    end
    with_spec(function(spec: any): ()
        local nv, nc = reorder_env(spec.env_vars or {}, spec.env_comments or {}, keys)
        spec.env_vars = nv
        spec.env_comments = nc
    end)
end

-- ── Deployments (build log + delete) ────────────────────────────────────────

-- Abre os build logs de um deployment numa JANELA à parte (mesma janela genérica
-- dos runtime logs — ver open_logs_window). Busca o histórico na hora via
-- GetBuildLogs (ação despachada → fetch funciona) e semeia; a janela então segue
-- ao vivo pelo endpoint dedicado /api/deployments/<id>/build-logs. Isola o custo
-- de rolar milhares de linhas de build fora da árvore da janela principal.
function dep_logs(id: string): ()
    if not id or id == "" then
        return
    end
    local seed = "[]"
    local api = State.api
    if api then
        local ok, res = api:rpc({ GetBuildLogs = { deployment_id = id } })
        if ok and res.BuildLogs then
            seed = json.encode(res.BuildLogs)
        end
    end
    open_window({
        file = "crates/rustploy-gui/views/log_window.xml",
        title = "Build — " .. string.sub(id, 1, 12),
        width = 900,
        height = 560,
        resizable = true,
        data = {
            api_url = ctx.api_url or "",
            api_token = ctx.api_token or "",
            lw_title = "Build logs · " .. string.sub(id, 1, 12),
            lw_stream_url = "/api/deployments/" .. id .. "/build-logs",
            lw_seed = seed,
        },
    })
end

function delete_deployment(id: string): ()
    confirm({
        title = "Remover deployment?",
        message = "O registro do deployment e seus build logs serão removidos. Essa ação não pode ser desfeita.",
        confirm_label = "Remover",
        confirm_action = "do_delete_deployment:" .. tostring(id),
        destructive = true,
    })
end

function do_delete_deployment(id: string): ()
    local api = State.api
    if not api then
        return
    end
    local ok, res = api:rpc_checked({ DeployDelete = { deployment_id = id } })
    ctx.svc_action_msg = ok and "deployment removido" or ("erro: " .. res)
    if not ok then
        Helpers.notify_err(res)
    end
    M.fetch_service_detail(ctx.selected_service)
end

-- ── Webhook de deploy ───────────────────────────────────────────────────────

-- Regenerar invalida o token antigo NA HORA: qualquer webhook já cadastrado no
-- GitHub/Gitea/Docker Hub com a URL anterior passa a receber 401. Por isso passa
-- pelo confirm.
function regen_webhook(): ()
    confirm({
        title = "Regenerar token do webhook?",
        message = "A URL atual para de funcionar imediatamente. Você precisará recadastrá-la no provedor (GitHub, Gitea, Docker Hub…).",
        confirm_label = "Regenerar",
        confirm_action = "do_regen_webhook",
        destructive = true,
    })
end

function do_regen_webhook(): ()
    local api = State.api
    local sid = ctx.selected_service
    if not api or sid == "" then
        return
    end
    local ok, res = api:rpc_checked({ RegenerateWebhookToken = { service_id = sid } })
    if ok and type(res) == "table" and res.WebhookUrl then
        ctx.svc_webhook_url = res.WebhookUrl
        ctx.svc_action_msg = "token do webhook regenerado"
        Helpers.notify_ok("Token do webhook regenerado")
    else
        local msg = type(res) == "string" and res or "falha ao regenerar o token"
        ctx.svc_action_msg = "erro: " .. msg
        Helpers.notify_err(msg)
    end
end

-- ── Ciclo de vida do serviço ─────────────────────────────────────────────────

local function svc_lifecycle(cmd: any, msg: string): ()
    local sid = ctx.selected_service
    local api = State.api
    if not api or sid == "" then
        return
    end
    ctx.svc_action_msg = msg
    local ok, res = api:rpc_checked(cmd)
    if not ok then
        ctx.svc_action_msg = "erro: " .. res
    end
    M.fetch_service_detail(sid)
end

-- Deploy de um serviço (a partir de um card / detalhe). Arma deploy_track para
-- o timer decorrido (atualizado pelo snapshot de 2s, via engine.active) e o
-- desfecho (via evento bus DeployStateChanged, ver handlers/stream.luau).
local function start_deploy(id: string?): ()
    local sid = (id and id ~= "") and id or ctx.selected_service
    local api = State.api
    if not api or not sid or sid == "" then
        return
    end
    ctx.svc_action_msg = "iniciando deploy…"
    ctx.svc_action_color = "#58A6FF"
    local ok, res = api:rpc_checked({ DeployStart = { service_id = sid } })
    if ok and res.Deployment then
        State.deploy_track = { service_id = sid, deployment_id = res.Deployment.id, running = true }
        ctx.svc_deploy_running = "true"
        ctx.svc_deploy_elapsed = "0s"
        ctx.svc_action_msg = "deploy iniciado · " .. tostring(res.Deployment.state)
        Helpers.notify_ok("Deploy iniciado")
    else
        ctx.svc_action_msg = ok and "resposta inesperada" or ("erro: " .. res)
        ctx.svc_action_color = "#F85149"
        Helpers.notify_err(ok and "resposta inesperada do daemon" or res)
    end
    if ctx.selected_service == sid then
        M.fetch_service_detail(sid)
    end
end

function svc_deploy(): ()
    start_deploy(ctx.selected_service)
end

function svc_rebuild(): ()
    start_deploy(ctx.selected_service)
end

function svc_reload(): ()
    svc_lifecycle({
        ServiceReload = {
            service_id = ctx.selected_service
        }
    }, "recarregando…")
end

function svc_stop(): ()
    confirm({
        title = "Parar serviço?",
        message = "O tráfego para este serviço será interrompido até um novo deploy.",
        confirm_label = "Parar",
        confirm_action = "do_svc_stop",
        destructive = true,
    })
end

function do_svc_stop(): ()
    svc_lifecycle({
        ServiceStop = {
            service_id = ctx.selected_service
        }
    }, "parando…")
end

-- ── Picker Gitea (conta → repos → branches) ─────────────────────────────────

function gitea_provider_pick(id: string): ()
    ctx.gitea_provider_id = id or ""
    ctx.gitea_repo = ""
    ctx.gitea_repos = "[]"
    ctx.gitea_branches = "[]"
    local api = State.api
    if not api or id == "" then
        return
    end
    ctx.gitea_msg = "carregando repositórios…"
    local ok, res = api:rpc({ GitRepoList = { provider_id = id } })
    if ok and res.GitRepos then
        ctx.gitea_repos = fmt.git_repos(res.GitRepos)
        ctx.gitea_msg = tostring(#res.GitRepos) .. " repositório(s)"
    else
        ctx.gitea_msg = "erro ao listar repositórios"
    end
end

function gitea_repo_pick(full_name: string): ()
    if not full_name then
        return
    end
    ctx.gitea_repo = full_name
    -- Resolve clone_url/branch a partir da lista já carregada.
    local ok, repos = pcall(json.decode, ctx.gitea_repos or "[]")
    if ok and type(repos) == "table" then
        for _, r in ipairs(repos) do
            if r.full_name == full_name then
                if r.clone_url and r.clone_url ~= "" then
                    ctx.f_repo_url = r.clone_url
                end
                if r.default_branch and r.default_branch ~= "" then
                    ctx.f_branch = r.default_branch
                end
                break
            end
        end
    end
    ctx.gitea_branches = "[]"
    local pid = ctx.gitea_provider_id or ""
    local api = State.api
    if not api or pid == "" then
        return
    end
    ctx.gitea_msg = "carregando branches…"
    local okb, resb = api:rpc({ GitBranchList = { provider_id = pid, repo_full_name = full_name } })
    if okb and resb.GitBranches then
        ctx.gitea_branches = fmt.git_branches(resb.GitBranches)
    end
end

-- Abre os logs ao vivo do serviço numa JANELA à parte (motor Glacier isolado e
-- leve — ver log_window.xml/.luau). Semeia a conexão, o serviço-alvo e um tail
-- FRESCO via open_window({data}); a janela abre sua própria SSE (endpoint
-- dedicado) e rola os logs sem o custo de reavaliar a árvore inteira do app
-- principal. Sendo uma ação despachada, o `fetch` do LogsGet aqui funciona (ao
-- contrário do init de uma janela) — pegamos o tail na hora para não haver vão
-- entre o que a principal tinha e o que a stream ao vivo passa a entregar.
function open_logs_window(): ()
    local sid = ctx.selected_service or ""
    if sid == "" then
        return
    end
    local seed = "[]"
    local api = State.api
    if api then
        local ok, res = api:rpc({ LogsGet = { service_id = sid, tail = 500 } })
        if ok and res.Logs then
            seed = json.encode(res.Logs)
        end
    end
    open_window({
        file = "crates/rustploy-gui/views/log_window.xml",
        title = "Logs — " .. (ctx.svc_name or sid),
        width = 900,
        height = 560,
        resizable = true,
        data = {
            api_url = ctx.api_url or "",
            api_token = ctx.api_token or "",
            lw_title = "Logs · " .. (ctx.svc_name or sid),
            lw_stream_url = "/api/services/" .. sid .. "/logs",
            lw_seed = seed,
        },
    })
end

return M
