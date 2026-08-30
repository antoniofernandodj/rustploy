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
    ctx.registry_repos = fmt.registry_repos(snap.registry_repos, term)
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

    -- Aba "Secrets (N)": só os nomes — `projects[].secrets` vem do snapshot
    -- (ver http_api.rs::snapshot), nunca o valor. A mesma lista alimenta os
    -- chips de "usar secret" da aba Variáveis.
    local secrets = proj.secrets or {}
    ctx.proj_secrets = fmt.secret_rows(secrets)
    ctx.proj_secrets_count = tostring(#secrets)

    -- Aba "Jobs (N)": jobs deste projeto (filtrados do `msg.jobs` global —
    -- já busca tudo a cada snapshot, sem RPC extra).
    local proj_jobs = {}
    for _, s in ipairs(snap.jobs or {}) do
        if s.job.project_id == pid then
            table.insert(proj_jobs, s)
        end
    end
    local jobs_json, jobs_count = fmt.job_summaries(proj_jobs, State.jobs_inflight)
    ctx.proj_jobs = jobs_json
    ctx.proj_jobs_count = tostring(jobs_count)
end

M.update_open_project = update_open_project

-- Recomputa SÓ `ctx.jobs_summary`/`ctx.jobs_count` (tela global "Schedules")
-- a partir do último snapshot em cache — sem RPC. Chamada por
-- handlers/jobs.luau::job_run_now logo após tocar `State.jobs_inflight`,
-- pra o botão desabilitar na hora, sem esperar o round-trip do
-- `JobRunNow`/`refresh_now()`.
local function refresh_jobs_summary_local(): ()
    local snap = State.snap
    if not snap then
        return
    end
    local jobs_summary, jobs_count = fmt.job_summaries(snap.jobs, State.jobs_inflight)
    ctx.jobs_summary = jobs_summary
    ctx.jobs_count = tostring(jobs_count)
end
M.refresh_jobs_summary_local = refresh_jobs_summary_local

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
        -- Fila global (um por vez): itens esperando + estado pausado.
        ctx.eng_queued = fmt.eng_queued(eng.queued)
        ctx.eng_queued_count = tostring(#(eng.queued or {}))
        ctx.eng_paused = (eng.paused and "true" or "false")

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

    -- Registry OCI embutido: status/config vem sempre; tags de um repo
    -- específico são buscadas sob demanda (ver handlers/registry.luau), não
    -- fazem parte do snapshot.
    local rst = msg.registry_status
    if rst then
        ctx.registry_status_enabled = tostring(rst.enabled)
        ctx.registry_host = (rst.domain and rst.domain ~= "") and rst.domain or ("127.0.0.1:" .. tostring(rst.port))
        ctx.registry_status_label = rst.enabled and ("ativo em " .. ctx.registry_host) or "desabilitado"
        ctx.registry_storage_human = fmt.fmt_bytes(rst.storage_bytes or 0)
    end
    ctx.registry_repos_count = tostring(#(msg.registry_repos or {}))

    -- Ingress + Monitoring (não filtrados pela busca).
    local ingress, icount = fmt.ingress(State.last_pairs)
    ctx.ingress = ingress
    ctx.ingress_count = tostring(icount)
    local host_ports, hpcount = fmt.host_ports(State.last_pairs)
    ctx.host_ports = host_ports
    ctx.host_ports_count = tostring(hpcount)
    ctx.monitoring = fmt.monitoring(State.last_pairs, State.metrics_by_id)

    -- Jobs one-shot (tela global "Schedules"): guarda o Job cru por id
    -- (job_toggle precisa do compose/name/main_service completos pra
    -- reenviar o JobUpdate) e formata as linhas de exibição.
    local jobs_by_id = {}
    for _, s in ipairs(msg.jobs or {}) do
        jobs_by_id[s.job.id] = s.job
    end
    State.jobs_by_id = jobs_by_id
    local jobs_summary, jobs_count = fmt.job_summaries(msg.jobs, State.jobs_inflight)
    ctx.jobs_summary = jobs_summary
    ctx.jobs_count = tostring(jobs_count)

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

-- Limpeza em massa da tela Deployments: apaga (DeployDelete) todo deployment
-- em estado terminal Stopped/Failed do último snapshot recebido — em
-- andamento (Live e os estados intermediários) nunca são tocados, e o próprio
-- handler do daemon recusa apagar um deployment não-terminal como segunda
-- trava.
function deployments_clear_finished(): ()
    local ok_confirm = confirm{
        title = "Limpar deployments parados/falhos?",
        message = "Remove do histórico todo deployment em estado Stopped ou Failed, e seus build logs. Deployments em andamento não são afetados. Ação irreversível.",
        confirm_label = "Limpar",
        destructive = true,
    }

    if not ok_confirm then
        return
    end

    local api = State.api
    local snap = State.snap
    if not api or not snap then
        return
    end
    local ids = {}
    for _, s in ipairs(snap.deployments or {}) do
        local d = s.deployment
        if d and (d.state == "Stopped" or d.state == "Failed") then
            table.insert(ids, d.id)
        end
    end
    if #ids == 0 then
        ctx.deployments_msg = "nada para limpar"
        return
    end
    ctx.deployments_msg = string.format("removendo %d deployment(s)…", #ids)
    local removed = 0
    local failed = 0
    for _, id in ipairs(ids) do
        local ok = api:rpc_checked{ DeployDelete = { deployment_id = id } }
        if ok then
            removed = removed + 1
        else
            failed = failed + 1
        end
    end
    if failed == 0 then
        ctx.deployments_msg = string.format("%d removido(s)", removed)
    else
        ctx.deployments_msg = string.format("%d removido(s), %d falharam", removed, failed)
        toast{
            message = tostring(
                string.format("%d deployment(s) não puderam ser removidos", failed)
            ),
            kind = "error"
        }
    end
    refresh_now()
end


-- Eventos vivos do bus: métricas e transições de deploy. Logs (runtime/build)
-- NÃO passam mais aqui — o daemon os tirou do firehose `/api/events` e os serve
-- por endpoints dedicados, consumidos só pelas janelas de log isoladas (ver
-- log_window.luau; open_logs_window / dep_logs em handlers/services.luau). Assim
-- a janela principal não reprocessa (reavalia) sua árvore inteira por linha.
local function apply_bus(ev: any): ()
    -- Eventos unit-variant chegam como string JSON (ex.: "DeployQueueChanged"),
    -- não table — tratados antes do guard de table abaixo.
    if ev == "DeployQueueChanged" then
        -- A fila global mudou: busca um snapshot fresco na hora, sem esperar o
        -- tick de 2s.
        refresh_now()
        return
    end
    if type(ev) ~= "table" then
        return
    end
    if ev.DockerCleanupCompleted then
        -- Settings.luau não é `require`ado aqui de propósito: ele já requer
        -- este arquivo (`Stream = require("stream")`), e um require cíclico
        -- devolveria um módulo incompleto. Atualiza os 3 campos direto.
        local p = ev.DockerCleanupCompleted
        ctx.dc_running = "false"
        ctx.dc_last_run_text = fmt.docker_cleanup_last_run_summary({ at = p.at, results = p.results })
        State.dc_last_run_at_raw = p.at
    elseif ev.ContainerMetrics then
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
                -- Notificação NATIVA DO SO no desfecho: o usuário disparou o deploy
                -- e provavelmente foi fazer outra coisa — o toast (dentro da janela)
                -- não seria visto. Emitida sempre, independente da tela/serviço
                -- selecionado (por isso fora do gate de selected_service abaixo).
                -- app_name = "Rustploy" (não o nome do binário "rustploy-gui"): há
                -- desktops (ex.: certos GNOME/extensões) que descartam a notificação
                -- quando o app_name casa com um .desktop instalado — o nome de
                -- exibição evita isso. Ver glacier-ui 0.43 e docs/notificacao-so.md.
                local svc_name = State.deploy_track.name
                if svc_name == "" then
                    svc_name = "serviço"
                end
                -- `d.message` é o MOTIVO da falha (o texto do Docker, o
                -- healthcheck que não passou…). O evento sempre o carregou; era
                -- este consumidor que o descartava, e a notificação dizia só
                -- "Failed" — o usuário ficava sem hipótese nenhuma. Ver
                -- docs/plano-erro-de-deploy-invisivel.md.
                local motivo = fmt.short_reason(d.message)
                if live then
                    notify{
                        title = "Deploy concluído",
                        body = svc_name .. " está no ar e recebendo tráfego.",
                        app_name = "Rustploy",
                        icon = "rustploy-gui",
                    }
                else
                    notify{
                        title = "Deploy falhou",
                        body = (
                            motivo ~= ""
                                and (svc_name .. ": " .. motivo)
                                or (svc_name .. ": " .. tostring(d.state) .. ".")
                        ),
                        app_name = "Rustploy",
                        icon = "rustploy-gui",
                    }
                    -- Toast além da notificação do SO: quem está com a janela
                    -- aberta olhando o deploy não recebe notificação em alguns
                    -- desktops, e é justamente quem mais quer ver o motivo.
                    toast{
                        message = svc_name .. ": " .. (
                            motivo ~= "" and motivo or ("deploy " .. tostring(d.state))
                        ),
                        kind = "error",
                    }
                end
                if d.service_id == ctx.selected_service then
                    ctx.svc_deploy_running = "false"
                    ctx.svc_action_msg = (
                        live and "deploy concluído"
                            or (
                                motivo ~= ""
                                    and ("deploy falhou: " .. motivo)
                                    or ("deploy: " .. tostring(d.state))
                            )
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

-- Abre a conexão SSE (chamado por connect(), em handlers/connection.luau,
-- após a autenticação inicial dar certo). Os callbacks vão inline: cada um é
-- usado num lugar só, e a assinatura dos três continua conferida contra o que o
-- motor entrega sem precisar anotar variável nenhuma — `glacier.d.luau` declara
-- `sse(url, opts: StreamOptions?)`, então é a própria chamada que checa a
-- tabela literal contra `StreamOptions`.

local function get_headers(ctx: any): any
    local headers = { }
    if ctx.api_token and ctx.api_token ~= "" then
        headers = {
            Authorization = "Bearer " .. ctx.api_token
        }
    end

    return headers
end

function M.open_stream(): ()
    if State.state_conn then
        return
    end

    local sse_url = ctx.api_url .. "/api/events"

    State.state_conn = sse(sse_url, {
        -- Cada evento SSE traz um `data:` JSON auto-descritivo com `kind`.
        -- Nota: o firehose /api/events não manda mais `bus_batch` (era só p/
        -- logs, que saíram daqui — ver apply_bus). Os lotes de log vivem no
        -- endpoint dedicado consumido pela janela isolada (log_window.luau).
        headers = get_headers(ctx),

        on_message = function(data: string): ()
            local ok, msg = pcall(json.decode, data)
            if not ok or type(msg) ~= "table" then
                return
            end
            if msg.kind == "snapshot" then
                apply_snapshot(msg)
            elseif msg.kind == "bus" then
                apply_bus(msg.event)
            end
        end,

        on_error = function(e: string): ()
            ctx.status_line = "stream: " .. tostring(e)
        end,

        on_close = function(): ()
            State.state_conn = nil
            if ctx.connected == "true" then
                ctx.status_line = "conexão encerrada"
                ctx.connected = "false"
                ctx.screen = "login"
            end
        end,
    })
end

return M
