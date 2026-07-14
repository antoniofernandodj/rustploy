--!strict
--!nolint FunctionUnused
-- handlers/jobs.luau — ações da tela global "Schedules" (sidebar) e da aba
-- "Jobs" do projeto: rodar agora, pausar/ativar, remover, abrir a janela
-- "Novo job".

local State = require("../state")
local Helpers = require("../helpers")
local Stream = require("stream")

function job_run_now(id: string): ()
    local api = State.api
    if not api then
        return
    end
    local ok, res = api:rpc_checked({ JobRunNow = { id = id } })
    if ok then
        Helpers.notify_ok("Job disparado")
    else
        Helpers.notify_err(res)
    end
    Stream.refresh_now()
end

-- Pausa/ativa um job: `JobUpdate` substitui o registro inteiro, então
-- reenviamos os campos como estão (só invertendo `enabled`) — o Job cru vem
-- de `State.jobs_by_id`, preenchido pelo `stream.luau` a cada snapshot.
function job_toggle(id: string): ()
    local api = State.api
    if not api then
        return
    end
    local job = State.jobs_by_id[id]
    if not job then
        Helpers.notify_err("job não encontrado (tela desatualizada — tenta de novo)")
        return
    end
    local ok, res = api:rpc_checked({
        JobUpdate = {
            id = id,
            name = job.name,
            compose = job.compose,
            main_service = job.main_service,
            enabled = not job.enabled,
            recurrence = job.recurrence,
        },
    })
    if ok then
        Helpers.notify_ok(job.enabled and "Job pausado" or "Job ativado")
    else
        Helpers.notify_err(res)
    end
    Stream.refresh_now()
end

-- Abre a janela de cadastro de job (motor Glacier próprio, isolado). Passa a
-- conexão atual + os catálogos de projetos/serviços JÁ BUSCADOS pelo snapshot
-- (a janela filha não pode buscar no init — só ações despachadas rodam o
-- fetch). Ao concluir, ela emite broadcast("job_created"), recebido por
-- on_broadcast em handlers/projects.luau. Ver views/new_job_window.gv.
function open_new_job_window(): ()
    local snap = State.snap
    local projects = {}
    for _, p in ipairs((snap and snap.projects) or {}) do
        table.insert(projects, { id = p.id, name = p.name })
    end
    local services = {}
    for _, e in ipairs((snap and snap.services) or {}) do
        local svc = e.service
        if svc then
            table.insert(services, { id = svc.id, name = svc.spec.name, project_id = svc.spec.project_id })
        end
    end
    open_window({
        file = "crates/rustploy-gui/views/new_job_window.gv",
        title = "Novo job — Rustploy",
        width = 560,
        height = 700,
        resizable = false,
        data = {
            api_url = ctx.api_url or "",
            api_token = ctx.api_token or "",
            njob_projects = json.encode(projects),
            njob_services = json.encode(services),
        },
    })
end

-- Logs da última execução de um job — mesma janela genérica de logs
-- (log_window.gv/.luau) usada por runtime/build, agora apontando pro SSE
-- dedicado `/api/jobs/runs/<id>/logs` (ver Event::JobLogLine).
function open_job_logs_window(job_run_id: string): ()
    if not job_run_id or job_run_id == "" then
        return
    end
    local seed = "[]"
    local api = State.api
    if api then
        local ok, res = api:rpc({ GetJobLogs = { job_run_id = job_run_id } })
        if ok and res.JobLogs then
            seed = json.encode(res.JobLogs)
        end
    end
    open_window({
        file = "crates/rustploy-gui/views/log_window.gv",
        title = "Job — " .. string.sub(job_run_id, 1, 12),
        width = 900,
        height = 560,
        resizable = true,
        data = {
            api_url = ctx.api_url or "",
            api_token = ctx.api_token or "",
            lw_title = "Job · " .. string.sub(job_run_id, 1, 12),
            lw_stream_url = "/api/jobs/runs/" .. job_run_id .. "/logs",
            lw_seed = seed,
        },
    })
end

function job_del(id: string): ()
    local api = State.api
    if not api then
        return
    end
    local ok, res = api:rpc_checked({ JobDelete = { id = id } })
    if ok then
        Helpers.notify_ok("Job removido")
    else
        Helpers.notify_err(res)
    end
    Stream.refresh_now()
end
