--!strict
--!nolint FunctionUnused
-- handlers/docker.luau — limpeza de recursos Docker sem uso (imagens, volumes,
-- redes). A aba se atualiza no próximo snapshot (2s).

local State = require("../state")
local fmt = require("../fmt")

local function prune(cmd: any, label: string): ()
    local api = State.api
    if not api then return end
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
