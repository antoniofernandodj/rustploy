--!strict
--!nolint FunctionUnused
-- handlers/deploy_queue.luau — gerência da fila global de deploys (um por vez)
-- na tela Deploy Engine: cancelar/promover um item enfileirado, reordenar por
-- arraste e pausar/retomar a fila inteira. Tudo via os Commands do daemon; o
-- refresh vem do snapshot (o daemon publica DeployQueueChanged → refresh_now).

local State = require("../state")
local Stream = require("stream")

-- Cancela um deploy que ainda está esperando na fila (não começou). Reaproveita
-- o DeployAbort — no daemon ele remove da fila e devolve o serviço ao repouso.
function queue_cancel(deployment_id: string): ()
    local api = State.api
    if not api or not deployment_id or deployment_id == "" then
        return
    end
    local ok, res = api:rpc_checked{ DeployAbort = { deployment_id = deployment_id } }
    if ok then
        toast{ message = tostring("Deploy removido da fila"), kind = "success" }
    else
        toast{ message = tostring(res), kind = "error" }
    end
    Stream.refresh_now()
end

-- Move um deploy enfileirado para o topo da fila ("furar fila").
function queue_promote(deployment_id: string): ()
    local api = State.api
    if not api or not deployment_id or deployment_id == "" then
        return
    end
    local ok, res = api:rpc_checked{ DeployQueuePromote = { deployment_id = deployment_id } }
    if not ok then
        toast{ message = tostring(res), kind = "error" }
    end
    Stream.refresh_now()
end

-- Reordena a fila (arraste): `v` é um JSON array de deployment_ids na nova ordem
-- (mesma forma que o env_reorder — o `reorder_key` do foreach é deployment_id).
function queue_reorder(v: string?): ()
    local api = State.api
    if not api or not v then
        return
    end
    local ok, order = pcall(json.decode, v)
    if not ok or type(order) ~= "table" then
        return
    end
    local ok2, res = api:rpc_checked{ DeployQueueReorder = { order = order } }
    if not ok2 then
        toast{ message = tostring(res), kind = "error" }
    end
    Stream.refresh_now()
end

-- Pausa/retoma a fila. `ctx.eng_paused` ("true"/"false") reflete o estado atual
-- vindo do snapshot; aqui só invertemos.
function queue_toggle_pause(): ()
    local api = State.api
    if not api then
        return
    end
    local paused = ctx.eng_paused ~= "true"
    local ok, res = api:rpc_checked{ DeployQueuePause = { paused = paused } }
    if ok then
        toast{ message = tostring(paused and "Fila pausada" or "Fila retomada"), kind = "success" }
    else
        toast{ message = tostring(res), kind = "error" }
    end
    Stream.refresh_now()
end
