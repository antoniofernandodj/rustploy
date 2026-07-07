--!strict
-- fmt/git.luau — builders dos provedores/repositórios/branches Gitea
-- conectados (Settings → Git e o picker da aba General do serviço).

local Util = require("util")

local M = {}

function M.git_providers(list: {any}?): string
    local rows = {}
    for _, p in ipairs(list or {}) do
        local login = p.account and p.account.login or nil
        local connected = login ~= nil
        local display: string
        local account_lbl: string
        local account_login: string
        if connected then
            display = tostring(p.name or "") .. " (@" .. tostring(login or "") .. ")"
            account_lbl = "@" .. tostring(login or "")
            account_login = tostring(login or "")
        else
            display = tostring(p.name or "") .. " — não conectado"
            account_lbl = "(pendente — autorize no navegador)"
            account_login = "não conectado"
        end
        local auth_mode = (p.auth_mode == "OAuth") and "OAuth2" or "PAT"
        table.insert(rows, {
            id = p.id,
            name = p.name,
            display = display,
            base_url = p.base_url,
            auth_mode = auth_mode,
            account = account_lbl,
            account_login = account_login,
            connected = connected and "true" or "false",
        })
    end
    return Util.enc_array(rows)
end

function M.git_repos(list: {any}?): string
    local rows = {}
    for _, r in ipairs(list or {}) do
        table.insert(rows, {
            full_name = r.full_name,
            clone_url = r.clone_url,
            default_branch = r.default_branch,
            private = r.private and "private" or "public",
        })
    end
    return Util.enc_array(rows)
end

function M.git_branches(list: {any}?): string
    local rows = {}
    for _, b in ipairs(list or {}) do
        table.insert(rows, { name = b.name })
    end
    return Util.enc_array(rows)
end

return M
