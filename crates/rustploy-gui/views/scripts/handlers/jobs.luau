--!strict
--!nolint FunctionUnused
-- handlers/jobs.luau — ações da tela global "Schedules" (sidebar) e da aba
-- "Jobs" do projeto: rodar agora, pausar/ativar, remover, abrir a janela
-- "Novo job".

local State = require("../state")
local Stream = require("stream")
local fmt = require("../fmt")

-- Bloqueia o botão IMEDIATAMENTE ao clicar (antes de qualquer resposta do
-- backend chegar) e trava double-click: `State.jobs_inflight[id] = true` +
-- os dois recomputos locais (sem RPC) fazem `j.running` virar `true` na
-- HORA em ambas as telas (sidebar Schedules e aba Jobs do projeto) — sem
-- isso o botão só desabilitava quando `refresh_now()` (rede) terminava,
-- alguns segundos depois do click. Só desbloqueia em erro (na hora, sem
-- esperar `refresh_now()`) ou depois que `refresh_now()` confirma o
-- `job_run` criado (dali em diante `j.running` já vem do snapshot real).
function job_run_now(id: string): ()
    local api = State.api
    if not api then
        return
    end
    if State.jobs_inflight[id] then
        return
    end
    State.jobs_inflight[id] = true
    Stream.update_open_project()
    Stream.refresh_jobs_summary_local()

    local ok, res = api:rpc_checked{ JobRunNow = { id = id } }
    if not ok then
        toast{ message = tostring(res), kind = "error" }
        State.jobs_inflight[id] = nil
        Stream.update_open_project()
        Stream.refresh_jobs_summary_local()
        return
    end
    toast{ message = tostring("Job disparado"), kind = "success" }
    Stream.refresh_now()
    State.jobs_inflight[id] = nil
end

-- Cancela um job_run em execução (ver docs/plano-cancelamento-de-jobs.md):
-- mata o processo `docker compose up` de verdade no daemon, não só a UI.
-- `job_run_id` vem de `j.last_run_id` — enquanto `j.running` só reflete o
-- estado otimista (ver `job_run_now`, antes do primeiro refresh confirmar o
-- job_run novo), pode ainda apontar pro run ANTERIOR; o daemon devolve
-- NotFound nesse caso (inofensivo, só o toast de erro).
function job_run_cancel(job_run_id: string): ()
    local api = State.api
    if not api or not job_run_id or job_run_id == "" then
        return
    end
    local ok, res = api:rpc_checked{ JobRunCancel = { job_run_id = job_run_id } }
    if ok then
        toast{ message = tostring("Cancelamento solicitado"), kind = "success" }
    else
        toast{ message = tostring(res), kind = "error" }
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
        toast{ message = tostring("job não encontrado (tela desatualizada — tenta de novo)"), kind = "error" }
        return
    end
    local ok, res = api:rpc_checked{
        JobUpdate = {
            id = id,
            name = job.name,
            compose = job.compose,
            git_source = job.git_source,
            main_service = job.main_service,
            env_vars = job.env_vars,
            env_comments = job.env_comments,
            enabled = not job.enabled,
            recurrence = job.recurrence,
        },
    }
    if ok then
        toast{ message = tostring(job.enabled and "Job pausado" or "Job ativado"), kind = "success" }
    else
        toast{ message = tostring(res), kind = "error" }
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
    open_window{
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
    }
end

-- Abre a MESMA janela em modo edição: pula os passos 1/2 (projeto/serviço
-- gatilho não são editáveis via JobUpdate) e semeia o passo 3 já preenchido.
-- Se o job for git-sourced, resolve conta/repo/branch ANTES de abrir (a
-- janela filha não pode buscar no init — mesma restrição de
-- open_new_job_window) pra o picker já vir com a seleção certa, sem forçar
-- o usuário a escolher de novo só pra editar outra coisa (ex.: env vars).
function open_edit_job_window(id: string): ()
    local job = State.jobs_by_id[id]
    if not job then
        toast{ message = tostring("job não encontrado (tela desatualizada — tenta de novo)"), kind = "error" }
        return
    end
    local api = State.api

    local njob_git_providers = "[]"
    local njob_git_repos = "[]"
    local njob_git_repo = ""
    local njob_git_branches = "[]"
    if job.git_source and api then
        local pid = job.git_source.provider_id
        if pid and pid ~= "" then
            -- Essa cascata de RPCs (conta → repos → branches) roda ANTES de
            -- `open_window` — a janela filha não pode buscar no próprio
            -- init (mesma restrição de `open_new_job_window`), então até ela
            -- abrir o clique em "Editar" fica sem NENHUM feedback visual.
            -- Só o toast avisa que algo está acontecendo nesse intervalo.
            toast{ message = tostring("Carregando dados do repositório…"), kind = "info" }
            local okp, resp = api:rpc("GitProviderList")
            if okp and resp.GitProviders then
                njob_git_providers = fmt.git_providers(resp.GitProviders)
            end
            local okr, resr = api:rpc{ GitRepoList = { provider_id = pid } }
            if okr and resr.GitRepos then
                njob_git_repos = fmt.git_repos(resr.GitRepos)
                for _, r in ipairs(resr.GitRepos) do
                    if r.clone_url == job.git_source.url then
                        njob_git_repo = r.full_name
                        break
                    end
                end
                if njob_git_repo ~= "" then
                    local okb, resb = api:rpc{
                        GitBranchList = { provider_id = pid, repo_full_name = njob_git_repo },
                    }
                    if okb and resb.GitBranches then
                        njob_git_branches = fmt.git_branches(resb.GitBranches)
                    end
                end
            end
        end
    end

    open_window{
        file = "crates/rustploy-gui/views/new_job_window.gv",
        title = "Editar job — Rustploy",
        width = 560,
        height = 700,
        resizable = false,
        data = {
            api_url = ctx.api_url or "",
            api_token = ctx.api_token or "",
            njob_edit_job = json.encode(job),
            njob_git_providers = njob_git_providers,
            njob_git_repos = njob_git_repos,
            njob_git_repo = njob_git_repo,
            njob_git_branches = njob_git_branches,
        },
    }
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
        local ok, res = api:rpc{ GetJobLogs = { job_run_id = job_run_id } }
        if ok and res.JobLogs then
            seed = json.encode(res.JobLogs)
        end
    end
    open_window{
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
    }
end

function job_del(id: string): ()
    local api = State.api
    if not api then
        return
    end
    local ok, res = api:rpc_checked{ JobDelete = { id = id } }
    if ok then
        toast{ message = tostring("Job removido"), kind = "success" }
    else
        toast{ message = tostring(res), kind = "error" }
    end
    Stream.refresh_now()
end
