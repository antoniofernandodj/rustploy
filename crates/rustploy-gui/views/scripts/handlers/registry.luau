--!strict
--!nolint FunctionUnused
-- handlers/registry.luau — sub-aba Docker > Registry: navega repo→tags (fetch
-- sob demanda, tags não vêm no snapshot periódico), remove tag/repositório e
-- executa o GC. Registry só existe pela Fase 1 do backend (push/pull via
-- docker CLI); esta tela não cria/edita nada.

local State = require("../state")
local fmt = require("../fmt")
local Stream = require("stream")

local M = {}

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
        message = "A tag será removida. Se outra tag apontar pro mesmo manifest (mesma imagem), ela também é removida — o delete é por digest, não por tag. O disco é liberado pelo botão \"Executar GC\". Ação irreversível.",
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
        message = "Todas as tags e manifests do repositório serão removidos. O disco é liberado pelo botão \"Executar GC\". Ação irreversível.",
        confirm_label = "Remover",
        confirm_action = "do_registry_rm_repo:" .. tostring(name),
        destructive = true,
    })
end

-- ── GC ───────────────────────────────────────────────────────────────────
-- Libera do disco o que nenhuma tag alcança: manifests pendurados (tag
-- republicada), blobs órfãos de remoções anteriores e uploads abandonados.
-- Imagens taggeadas nunca são afetadas. Também roda sozinho 1x/dia no daemon.

function registry_gc(): ()
    confirm({
        title = "Executar GC?",
        message = "Remove do disco blobs e manifests que nenhuma tag referencia (sobras de remoções e de tags republicadas) e uploads abandonados há mais de 24h. Imagens com tag não são afetadas.",
        confirm_label = "Executar",
        confirm_action = "do_registry_gc",
        destructive = true,
    })
end

function do_registry_gc(): ()
    local api = State.api
    if not api then
        return
    end
    ctx.registry_msg = "executando GC…"
    local ok, res = api:rpc_checked("RegistryGc")
    if ok and res.RegistryGcResult then
        ctx.registry_msg = string.format("GC: %d arquivo(s) removido(s) · %s liberados",
            res.RegistryGcResult.blobs_removed or 0,
            fmt.fmt_bytes(res.RegistryGcResult.bytes_freed or 0))
        Stream.refresh_now()
    else
        ctx.registry_msg = "erro: " .. tostring(res)
    end
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

-- ── Tokens de acesso (Basic auth) ────────────────────────────────────────
-- Lista não vem no snapshot (tela de baixo tráfego) — buscada ao entrar na
-- sub-aba (ver handlers/nav.luau::docker_tab) e reatualizada após criar/
-- revogar um token.

function registry_refresh_tokens(): ()
    local api = State.api
    if not api then
        return
    end
    local ok, res = api:rpc("RegistryTokenList")
    if ok and res.RegistryTokens then
        ctx.registry_tokens = fmt.registry_tokens(res.RegistryTokens)
        ctx.registry_tokens_count = tostring(#res.RegistryTokens)
    end
end
M.refresh_tokens = registry_refresh_tokens

-- Abre a janela separada de criação (segredo só aparece lá, uma vez — ver
-- new_registry_token_window.xml/.luau).
function registry_open_token_window(): ()
    open_window({
        file = "crates/rustploy-gui/views/new_registry_token_window.xml",
        title = "Novo token — Rustploy",
        width = 480,
        height = 420,
        resizable = false,
        data = {
            api_url = ctx.api_url or "",
            api_token = ctx.api_token or "",
            registry_host = ctx.registry_host or "127.0.0.1:5100",
        },
    })
end

function registry_rm_token(name: string): ()
    confirm({
        title = "Revogar token?",
        message = "Qualquer `docker login`/push/pull usando esse token deixa de funcionar imediatamente. Ação irreversível.",
        confirm_label = "Revogar",
        confirm_action = "do_registry_rm_token:" .. tostring(name),
        destructive = true,
    })
end

function do_registry_rm_token(name: string): ()
    local api = State.api
    if not api then
        return
    end
    ctx.registry_msg = "revogando token…"
    local ok, res = api:rpc_checked({ RegistryTokenRevoke = { name = name } })
    if ok then
        ctx.registry_msg = "token revogado"
        registry_refresh_tokens()
    else
        ctx.registry_msg = "erro: " .. tostring(res)
    end
end

return M
