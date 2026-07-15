--!strict
--!nolint FunctionUnused
-- handlers/settings.luau — Settings (Web Server) e Settings → Git (provedores
-- Gitea: conectar via OAuth/PAT, atualizar lista, remover).

local State = require("../state")
local Helpers = require("../helpers")
local fmt = require("../fmt")
local Stream = require("stream")

-- ── Settings (Web Server) ────────────────────────────────────────────────

function settings_tab(t: string): ()
    ctx.settings_tab = t
end

function settings_save(): ()
    local api = State.api
    if not api then
        return
    end
    local email = Helpers.trim(ctx.ss_email)
    local registry_domain = Helpers.trim(ctx.ss_registry_domain)
    ctx.settings_msg = "salvando…"
    local ok, res = api:rpc_checked{ SetDaemonSettings = {
        acme_email = (email ~= "" and email or nil),
        registry_domain = (registry_domain ~= "" and registry_domain or nil),
    } }
    ctx.settings_msg = ok and "configurações salvas" or ("erro: " .. res)
    if ok then
        toast{ message = tostring("Configurações salvas"), kind = "success" }
    else
        toast{ message = tostring(res), kind = "error" }
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
        local oko, reso = api:rpc{ GitOAuthStart = { provider_id = pid } }
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
    local ok, res = api:rpc_checked{ GitProviderDelete = { id = id } }
    ctx.gp_msg = ok and "provider removido" or ("erro: " .. res)
    if not ok then
        toast{ message = tostring(res), kind = "error" }
    end
    gp_refresh_list()
end

-- ── Settings → Infra as Code (export/import declarativo) ─────────────────

function iac_export(): ()
    local api = State.api
    if not api then
        return
    end
    ctx.iac_export_msg = "exportando…"
    local ok, res = api:rpc_checked("ManifestExportAll")
    if not ok or type(res) ~= "table" or not res.ManifestBundle then
        ctx.iac_export_msg = "erro: " .. tostring(res)
        toast{ message = tostring(tostring(res)), kind = "error" }
        return
    end
    ctx.iac_yaml = res.ManifestBundle.yaml
    ctx.iac_dotenv = res.ManifestBundle.dotenv
    ctx.iac_has_export = "true"
    ctx.iac_export_msg = "exportado ✓"
end

function iac_import(): ()
    local api = State.api
    if not api then
        return
    end
    ctx.iac_has_missing = "false"
    ctx.iac_has_report = "false"
    ctx.iac_missing_vars = ""
    ctx.iac_report_lines = {}

    local yaml = ctx.iac_import_yaml or ""
    if Helpers.trim(yaml) == "" then
        ctx.iac_import_msg = "cole o YAML do manifesto"
        return
    end

    ctx.iac_import_msg = "importando…"
    local ok, res = api:rpc{
        ManifestImport = {
            yaml = yaml,
            dotenv = ctx.iac_import_dotenv or "",
            prune = ctx.iac_prune == "true",
            deploy = ctx.iac_deploy == "true",
        },
    }
    if not ok then
        ctx.iac_import_msg = "erro: " .. tostring(res)
        toast{ message = tostring(tostring(res)), kind = "error" }
        return
    end

    if type(res) == "table" and res.MissingEnvVars ~= nil then
        ctx.iac_has_missing = "true"
        ctx.iac_missing_vars = table.concat(res.MissingEnvVars, ", ")
        ctx.iac_import_msg = "faltam variáveis — nada foi aplicado"
        toast{ message = tostring("Faltam variáveis no .env: " .. ctx.iac_missing_vars), kind = "error" }
        return
    end

    if type(res) == "table" and res.Err ~= nil then
        ctx.iac_import_msg = "erro: " .. tostring(res.Err.code) .. ": " .. tostring(res.Err.message)
        toast{ message = tostring(tostring(res.Err.message)), kind = "error" }
        return
    end

    if type(res) ~= "table" or not res.ManifestReport then
        ctx.iac_import_msg = "resposta inesperada do daemon"
        return
    end

    local lines = {}
    for _, a in ipairs(res.ManifestReport.actions or {}) do
        table.insert(lines, "[" .. tostring(a.action) .. "] " .. tostring(a.kind) .. " " .. tostring(a.name))
    end
    if #(res.ManifestReport.deployed or {}) > 0 then
        table.insert(lines, "deploy disparado: " .. table.concat(res.ManifestReport.deployed, ", "))
    end
    ctx.iac_report_lines = lines
    ctx.iac_has_report = "true"
    ctx.iac_import_msg = "import concluído ✓"
    toast{ message = tostring("Infra as Code importado"), kind = "success" }
    Stream.refresh_now()
end
