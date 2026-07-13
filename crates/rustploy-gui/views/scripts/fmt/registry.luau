--!strict
-- fmt/registry.luau — formata repositórios/tags do registry OCI embutido pra
-- exibição na sub-aba Docker > Registry.

local Time = require("time")
local Util = require("util")

local M = {}

-- Lista de repositórios (filtrada pela busca global, como docker_images etc.).
function M.repos(list: {any}?, term: string?): string
    local rows = {}
    for _, r in ipairs(list or {}) do
        if Util.matches(term, { r.name }) then
            table.insert(rows, {
                name = r.name,
                tag_count = tostring(r.tag_count or 0),
                size = Time.fmt_bytes(r.size_bytes or 0),
                created = Time.date_dm_hm(r.created_at),
            })
        end
    end
    return Util.enc_array(rows)
end

-- Tags de UM repositório (sem filtro de busca — lista pequena, buscada sob
-- demanda ao abrir o repositório, não vem no snapshot periódico).
function M.tags(list: {any}?): string
    local rows = {}
    for _, t in ipairs(list or {}) do
        table.insert(rows, {
            tag = t.tag,
            digest = string.sub(t.digest, 1, 12),
            digest_full = t.digest,
            media_type = t.media_type,
            size = Time.fmt_bytes(t.size_bytes or 0),
            updated = Time.date_dm_hm(t.updated_at),
        })
    end
    return Util.enc_array(rows)
end

-- Tokens de acesso (Basic auth) — sem filtro de busca (lista tipicamente
-- pequena, buscada uma vez ao abrir a sub-aba, não vem no snapshot).
function M.tokens(list: {any}?): string
    local rows = {}
    for _, t in ipairs(list or {}) do
        table.insert(rows, {
            name = t.name,
            scope = t.scope,
            created = Time.date_dm_hm(t.created_at),
            last_used = t.last_used_at and Time.date_dm_hm(t.last_used_at) or "nunca",
        })
    end
    return Util.enc_array(rows)
end

return M
