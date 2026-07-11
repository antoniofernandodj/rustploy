--!strict
--!nolint FunctionUnused
-- handlers/stream.luau — consumidor do SSE de /api/events: aplica o snapshot
-- periódico (2s) e os eventos vivos do bus (métricas, logs, deploy). Exporta
-- open_stream/rebuild_lists/update_open_project/refresh_now para os demais
-- handlers (login, busca, abrir projeto, mutações que refletem na hora).

local State = require("../state")
local fmt = require("../fmt")
local Services = require("services")

local M = {}

-- Reconstrói as listas filtradas pela busca a partir do último snapshot — sem
-- rede, para a busca filtrar na hora (equivalente ao antigo search_pairs).
local function rebuild_lists(): ()
    local snap = State.snap
    if not snap then
        return
    end
    local term = State.search_term
    ctx.deployments = fmt.deployments(snap.deployments, term)
    ctx.projects = fmt.projects(snap.projects, State.last_pairs, term)
    ctx.project_rows = fmt.project_rows(snap.projects, State.last_pairs, term)
    ctx.services = fmt.services(State.last_pairs, State.metrics_by_id, term)
    ctx.service_rows = fmt.service_rows(State.last_pairs, State.metrics_by_id, term)
    -- ctx.docker_rows: a antiga sub-aba Containers (só serviços rustploy) foi
    -- substituída pela lista host-wide (docker_containers). fmt.docker_rows fica
    -- disponível para reuso; não recalculamos aqui à toa.
    -- ctx.docker_rows = fmt.docker_rows(State.last_pairs, term)
    ctx.docker_containers = fmt.docker_containers(snap.docker_containers, term)
    ctx.docker_images = fmt.docker_images(snap.docker_images, term)
    ctx.docker_volumes = fmt.docker_volumes(snap.docker_volumes, term)
    ctx.docker_networks = fmt.docker_networks(snap.docker_networks, term)
end
M.rebuild_lists = rebuild_lists

-- Cabeçalho + grade de serviços do projeto aberto (view=project_services).
local function update_open_project(): ()
    local snap = State.snap
    if not snap then
        return
    end
    local pid = ctx.selected_project_id or ""
    if pid == "" then
        return
    end
    local proj
    for _, p in ipairs(snap.projects or {}) do
        if p.id == pid then
            proj = p;
            break
        end
    end
    if not proj then
        return
    end
    local proj_pairs = {}
    for _, e in ipairs(State.last_pairs) do
        if e.svc.spec.project_id == pid then
            table.insert(proj_pairs, e)
        end
    end
    ctx.proj_name = proj.name
    ctx.proj_description = proj.description or ""
    ctx.proj_can_delete = (#proj_pairs == 0) and "1" or "0"
    ctx.proj_loading = "false"
    ctx.project_services = fmt.service_rows(
        proj_pairs,
        State.metrics_by_id,
        State.search_term
    )
    -- Variáveis de ambiente do projeto (aba "Variáveis (N)"). Sem isto, a aba
    -- nasce "Variáveis ()" até a primeira edição.
    local penv = proj.env_vars or {}
    local pcomments = proj.env_comments or {}
    ctx.proj_env = fmt.env_json_with_comments(penv, pcomments)
    ctx.proj_env_count = tostring(#penv)
    -- O snapshot chega ~1x/s; se reescrevêssemos proj_env_text (o value do
    -- textarea) toda vez, o que o usuário digita seria resetado. Só
    -- ressincroniza o editor .env quando ele está fechado.
    if ctx.penv_text_open ~= "true" then
        local pdotenv = fmt.env_dotenv_with_comments(penv, pcomments)
        ctx.proj_env_text = pdotenv
        ctx.proj_env_text_orig = pdotenv
    end
end

M.update_open_project = update_open_project

local function apply_snapshot(msg: any): ()
    State.snap = msg
    State.last_pairs = fmt.pair_list(msg.services)

    -- Status do daemon.
    local st = msg.status
    if st then
        ctx.daemon_version = st.version or ""
        ctx.daemon_uptime = fmt.fmt_uptime(st.uptime_secs)
        ctx.services_running = tostring(st.services_running or 0)
        ctx.services_total = tostring(st.services_total or 0)
        ctx.services_label = (
            tostring(st.services_running or 0) .. "/" .. tostring(st.services_total or 0)
        )
    end

    -- Deploy Engine (inline, não filtrado pela busca).
    local eng = msg.engine
    if eng then
        ctx.eng_active_count = tostring(#(eng.active or {}))
        ctx.eng_success_24h = tostring(eng.successful_24h or 0)
        ctx.eng_failed_24h = tostring(eng.failed_24h or 0)
        ctx.eng_total_24h = tostring(eng.total_24h or 0)
        ctx.eng_uptime = fmt.fmt_uptime(eng.uptime_secs)
        ctx.eng_active = fmt.eng_active(eng.active)
        ctx.eng_recent = fmt.eng_recent(eng.recent)
        ctx.eng_recent_count = tostring(#(eng.recent or {}))

        -- Timer decorrido do deploy em voo do serviço aberto (granularidade de
        -- 2s — sem timer no Luau, vem do próprio snapshot; ver o plano de
        -- migração). O desfecho terminal vem por DeployStateChanged (apply_bus).
        if
            State.deploy_track.running and
            ctx.selected_service == State.deploy_track.service_id
        then
            for _, a in ipairs(eng.active or {}) do
                if a.service_id == State.deploy_track.service_id then
                    ctx.svc_deploy_running = "true"
                    ctx.svc_deploy_elapsed = fmt.fmt_secs(a.elapsed_secs)
                    break
                end
            end
        end
    end

    -- Contagens.
    ctx.deployments_count = tostring(#(msg.deployments or {}))
    ctx.projects_count = tostring(#(msg.projects or {}))
    ctx.services_count = tostring(#State.last_pairs)
    ctx.docker_images_count = tostring(#(msg.docker_images or {}))
    ctx.docker_volumes_count = tostring(#(msg.docker_volumes or {}))
    ctx.docker_networks_count = tostring(#(msg.docker_networks or {}))
    ctx.docker_containers_count = tostring(#(msg.docker_containers or {}))

    -- Ingress + Monitoring (não filtrados pela busca).
    local ingress, icount = fmt.ingress(State.last_pairs)
    ctx.ingress = ingress
    ctx.ingress_count = tostring(icount)
    ctx.monitoring = fmt.monitoring(State.last_pairs, State.metrics_by_id)

    rebuild_lists()
    update_open_project()
    ctx.data_loading = "false"
end

-- Refresh imediato após uma mutação: pega o snapshot completo num único RPC (o
-- mesmo builder do SSE) e aplica, para a tela refletir na hora sem esperar o
-- tick de 2s. Chamado pelos handlers de criar/remover projeto/serviço.
local function refresh_now(): ()
    local api = State.api
    if not api then
        return
    end
    local ok, res = api:rpc("Snapshot")
    if ok and res.Snapshot then
        local okd, m = pcall(json.decode, res.Snapshot)
        if okd and type(m) == "table" then
            apply_snapshot(m)
        end
    end
end
M.refresh_now = refresh_now

-- Eventos vivos do bus: métricas e transições de deploy. Logs (runtime/build)
-- NÃO passam mais aqui — o daemon os tirou do firehose `/api/events` e os serve
-- por endpoints dedicados, consumidos só pelas janelas de log isoladas (ver
-- log_window.luau; open_logs_window / dep_logs em handlers/services.luau). Assim
-- a janela principal não reprocessa (reavalia) sua árvore inteira por linha.
local function apply_bus(ev: any): ()
    if type(ev) ~= "table" then
        return
    end
    if ev.ContainerMetrics then
        local p = ev.ContainerMetrics
        if p.service_id then
            State.metrics_by_id[p.service_id] = p
        end
    elseif ev.SystemMetrics then
        local s = ev.SystemMetrics
        ctx.sys_cpu = string.format("%.0f%%", s.cpu_percent or 0)
        ctx.sys_mem = (
            fmt.fmt_bytes(s.mem_used_bytes) .. " / " .. fmt.fmt_bytes(s.mem_total_bytes)
        )
        ctx.sys_disk = (
            fmt.fmt_bytes(s.disk_used_bytes) .. " / " .. fmt.fmt_bytes(s.disk_total_bytes)
        )
        ctx.sys_load = string.format(
            "%.2f %.2f %.2f",
            s.load_avg_1 or 0,
            s.load_avg_5 or 0,
            s.load_avg_15 or 0
        )
    elseif ev.DeployStateChanged then
        local d = ev.DeployStateChanged
        if d.service_id == State.deploy_track.service_id then
            local terminal = d.state == "Live" or d.state == "Failed" or d.state == "Stopped"
            if terminal then
                State.deploy_track.running = false
                local live = d.state == "Live"
                if d.service_id == ctx.selected_service then
                    ctx.svc_deploy_running = "false"
                    ctx.svc_action_msg = (
                        live and "deploy concluído" or ("deploy: " .. tostring(d.state))
                    )
                    ctx.svc_action_color = live and "#3FB950" or "#F85149"
                    Services.fetch_service_detail(d.service_id)
                end
            elseif d.service_id == ctx.selected_service then
                ctx.svc_deploy_running = "true"
                ctx.svc_action_msg = "deploy · " .. tostring(d.state)
                ctx.svc_action_color = "#58A6FF"
            end
        end
    end
end

-- Cada evento SSE traz um `data:` JSON auto-descritivo com `kind`.
function on_state(data: string): ()
    local ok, msg = pcall(json.decode, data)
    if not ok or type(msg) ~= "table" then
        return
    end
    if msg.kind == "snapshot" then
        apply_snapshot(msg)
    elseif msg.kind == "bus" then
        apply_bus(msg.event)
    end
    -- Nota: o firehose /api/events não manda mais `bus_batch` (era só p/ logs, que
    -- saíram daqui — ver apply_bus). Os lotes de log vivem no endpoint dedicado
    -- consumido pela janela isolada (log_window.luau).
end

function on_stream_error(e: string): ()
    ctx.status_line = "stream: " .. tostring(e)
end

function on_stream_close(): ()
    State.state_conn = nil
    if ctx.connected == "true" then
        ctx.status_line = "conexão encerrada"
        ctx.connected = "false"
        ctx.screen = "login"
    end
end

-- Abre a conexão SSE (chamado por connect(), em handlers/connection.luau,
-- após a autenticação inicial dar certo).
function M.open_stream(): ()
    if State.state_conn then
        return
    end
    local opts = {
        on_message = "on_state",
        on_error = "on_stream_error",
        on_close = "on_stream_close"
    }
    if ctx.api_token and ctx.api_token ~= "" then
        opts.headers = {
            Authorization = "Bearer " .. ctx.api_token
        }
    end
    State.state_conn = sse(ctx.api_url .. "/api/events", opts)
end

return M
