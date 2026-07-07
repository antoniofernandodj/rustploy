--!strict
--!nolint FunctionUnused
-- handlers/settings.luau — Settings (Web Server) e Settings → Git (provedores
-- Gitea: conectar via OAuth/PAT, atualizar lista, remover).

local State = require("../state")
local Helpers = require("../helpers")
local fmt = require("../fmt")

-- ── Settings (Web Server) ────────────────────────────────────────────────

function ss_domain(v: string): ()
    ctx.ss_domain = v or ""
    ctx.gp_redirect = Helpers.oauth_redirect_uri(v or "")
end

function settings_tab(t: string): ()
    ctx.settings_tab = t
end

function settings_save(): ()
    local api = State.api
    if not api then
        return
    end
    local domain = Helpers.trim(ctx.ss_domain)
    local email = Helpers.trim(ctx.ss_email)
    ctx.settings_msg = "salvando…"
    local ok, res = api:rpc_checked({ SetDaemonSettings = {
        webhook_base_url = (domain ~= "" and domain or nil),
        acme_email = (email ~= "" and email or nil),
    } })
    ctx.settings_msg = ok and "configurações salvas" or ("erro: " .. res)
    if ok then
        Helpers.notify_ok("Configurações salvas")
    else
        Helpers.notify_err(res)
    end
end

-- ── Settings → Git (provedores Gitea) ────────────────────────────────────

function gp_mode(m: string): ()
    ctx.gp_mode = m
end

local function gp_refresh_list(): ()
    local api = State.api
    if not api then
        return
    end
    local ok, res = api:rpc("GitProviderList")
    if ok and res.GitProviders then
        ctx.gitea_providers = fmt.git_providers(res.GitProviders)
        ctx.gitea_count = tostring(#res.GitProviders)
    end
end

function gp_refresh(): ()
    if not State.api then
        return
    end
    gp_refresh_list()
end

function gp_connect(): ()
    local api = State.api
    if not api then
        return
    end
    ctx.gp_oauth_url = ""
    local base = Helpers.trim(ctx.gp_base_url)
    if base == "" then
        ctx.gp_msg = "informe a Base URL do Gitea";
        return
    end
    local name = Helpers.trim(ctx.gp_name); if name == "" then
        name = "Gitea"
    end
    local is_oauth = (ctx.gp_mode or "oauth") ~= "pat"
    local cmd: any
    if is_oauth then
        local cid = Helpers.trim(ctx.gp_client_id)
        local csec = ctx.gp_client_secret or ""
        if cid == "" or Helpers.trim(csec) == "" then
            ctx.gp_msg = "Client ID e Client Secret são obrigatórios";
            return
        end
        cmd = { GitProviderCreate = { kind = "Gitea", name = name, base_url = base,
            auth_mode = "OAuth", oauth_client_id = cid, oauth_client_secret = csec, pat = nil } }
    else
        local pat = ctx.gp_pat or ""
        if Helpers.trim(pat) == "" then
            ctx.gp_msg = "informe o Personal Access Token";
            return
        end
        cmd = {
            GitProviderCreate = {
                kind = "Gitea",
                name = name,
                base_url = base,
                auth_mode = "Pat",
                oauth_client_id = nil,
                oauth_client_secret = nil,
                pat = pat
            }
        }
    end
    ctx.gp_msg = "conectando…"
    local ok, res = api:rpc(cmd)
    if not ok or not res.GitProviderInfo then
        ctx.gp_msg = "erro: " .. (ok and "resposta inesperada" or res);
        return
    end
    local pid = res.GitProviderInfo.id
    if is_oauth then
        -- OAuth precisa de round-trip no navegador. O Luau não abre o browser;
        -- surface a URL para o usuário abrir (built-in open:<chave> no template)
        -- e depois clicar em "Atualizar lista".
        local oko, reso = api:rpc({ GitOAuthStart = { provider_id = pid } })
        if oko and reso.OAuthUrl then
            ctx.gp_oauth_url = reso.OAuthUrl
            ctx.gp_msg = "abra para autorizar: " .. reso.OAuthUrl
        else
            ctx.gp_msg = "provider criado; inicie o OAuth manualmente"
        end
    else
        ctx.gp_msg = "conta Gitea conectada ✓"
    end
    ctx.gp_name = ""
    ctx.gp_base_url = ""
    ctx.gp_client_id = ""
    ctx.gp_client_secret = ""
    ctx.gp_pat = ""
    gp_refresh_list()
end

function gp_delete(id: string): ()
    local api = State.api
    if not api then
        return
    end
    local ok, res = api:rpc_checked({ GitProviderDelete = { id = id } })
    ctx.gp_msg = ok and "provider removido" or ("erro: " .. res)
    if not ok then
        Helpers.notify_err(res)
    end
    gp_refresh_list()
end
