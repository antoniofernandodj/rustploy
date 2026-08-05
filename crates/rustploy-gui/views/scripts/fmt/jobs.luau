--!strict
-- fmt/jobs.luau — formata Job/JobSummary/JobRun (tarefas one-shot via
-- docker-compose) pra exibição: tela global "Schedules" (sidebar) e aba
-- "Jobs" do detalhe do projeto. Recorrência é estruturada (Interval/Daily/
-- Weekly, ou nil = só manual) — resumida em texto curto pra tabela.

local Time = require("time")
local Util = require("util")

local M = {}

local WEEKDAYS = { "seg", "ter", "qua", "qui", "sex", "sáb", "dom" }

-- Resumo textual de uma `Recurrence?` (serde externally-tagged: `nil` = só
-- manual; `{IntervalHours=6}` / `{Daily={hour=,minute=}}` / `{Weekly={...}}`).
function M.recurrence_label(r: any): string
    if r == nil then
        return "manual"
    end
    if r.IntervalHours ~= nil then
        return "a cada " .. tostring(r.IntervalHours) .. "h"
    end
    if r.Daily ~= nil then
        return string.format("diariamente às %02d:%02d", r.Daily.hour, r.Daily.minute)
    end
    if r.Weekly ~= nil then
        local wd = WEEKDAYS[(r.Weekly.weekday % 7) + 1] or "?"
        return string.format("semanalmente %s às %02d:%02d", wd, r.Weekly.hour, r.Weekly.minute)
    end
    return "—"
end

-- Rótulo/cor (mesmo esquema de StateCell) da última execução de um job.
-- `run == nil` (nunca rodou) / `run.success == nil` (rodando agora) /
-- `true`/`false` (ok/falhou).
function M.run_status(run: any): (string, string)
    if run == nil then
        return "nunca rodou", "muted"
    end
    if run.success == nil then
        return "rodando…", "warn"
    end
    if run.success then
        return "ok", "ok"
    end
    return "falhou", "bad"
end

-- Tela global "Schedules": uma linha por job, de todos os projetos.
function M.summaries(list: { any }?): (string, number)
    local B = Util.col_budgets()
    local rows = {}
    for _, s in ipairs(list or {}) do
        local job = s.job
        local status_label, status_kind = M.run_status(s.last_run)
        table.insert(rows, {
            id = job.id,
            name = Util.ellipsis(job.name, B.svc),
            project = s.project_name,
            trigger_service = s.trigger_service_name,
            -- trigger_service_name é nil quando o job é autônomo (sem
            -- serviço gatilho) — mostra só o projeto nesse caso.
            owner = s.trigger_service_name
                and Util.ellipsis(tostring(s.project_name) .. " / " .. tostring(s.trigger_service_name), B.proj)
                or Util.ellipsis(tostring(s.project_name), B.proj),
            recurrence = M.recurrence_label(job.recurrence),
            enabled = job.enabled,
            enabled_label = job.enabled and "Pausar" or "Ativar",
            last_run_label = status_label,
            last_run_kind = status_kind,
            -- `running`: usado pra desativar/trocar o botão "Rodar agora"
            -- enquanto o job_run em voo não termina (mesma condição de
            -- status_kind == "warn").
            running = status_kind == "warn",
            last_run_id = s.last_run and s.last_run.id or "",
            next_run_at = job.next_run_at and Time.date_dm_hm(job.next_run_at) or "—",
        })
    end
    return Util.enc_array(rows), #rows
end

-- Fila de checks de pré-deploy do serviço (aba Advanced), na ORDEM
-- configurada, com o nome resolvido a partir dos Jobs do projeto — mesmo
-- idioma de retrocompat do `ServiceSpec::pre_deploy_checks` no lado Rust
-- (`Util.pre_deploy_checks`, que cai no `pre_deploy_job_id` legado quando a
-- fila nova está vazia). Um job apagado por fora (Schedules) mas ainda
-- referenciado aparece com rótulo "(job removido)" em vez de sumir — assim
-- dá pra removê-lo da fila pela própria UI.
function M.pre_deploy_queue(spec: any, list: { any }?): string
    local by_id = {}
    for _, job in ipairs(list or {}) do
        by_id[job.id] = job.name
    end
    local rows = {}
    for i, id in ipairs(Util.pre_deploy_checks(spec)) do
        table.insert(rows, {
            id = id,
            name = by_id[id] or ("(job removido: " .. id .. ")"),
            index = tostring(i),
        })
    end
    return Util.enc_array(rows)
end

-- Jobs do projeto que AINDA NÃO estão na fila — opções do `<select>` de
-- "adicionar à fila" (evita duplicar o mesmo job na sequência).
function M.pre_deploy_available(spec: any, list: { any }?): string
    local in_queue = {}
    for _, id in ipairs(Util.pre_deploy_checks(spec)) do
        in_queue[id] = true
    end
    local rows = {}
    for _, job in ipairs(list or {}) do
        if not in_queue[job.id] then
            table.insert(rows, { id = job.id, name = job.name })
        end
    end
    return Util.enc_array(rows)
end

-- Histórico de execuções (`Command::JobRunHistory`) — usado pra listar runs e
-- abrir "Ver logs" (log_window apontando pro `job_run_id`).
function M.runs(list: { any }?): string
    local rows = {}
    for _, r in ipairs(list or {}) do
        local label, kind = M.run_status(r)
        table.insert(rows, {
            id = r.id,
            started_at = Time.date_dm_hms(r.started_at),
            finished_at = r.finished_at and Time.date_dm_hms(r.finished_at) or "—",
            exit_code = r.exit_code ~= nil and tostring(r.exit_code) or "—",
            status_label = label,
            status_kind = kind,
        })
    end
    return Util.enc_array(rows)
end

return M
