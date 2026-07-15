--!strict
--!nolint FunctionUnused
-- log_window.luau — script da JANELA de logs (runtime OU build) de um serviço/
-- deployment, um motor Glacier próprio e ISOLADO do app principal (aberto via
-- open_window por handlers/services.luau: open_logs_window / dep_logs).
-- Genérico: a mesma janela serve os dois tipos, parametrizada por `data`:
--   api_url / api_token   — conexão com o daemon
--   lw_title              — título (ex.: "Logs · api" / "Build logs · dep_01KX…")
--   lw_stream_url         — path do SSE dedicado a seguir:
--                             "/api/services/<id>/logs"          (runtime)
--                             "/api/deployments/<id>/build-logs" (build)
--   lw_seed               — JSON do tail/histórico inicial (o pai já buscou)
--
-- O corpo é um <textarea> (text_editor): texto SELECIONÁVEL/copiável. Cada linha
-- nova é APPENDADA no fim via append_textarea (o motor insere no Content sem
-- recriá-lo — preserva scroll e custa O(linha)), em vez de reescrever o buffer
-- inteiro a cada linha (que apagaria a seleção e pularia pro topo). O SSE já só
-- emite quando HÁ linha nova, então não há atualização "de graça".
--
-- sse() no init() FUNCIONA (o motor liga as streams abertas no init); já
-- `append_textarea` NÃO (efeitos de init são descartados, como `fetch`), por isso
-- o histórico inicial vai por um SET completo (ctx.lw_logs_text) no init, e só as
-- linhas VIVAS (em lw_on_state, ação despachada) usam append_textarea.
--
-- require resolve relativo a ESTE arquivo (scripts/): "fmt" (fachada de fmt/*).

local fmt = require("fmt")

-- Buffer de eventos {stream?,line,timestamp} p/ reconstrução em corte (trim). O
-- <textarea> cresce por append; para não crescer sem limite numa sessão longa,
-- ao passar de RING_HIGH cortamos a cauda para RING_LOW e reconstruímos o
-- buffer UMA vez (raro — o intervalo largo evita reconstruir a cada linha).
local RING_HIGH = 6000
local RING_LOW = 4000
local buf: { any } = {}

-- Set COMPLETO do textarea a partir do buf (usado no seed e no corte). Mantém o
-- fim com "\n" para os appends seguintes começarem em linha nova.
local function set_full(): ()
    local text = fmt.join_log_lines(buf)
    ctx.lw_logs_text = (#buf > 0) and (text .. "\n") or ""
    ctx.lw_count = tostring(#buf)
end

-- Aplica um lote de eventos novos: appenda no fim (barato) ou, se estourou o
-- teto, corta a cauda e reconstrói (raro).
local function push_batch(events: { any }): ()
    if #events == 0 then
        return
    end
    for _, e in ipairs(events) do
        table.insert(buf, e)
    end
    if #buf > RING_HIGH then
        while #buf > RING_LOW do
            table.remove(buf, 1)
        end
        set_full()
    else
        append_textarea("lw_logs_text", fmt.join_log_lines(events) .. "\n")
        ctx.lw_count = tostring(#buf)
    end
end

-- Um evento do bus: os endpoints já filtram por id; pegamos o LogLine (runtime),
-- BuildLog (build) ou JobLogLine (job one-shot), o que vier. BuildLog não traz
-- `stream` (cor padrão).
local function to_row(ev: any): any?
    if type(ev) ~= "table" then
        return nil
    end
    return ev.LogLine or ev.BuildLog or ev.JobLogLine
end

-- Callback do sse(): cada `data:` é um JSON auto-descritivo (kind). O endpoint
-- dedicado manda lotes `bus_batch`; tratamos `bus` avulso por robustez.
function lw_on_state(data: string): ()
    local ok, msg = pcall(json.decode, data)
    if not ok or type(msg) ~= "table" then
        return
    end
    if msg.kind == "bus" then
        local row = to_row(msg.event)
        if row then
            push_batch{ row }
        end
    elseif msg.kind == "bus_batch" then
        local rows = {}
        for _, e in ipairs(msg.events or {}) do
            local row = to_row(e)
            if row then
                table.insert(rows, row)
            end
        end
        push_batch(rows)
    end
end

function init(): ()
    ctx.lw_title = ctx.lw_title or "Logs"

    -- Histórico inicial semeado pelo pai (SET completo — append_textarea de init
    -- seria descartado). Daí em diante, só appends vivos em lw_on_state.
    local oks, seed = pcall(json.decode, ctx.lw_seed or "[]")
    buf = (oks and type(seed) == "table") and seed or {}
    set_full()

    -- SSE própria desta janela no endpoint dedicado (o motor liga streams
    -- abertas no init; o handle não precisa ser guardado — vive com a janela).
    local opts: any = { on_message = "lw_on_state" }
    if ctx.api_token and ctx.api_token ~= "" then
        opts.headers = { Authorization = "Bearer " .. ctx.api_token }
    end
    sse((ctx.api_url or "") .. (ctx.lw_stream_url or ""), opts)
end
