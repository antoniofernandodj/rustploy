--!strict
--!nolint FunctionUnused
-- log_window.luau — script da JANELA de logs ao vivo de um serviço, um motor
-- Glacier próprio e ISOLADO do app principal (aberto via open_window por
-- handlers/services.luau::open_logs_window). Recebe semeados no contexto por
-- open_window({ data = ... }):
--   api_url / api_token   — conexão com o daemon
--   lw_service_id         — serviço cujos logs seguir (filtro do LogLine)
--   lw_service_name       — só p/ o título
--   lw_seed               — JSON do ring de logs atual (tail já buscado pelo pai)
--
-- Por que uma janela à parte: o glacier reavalia TODOS os templates a cada
-- reavaliação; neste motor só existe log_window.xml (minúsculo), então rolar
-- log a alta frequência é barato aqui, ao contrário da janela principal.
--
-- sse() no init() FUNCIONA (o motor liga as streams abertas no init — ver
-- install_component no glacier); só `fetch` no init é descartado, por isso o
-- tail inicial vem semeado via `lw_seed` em vez de um LogsGet aqui.
--
-- A stream usa o endpoint DEDICADO `/api/services/{id}/logs` (só os LogLine
-- deste serviço), não o firehose `/api/events` — que de propósito não carrega
-- mais logs de container, para a janela principal não reprocessá-los (todas as
-- janelas do iced::daemon dividem uma thread de UI). Ver `service_logs` no daemon.
--
-- require resolve relativo a ESTE arquivo (scripts/): "fmt" (fachada de fmt/*).

local fmt = require("fmt")

local RING = 500
local FLUSH_SECS = 0.1

local logs: { any } = {}
local dirty = false
local flush_at = 0

-- Reflete o ring no contexto (lista colorida + textão p/ copiar). fmt.logs_json
-- já limpa ANSI e limita à cauda; fmt.join_log_lines idem (sem cap, p/ copiar).
local function flush(): ()
    ctx.lw_logs = fmt.logs_json(logs)
    ctx.lw_count = tostring(#logs)
    ctx.lw_logs_text = fmt.join_log_lines(logs)
    dirty = false
    flush_at = os.clock() + FLUSH_SECS
end

local function push(e: any): ()
    table.insert(logs, { stream = e.stream, line = e.line, timestamp = e.timestamp })
    while #logs > RING do
        table.remove(logs, 1)
    end
    dirty = true
    if os.clock() >= flush_at then
        flush()
    end
end

-- Um evento do bus: só nos interessa LogLine do serviço-alvo.
local function on_bus(ev: any): ()
    if type(ev) == "table" and ev.LogLine then
        local e = ev.LogLine
        if e.service_id == ctx.lw_service_id then
            push(e)
        end
    end
end

-- Callback do sse(): cada `data:` é um JSON auto-descritivo (kind). O endpoint
-- dedicado só manda lotes `bus_batch` de LogLine; ainda tratamos `bus` avulso
-- por robustez. (Não há `snapshot` neste endpoint.)
function lw_on_state(data: string): ()
    local ok, msg = pcall(json.decode, data)
    if not ok or type(msg) ~= "table" then
        return
    end
    if msg.kind == "bus" then
        on_bus(msg.event)
    elseif msg.kind == "bus_batch" then
        for _, e in ipairs(msg.events or {}) do
            on_bus(e)
        end
        -- Escoa o lote inteiro num rebuild só (ignora o debounce por-linha).
        if dirty then
            flush()
        end
    end
end

function init(): ()
    ctx.lw_service_id = ctx.lw_service_id or ""
    ctx.lw_service_name = ctx.lw_service_name or ""

    -- Tail inicial semeado pelo pai (o LogsGet já foi feito lá).
    local oks, seed = pcall(json.decode, ctx.lw_seed or "[]")
    logs = (oks and type(seed) == "table") and seed or {}
    flush()

    -- SSE própria desta janela no endpoint dedicado de logs do serviço (o motor
    -- liga streams abertas no init; o handle não precisa ser guardado — a stream
    -- vive enquanto a janela/motor existir).
    local opts: any = { on_message = "lw_on_state" }
    if ctx.api_token and ctx.api_token ~= "" then
        opts.headers = { Authorization = "Bearer " .. ctx.api_token }
    end
    sse((ctx.api_url or "") .. "/api/services/" .. (ctx.lw_service_id or "") .. "/logs", opts)
end
