--!strict
-- fmt/time.luau — timestamps, durações e barra de progresso. chrono (Rust)
-- formatava em horário LOCAL; aqui parseamos o RFC3339 (UTC, com "Z") e
-- convertemos para epoch verdadeiro somando o offset local, para então
-- formatar com os.date (que usa horário local).

local M = {}

local TZ_OFFSET: number = 0
do
    local ok, off = pcall(function()
        local now = os.time()
        return os.difftime(now, os.time(os.date("!*t", now)))
    end)
    if ok and type(off) == "number" then
        TZ_OFFSET = off
    end
end

-- RFC3339 "2026-07-06T12:34:56(.789)?(Z|±hh:mm)?" → epoch (segundos). Os
-- componentes são tratados como UTC; +TZ_OFFSET recupera o epoch verdadeiro.
local function iso_to_epoch(iso: string?): number?
    if type(iso) ~= "string" then return nil end
    local y, mo, d, h, mi, s = string.match(iso, "(%d+)-(%d+)-(%d+)T(%d+):(%d+):(%d+)")
    if not y then return nil end
    local utc = os.time{
        year = tonumber(y) :: number, month = tonumber(mo) :: number, day = tonumber(d) :: number,
        hour = tonumber(h) :: number, min = tonumber(mi) :: number, sec = tonumber(s) :: number,
    }
    if not utc then return nil end
    return utc + TZ_OFFSET
end

-- HH:MM:SS local de um timestamp ISO (ou "" se inválido).
function M.time_hms(iso: string?): string
    local ep = iso_to_epoch(iso)
    if not ep then return "" end
    return os.date("%H:%M:%S", ep)
end

-- "dd/mm HH:MM" local.
function M.date_dm_hm(iso: string?): string
    local ep = iso_to_epoch(iso)
    if not ep then return "" end
    return os.date("%d/%m %H:%M", ep)
end

-- "dd/mm HH:MM:SS" local.
function M.date_dm_hms(iso: string?): string
    local ep = iso_to_epoch(iso)
    if not ep then return "" end
    return os.date("%d/%m %H:%M:%S", ep)
end

-- Ns ou Mm Ns.
function M.fmt_secs(secs: any): string
    local num_secs = math.floor(tonumber(secs) or 0)
    local m = math.floor(num_secs / 60)
    local s = num_secs % 60
    if m > 0 then return string.format("%dm %ds", m, s) end
    return string.format("%ds", s)
end

function M.fmt_uptime(secs: any): string
    local num_secs = math.floor(tonumber(secs) or 0)
    local d = math.floor(num_secs / 86400)
    local h = math.floor((num_secs % 86400) / 3600)
    local m = math.floor((num_secs % 3600) / 60)
    local s = num_secs % 60
    if d > 0 then return string.format("%dd %dh %dm", d, h, m) end
    if h > 0 then return string.format("%dh %dm", h, m) end
    return string.format("%dm %ds", m, s)
end

-- Tamanho de bytes legível ("—" para 0), igual a view::fmt_bytes.
function M.fmt_bytes(b: any): string
    local num_b = tonumber(b) or 0
    local KB = 1024.0
    local MB = KB * 1024.0
    local GB = MB * 1024.0
    if num_b == 0 then return "—" end
    local f = num_b + 0.0
    if f >= GB then return string.format("%.1f GB", f / GB) end
    if f >= MB then return string.format("%.0f MB", f / MB) end
    return string.format("%.0f KB", f / KB)
end

-- Duração de um deployment (finished_at ou agora) − started_at, em fmt_secs.
function M.fmt_duration(dep: any): string
    local start = iso_to_epoch(dep.started_at)
    if not start then return "0s" end
    local finish = dep.finished_at and iso_to_epoch(dep.finished_at) or os.time()
    local secs = finish - start
    if secs < 0 then secs = 0 end
    return M.fmt_secs(secs)
end

-- Dias da semana no formato que o <radiogroup> do glacier-ui pede: uma coleção
-- de `{id, label}` no contexto, e não uma lista literal no atributo (o for-each
-- do motor lê chave, não texto). O `id` é o índice que `Recurrence::Weekly`
-- usa no daemon — 0 = segunda —, então ele viaja para o payload sem conversão.
-- Mora aqui, e não em cada tela, porque as duas janelas que agendam por semana
-- (limpeza automática e jobs) precisam da mesma lista.
-- Delimitador `[=[ ]=]` (e não `[[ ]]`) porque o conteúdo é um array JSON:
-- ele começa com `[` e termina com `]`, e as duas pontas encostariam nos
-- colchetes do próprio literal.
M.WEEKDAYS_JSON = [=[[{"id":"0","label":"Seg"},{"id":"1","label":"Ter"},{"id":"2","label":"Qua"},{"id":"3","label":"Qui"},{"id":"4","label":"Sex"},{"id":"5","label":"Sáb"},{"id":"6","label":"Dom"}]]=]

-- ── Ponte entre o <timeedit> e o protocolo ──────────────────────────────────
-- O <timeedit> do glacier-ui (0.68) guarda o horário numa chave ÚNICA, em ISO
-- ("HH:MM"). O daemon, do outro lado, sempre falou em dois campos separados
-- (`{hour, minute}`, em Recurrence::Daily/Weekly e em DockerCleanupConfig).
-- Estas duas funções são a tradução, e existem para que o contrato HTTP não
-- precise mudar por causa de uma troca de widget.

-- (hour, minute) -> "HH:MM". Aceita número ou string (o unpack das recurrences
-- devolve string) e satura na faixa válida, para que um valor corrompido vindo
-- do servidor não produza uma chave que o widget não consegue parsear.
function M.hm_join(hour: any, minute: any): string
    local h = math.floor(tonumber(hour) or 0)
    local m = math.floor(tonumber(minute) or 0)
    if h < 0 then h = 0 elseif h > 23 then h = 23 end
    if m < 0 then m = 0 elseif m > 59 then m = 59 end
    return string.format("%02d:%02d", h, m)
end

-- "HH:MM" -> hour, minute (números). Uma chave vazia ou malformada vira 0h00,
-- que é o mesmo default que o `tonumber(...) or 0` dos handlers já usava.
function M.hm_split(hm: any): (number, number)
    local texto = tostring(hm or "")
    local h, m = string.match(texto, "^(%d+):(%d+)$")
    if not h then return 0, 0 end
    local hora = math.floor(tonumber(h) or 0)
    local minuto = math.floor(tonumber(m) or 0)
    if hora < 0 or hora > 23 then hora = 0 end
    if minuto < 0 or minuto > 59 then minuto = 0 end
    return hora, minuto
end

-- Barra de progresso em blocos.
function M.progress_bar(percent: any, width: number?): string
    local p = tonumber(percent) or 0
    local w = width or 12
    local filled = math.floor(p * w / 100)
    if filled > w then filled = w end
    if filled < 0 then filled = 0 end
    return string.rep("█", filled) .. string.rep("░", w - filled)
end

return M
