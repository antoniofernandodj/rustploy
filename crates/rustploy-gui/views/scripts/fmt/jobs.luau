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
            last_run_id = s.last_run and s.last_run.id or "",
            next_run_at = job.next_run_at and Time.date_dm_hm(job.next_run_at) or "—",
        })
    end
    return Util.enc_array(rows), #rows
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
