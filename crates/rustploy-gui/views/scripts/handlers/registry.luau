--!strict
--!nolint FunctionUnused
-- handlers/registry.luau — sub-aba Docker > Registry: navega repo→tags (fetch
-- sob demanda, tags não vêm no snapshot periódico) e remove tag/repositório.
-- Registry só existe pela Fase 1 do backend (push/pull via docker CLI); esta
-- tela é somente leitura + delete — sem criar/editar nada.

local State = require("../state")
local fmt = require("../fmt")
local Stream = require("stream")

-- Abre a visão de tags de um repositório: fetch on-demand (tags não vêm no
-- snapshot periódico, só a lista de repos).
function registry_open_repo(name: string): ()
    local api = State.api
    if not api then
        return
    end
    ctx.registry_selected_repo = name
    ctx.registry_tags_loading = "true"
    local ok, res = api:rpc_checked({ RegistryTagList = { repo = name } })
    if ok and res.RegistryTags then
        ctx.registry_tags = fmt.registry_tags(res.RegistryTags)
        ctx.registry_tags_count = tostring(#res.RegistryTags)
        ctx.registry_msg = ""
    else
        ctx.registry_tags = "[]"
        ctx.registry_tags_count = "0"
        ctx.registry_msg = "erro: " .. tostring(res)
    end
    ctx.registry_tags_loading = "false"
end

function registry_close_repo(): ()
    ctx.registry_selected_repo = ""
    ctx.registry_tags = "[]"
    ctx.registry_tags_count = "0"
end

-- ── Remoção ──────────────────────────────────────────────────────────────
-- Mesmo padrão confirm→do_* de handlers/docker.luau. `tag` vem no sufixo da
-- ação; o repositório é lido de ctx.registry_selected_repo (a tag só é
-- visível dentro de um repo já aberto).

function registry_rm_tag(tag: string): ()
    confirm({
        title = "Remover tag?",
        message = "A tag será removida. Se outra tag apontar pro mesmo manifest (mesma imagem), ela também é removida — o delete é por digest, não por tag. Blobs no disco só são liberados num GC futuro (ainda não implementado). Ação irreversível.",
        confirm_label = "Remover",
        confirm_action = "do_registry_rm_tag:" .. tostring(tag),
        destructive = true,
    })
end

function do_registry_rm_tag(tag: string): ()
    local api = State.api
    local repo = ctx.registry_selected_repo
    if not api or not repo or repo == "" then
        return
    end
    ctx.registry_msg = "removendo tag…"
    local ok, res = api:rpc_checked({ RegistryTagDelete = { repo = repo, tag = tag } })
    if ok then
        ctx.registry_msg = "removida"
        registry_open_repo(repo)
        Stream.refresh_now()
    else
        ctx.registry_msg = "erro: " .. tostring(res)
    end
end

function registry_rm_repo(name: string): ()
    confirm({
        title = "Remover repositório?",
        message = "Todas as tags e manifests do repositório serão removidos. Blobs no disco só são liberados num GC futuro (ainda não implementado). Ação irreversível.",
        confirm_label = "Remover",
        confirm_action = "do_registry_rm_repo:" .. tostring(name),
        destructive = true,
    })
end

function do_registry_rm_repo(name: string): ()
    local api = State.api
    if not api then
        return
    end
    ctx.registry_msg = "removendo repositório…"
    local ok, res = api:rpc_checked({ RegistryRepoDelete = { repo = name } })
    if ok then
        ctx.registry_msg = "removido"
        if ctx.registry_selected_repo == name then
            registry_close_repo()
        end
        Stream.refresh_now()
    else
        ctx.registry_msg = "erro: " .. tostring(res)
    end
end
