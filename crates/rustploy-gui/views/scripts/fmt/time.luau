--!strict
-- fmt/time.luau — timestamps, durações e barra de progresso.
--
-- Os timestamps chegam do daemon em RFC 3339 UTC ("2026-07-06T12:34:56Z") e a
-- tela mostra hora local. Desde a glacier-ui 0.73 essa tradução é do motor: o
-- global `date` aceita o formato com fuso, `to_local` desloca o instante e
-- `format` desenha — então o que sobra aqui é só a escolha do formato de
-- exibição, uma linha por função.
--
-- Antes disto o arquivo tinha um `iso_to_epoch` próprio e um `TZ_OFFSET`
-- calculado com `os.difftime(now, os.time(os.date("!*t", now)))`. Esse offset
-- era SEMPRE ZERO, e vale registrar por quê: o `os.time` do Luau usa `timegm`,
-- e não o `mktime` do Lua, então uma tabela de componentes é lida como UTC e
-- `os.time(os.date("!*t", t))` devolve o próprio `t`. A soma era inofensiva
-- (quem convertia para local era o `os.date` seguinte), mas o comentário
-- descrevia uma conta que não acontecia. O equivalente correto no dialeto é
-- `os.time(os.date("*t", t)) - t`, e é o que o `date` do motor usa por dentro.

local M = {}

-- As três funções abaixo aceitam `nil` e devolvem "": um `finished_at` ainda
-- não preenchido, ou um campo que o daemon omitiu, é o caso normal e não
-- deveria obrigar cada chamador a testar. `date.to_local` e `date.format`
-- propagam o nil sozinhos, então basta o `or ""` no fim.

-- HH:MM:SS local de um timestamp do daemon.
function M.time_hms(iso: string?): string
    return date.format(date.to_local(iso), "HH:mm:SS") or ""
end

-- "dd/mm HH:MM" local.
function M.date_dm_hm(iso: string?): string
    return date.format(date.to_local(iso), "DD/MM HH:mm") or ""
end

-- "dd/mm HH:MM:SS" local.
function M.date_dm_hms(iso: string?): string
    return date.format(date.to_local(iso), "DD/MM HH:mm:SS") or ""
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
--
-- `date.diff_seconds` compara INSTANTES, não texto: ele traz cada lado para a
-- hora local antes de subtrair, e é por isso que dá para misturar o
-- `finished_at` do daemon (RFC 3339 com "Z") com o `date.now()` da máquina
-- (sem fuso, hora local) sem converter nenhum dos dois na mão.
function M.fmt_duration(dep: any): string
    local fim: string = dep.finished_at or date.now(true)
    local secs: number? = date.diff_seconds(fim, dep.started_at)
    if secs == nil or secs < 0 then return "0s" end
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
