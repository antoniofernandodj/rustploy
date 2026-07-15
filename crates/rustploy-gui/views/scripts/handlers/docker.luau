--!strict
--!nolint FunctionUnused
-- handlers/docker.luau — limpeza de recursos Docker sem uso (imagens, volumes,
-- redes). A aba se atualiza no próximo snapshot (2s).

local State = require("../state")
local fmt = require("../fmt")
local Stream = require("stream")

local function prune(cmd: any, label: string): ()
    local api = State.api
    if not api then
        return
    end
    ctx.docker_msg = label
    local ok, res = api:rpc_checked(cmd)
    if ok and res.PruneResult then
        ctx.docker_msg = string.format("removidos: %d · %s liberados",
            res.PruneResult.count or 0, fmt.fmt_bytes(res.PruneResult.reclaimed_bytes))
    elseif ok then
        ctx.docker_msg = "limpeza concluída"
    else
        ctx.docker_msg = "erro: " .. res
    end
end

function docker_prune_containers(): ()
    local ok = confirm({
        title = "Limpar containers parados?",
        message = "Todos os containers parados serão removidos (docker container prune). Ação irreversível.",
        confirm_label = "Limpar",
        destructive = true,
    })

    if ok then
        prune("PruneContainers", "removendo containers parados…")
    end
end


function docker_prune_images(): ()
    local ok = confirm({
        title = "Limpar imagens sem uso?",
        message = "As imagens Docker sem uso serão removidas agora. Essa limpeza não pode ser desfeita.",
        confirm_label = "Limpar",
        destructive = true,
    })

    if ok then
        local all = ctx.docker_prune_all_images == "true"
        prune({ PruneImages = { all = all } },
            all and "removendo TODAS as imagens sem uso…" or "removendo imagens dangling…")
    end
end



function docker_prune_volumes(): ()
    local ok = confirm({
        title = "Limpar volumes sem uso?",
        message = "Os volumes Docker sem uso serão removidos agora. Essa limpeza não pode ser desfeita.",
        confirm_label = "Limpar",
        destructive = true,
    })

    if ok then
        local all = ctx.docker_prune_all_volumes == "true"
        prune({ PruneVolumes = { all = all } }, "removendo volumes sem uso…")
    end
end



function docker_prune_networks(): ()
    local ok = confirm({
        title = "Limpar redes sem uso?",
        message = "As redes Docker sem uso serão removidas agora. Essa limpeza não pode ser desfeita.",
        confirm_label = "Limpar",
        destructive = true,
    })

    if ok then
        prune("PruneNetworks", "removendo redes sem uso…")
    end
end

-- ── Remoção INDIVIDUAL (por item) ───────────────────────────────────────────
-- Diferente do prune (em lote), remove UM recurso escolhido. Refresca na hora
-- (refresh_now) para a linha sumir sem esperar o snapshot de 2s. O Docker recusa
-- remover recursos em uso (sem force) — o erro volta e é exibido em docker_msg.

local function remove_one(cmd: any, doing: string): ()
    local api = State.api
    if not api then
        return
    end
    ctx.docker_msg = doing
    local ok, res = api:rpc_checked(cmd)
    if ok then
        ctx.docker_msg = "removido"
        Stream.refresh_now()
    else
        ctx.docker_msg = "erro: " .. tostring(res)
    end
end

-- Cada par confirm→do_* espelha o padrão de delete_deployment: o `confirm`
-- suspende e, se o usuário confirmar, chama o handler `do_*` na hora com o
-- id/name capturado no escopo.
function docker_rm_container(id: string): ()
    local ok = confirm({
        title = "Remover container?",
        message = "O container parado será removido (docker rm). Ação irreversível.",
        confirm_label = "Remover",
        destructive = true,
    })

    if ok then
        remove_one({ RemoveContainer = { id = id } }, "removendo container…")
    end
end

function docker_rm_image(id: string): ()
    local ok = confirm({
        title = "Remover imagem?",
        message = "A imagem será removida (docker rmi). Falha se algum container a referencia.",
        confirm_label = "Remover",
        destructive = true,
    })

    if ok then
        remove_one({ RemoveImage = { id = id } }, "removendo imagem…")
    end
end

function docker_rm_volume(name: string): ()
    local ok = confirm({
        title = "Remover volume?",
        message = "O volume será removido (docker volume rm). Falha se estiver em uso. Ação irreversível.",
        confirm_label = "Remover",
        destructive = true,
    })

    if ok then
        remove_one({ RemoveVolume = { name = name } }, "removendo volume…")
    end
end

function docker_rm_network(id: string): ()
    local ok = confirm({
        title = "Remover rede?",
        message = "A rede será removida (docker network rm). Falha se houver container anexado.",
        confirm_label = "Remover",
        destructive = true,
    })

    if ok then
        remove_one({ RemoveNetwork = { id = id } }, "removendo rede…")
    end
end
