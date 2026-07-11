--!strict
--!nolint FunctionUnused
-- log_window.luau — script da JANELA de logs (runtime OU build) de um serviço/
-- deployment, um motor Glacier próprio e ISOLADO do app principal (aberto via
-- open_window por handlers/services.luau: open_logs_window / open_build_logs_window).
-- Genérico: a mesma janela serve os dois tipos, parametrizada por `data`:
--   api_url / api_token   — conexão com o daemon
--   lw_title              — título (ex.: "Logs — api" / "Build — dep_01KX…")
--   lw_stream_url         — path do SSE dedicado a seguir:
--                             "/api/services/<id>/logs"          (runtime)
--                             "/api/deployments/<id>/build-logs" (build)
--   lw_seed               — JSON do tail/histórico inicial (o pai já buscou)
--
-- Por que uma janela à parte: o glacier reavalia TODOS os templates a cada
-- reavaliação; neste motor só existe log_window.xml (minúsculo), então rolar
-- log a alta frequência é barato aqui, ao contrário da janela principal (cuja
-- árvore inteira — shell+home+service — seria reavaliada por linha, congelando;
-- todas as janelas do iced::daemon dividem UMA thread de UI).
--
-- sse() no init() FUNCIONA (o motor liga as streams abertas no init — ver
-- install_component no glacier); só `fetch` no init é descartado, por isso o
-- histórico inicial vem semeado via `lw_seed` em vez de um RPC aqui. A stream usa
-- o endpoint DEDICADO (não o firehose `/api/events`, que de propósito não carrega
-- mais logs). Ver `service_logs`/`deployment_build_logs` no daemon.
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

-- Um evento do bus: os endpoints já filtram por id, então basta pegar o LogLine
-- (runtime) ou BuildLog (build), o que vier. BuildLog não traz `stream` (cor cai
-- no padrão em fmt.logs_json).
local function on_ev(ev: any): ()
    if type(ev) ~= "table" then
        return
    end
    local e = ev.LogLine or ev.BuildLog
    if e then
        push(e)
    end
end

-- Callback do sse(): cada `data:` é um JSON auto-descritivo (kind). O endpoint
-- dedicado só manda lotes `bus_batch`; ainda tratamos `bus` avulso por robustez.
function lw_on_state(data: string): ()
    local ok, msg = pcall(json.decode, data)
    if not ok or type(msg) ~= "table" then
        return
    end
    if msg.kind == "bus" then
        on_ev(msg.event)
    elseif msg.kind == "bus_batch" then
        for _, e in ipairs(msg.events or {}) do
            on_ev(e)
        end
        -- Escoa o lote inteiro num rebuild só (ignora o debounce por-linha).
        if dirty then
            flush()
        end
    end
end

function init(): ()
    ctx.lw_title = ctx.lw_title or "Logs"

    -- Histórico inicial semeado pelo pai (o RPC já foi feito lá).
    local oks, seed = pcall(json.decode, ctx.lw_seed or "[]")
    logs = (oks and type(seed) == "table") and seed or {}
    flush()

    -- SSE própria desta janela no endpoint dedicado (o motor liga streams abertas
    -- no init; o handle não precisa ser guardado — a stream vive enquanto a
    -- janela/motor existir).
    local opts: any = { on_message = "lw_on_state" }
    if ctx.api_token and ctx.api_token ~= "" then
        opts.headers = { Authorization = "Bearer " .. ctx.api_token }
    end
    sse((ctx.api_url or "") .. (ctx.lw_stream_url or ""), opts)
end
