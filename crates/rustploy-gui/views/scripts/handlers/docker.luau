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
    confirm({
        title = "Limpar containers parados?",
        message = "Todos os containers parados serão removidos (docker container prune). Ação irreversível.",
        confirm_label = "Limpar",
        confirm_action = "do_docker_prune_containers",
        destructive = true,
    })
end

function do_docker_prune_containers(): ()
    prune("PruneContainers", "removendo containers parados…")
end

function docker_prune_images(): ()
    confirm({
        title = "Limpar imagens sem uso?",
        message = "As imagens Docker sem uso serão removidas agora. Essa limpeza não pode ser desfeita.",
        confirm_label = "Limpar",
        confirm_action = "do_docker_prune_images",
        destructive = true,
    })
end

function do_docker_prune_images(): ()
    local all = ctx.docker_prune_all_images == "true"
    prune({ PruneImages = { all = all } },
        all and "removendo TODAS as imagens sem uso…" or "removendo imagens dangling…")
end

function docker_prune_volumes(): ()
    confirm({
        title = "Limpar volumes sem uso?",
        message = "Os volumes Docker sem uso serão removidos agora. Essa limpeza não pode ser desfeita.",
        confirm_label = "Limpar",
        confirm_action = "do_docker_prune_volumes",
        destructive = true,
    })
end

function do_docker_prune_volumes(): ()
    local all = ctx.docker_prune_all_volumes == "true"
    prune({ PruneVolumes = { all = all } }, "removendo volumes sem uso…")
end

function docker_prune_networks(): ()
    confirm({
        title = "Limpar redes sem uso?",
        message = "As redes Docker sem uso serão removidas agora. Essa limpeza não pode ser desfeita.",
        confirm_label = "Limpar",
        confirm_action = "do_docker_prune_networks",
        destructive = true,
    })
end

function do_docker_prune_networks(): ()
    prune("PruneNetworks", "removendo redes sem uso…")
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

-- Cada par confirm→do_* espelha o padrão de delete_deployment: o id/name vem no
-- sufixo da ação (nome:sufixo), repassado ao handler como argumento.
function docker_rm_container(id: string): ()
    confirm({
        title = "Remover container?",
        message = "O container parado será removido (docker rm). Ação irreversível.",
        confirm_label = "Remover",
        confirm_action = "do_docker_rm_container:" .. tostring(id),
        destructive = true,
    })
end
function do_docker_rm_container(id: string): ()
    remove_one({ RemoveContainer = { id = id } }, "removendo container…")
end

function docker_rm_image(id: string): ()
    confirm({
        title = "Remover imagem?",
        message = "A imagem será removida (docker rmi). Falha se algum container a referencia.",
        confirm_label = "Remover",
        confirm_action = "do_docker_rm_image:" .. tostring(id),
        destructive = true,
    })
end
function do_docker_rm_image(id: string): ()
    remove_one({ RemoveImage = { id = id } }, "removendo imagem…")
end

function docker_rm_volume(name: string): ()
    confirm({
        title = "Remover volume?",
        message = "O volume será removido (docker volume rm). Falha se estiver em uso. Ação irreversível.",
        confirm_label = "Remover",
        confirm_action = "do_docker_rm_volume:" .. tostring(name),
        destructive = true,
    })
end
function do_docker_rm_volume(name: string): ()
    remove_one({ RemoveVolume = { name = name } }, "removendo volume…")
end

function docker_rm_network(id: string): ()
    confirm({
        title = "Remover rede?",
        message = "A rede será removida (docker network rm). Falha se houver container anexado.",
        confirm_label = "Remover",
        confirm_action = "do_docker_rm_network:" .. tostring(id),
        destructive = true,
    })
end
function do_docker_rm_network(id: string): ()
    remove_one({ RemoveNetwork = { id = id } }, "removendo rede…")
end
