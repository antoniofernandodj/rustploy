--!strict
--!nolint FunctionUnused
-- handlers/settings.luau — Settings (Web Server) e Settings → Git (provedores
-- Gitea: conectar via OAuth/PAT, atualizar lista, remover).

local State = require("../state")
local Helpers = require("../helpers")
local fmt = require("../fmt")
local Stream = require("stream")

local M = {}

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

-- ── Settings → Git (provedores Gitea / GitHub) ───────────────────────────

function gp_mode(m: string): ()
    ctx.gp_mode = m
end

-- Alterna o tipo de provedor (gitea | github) e recomputa a redirect URI, que
-- tem path por provedor (/oauth/gitea/callback vs /oauth/github/callback).
function gp_kind(k: string): ()
    local kind = (k == "github") and "github" or "gitea"
    ctx.gp_kind = kind
    ctx.gp_redirect = Helpers.oauth_redirect_uri(ctx.ss_public_base, kind)
    -- GitHub conecta ao github.com por padrão; Base URL só p/ Enterprise.
    if kind == "github" and Helpers.trim(ctx.gp_base_url) == "" then
        ctx.gp_name = (Helpers.trim(ctx.gp_name) == "" and "GitHub" or ctx.gp_name)
    end
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
    local is_github = (ctx.gp_kind or "gitea") == "github"
    local kind_wire = is_github and "Github" or "Gitea"
    local label = is_github and "GitHub" or "Gitea"
    local base = Helpers.trim(ctx.gp_base_url)
    -- GitHub conecta ao github.com por padrão; a Base URL só é preenchida para
    -- GitHub Enterprise Server. Gitea sempre exige a Base URL.
    if base == "" then
        if is_github then
            base = "https://github.com"
        else
            ctx.gp_msg = "informe a Base URL do Gitea";
            return
        end
    end
    local name = Helpers.trim(ctx.gp_name); if name == "" then
        name = label
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
        cmd = { GitProviderCreate = { kind = kind_wire, name = name, base_url = base,
            auth_mode = "OAuth", oauth_client_id = cid, oauth_client_secret = csec, pat = nil } }
    else
        local pat = ctx.gp_pat or ""
        if Helpers.trim(pat) == "" then
            ctx.gp_msg = "informe o Personal Access Token";
            return
        end
        cmd = {
            GitProviderCreate = {
                kind = kind_wire,
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
        ctx.gp_msg = "conta " .. label .. " conectada ✓"
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
        toast{ message = tostring("Faltam variáveis no TOML: " .. ctx.iac_missing_vars), kind = "error" }
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

-- ── Settings → Manutenção (limpeza automática de Docker) ─────────────────
-- Ver docs/plano-limpeza-automatica-docker.md. Config é um singleton do
-- daemon (não uma lista) — sem tabela própria, carregada/salva inteira a
-- cada Get/Set. `dc_last_run_at_raw`/`dc_next_run_at_raw` são campos "mudos"
-- (não exibidos, só round-trip): preservam o `last_run_at` real entre saves
-- pra um agendamento `IntervalHours` não reiniciar a contagem toda vez que o
-- usuário aperta "Save" sem ter mudado nada de agendamento.

-- Recorrência (Option<Recurrence>, externally-tagged) → (kind, hours, hour,
-- minute, weekday) — mesmo formato de new_job_window.luau::unpack_recurrence,
-- mas aqui a recorrência nunca é `nil` (a config sempre tem uma, mesmo com o
-- interruptor geral desligado).
local function dc_unpack_recurrence(r: any): (string, string, string, string, string)
    if r ~= nil and r.IntervalHours ~= nil then
        return "interval", tostring(r.IntervalHours), "3", "0", "0"
    end
    if r ~= nil and r.Weekly ~= nil then
        return "weekly", "6", tostring(r.Weekly.hour), tostring(r.Weekly.minute), tostring(r.Weekly.weekday)
    end
    if r ~= nil and r.Daily ~= nil then
        return "daily", "6", tostring(r.Daily.hour), tostring(r.Daily.minute), "0"
    end
    return "daily", "6", "3", "0", "0"
end

local function dc_apply_config(cfg: any): ()
    ctx.dc_enabled = tostring(cfg.enabled)
    ctx.dc_containers = tostring(cfg.containers)
    ctx.dc_images = tostring(cfg.images)
    ctx.dc_images_all = tostring(cfg.images_all)
    ctx.dc_volumes = tostring(cfg.volumes)
    ctx.dc_volumes_all = tostring(cfg.volumes_all)
    ctx.dc_networks = tostring(cfg.networks)
    ctx.dc_build_cache = tostring(cfg.build_cache)
    local kind, hours, hour, minute, weekday = dc_unpack_recurrence(cfg.recurrence)
    ctx.dc_kind = kind
    ctx.dc_hours = hours
    ctx.dc_hour = hour
    ctx.dc_minute = minute
    ctx.dc_weekday = weekday
    ctx.dc_next_run_label = cfg.next_run_at and fmt.date_dm_hm(cfg.next_run_at) or "—"
    State.dc_last_run_at_raw = cfg.last_run_at
end

-- Carregado uma vez na conexão (handlers/connection.luau::load_settings) —
-- não faz parte do snapshot de 2s, é config, não estado que muda sozinho
-- (exceto por `last_run`, atualizado ao vivo por `Event::DockerCleanupCompleted`
-- em stream.luau, sem precisar buscar de novo).
function M.dc_load(): ()
    local api = State.api
    if not api then
        return
    end
    local ok, res = api:rpc("DockerCleanupConfigGet")
    if ok and res.DockerCleanupConfig then
        dc_apply_config(res.DockerCleanupConfig.config)
        ctx.dc_last_run_text = fmt.docker_cleanup_last_run_summary(res.DockerCleanupConfig.last_run)
    end
end

function dc_kind(k: string): ()
    ctx.dc_kind = k
end

function dc_weekday(w: string): ()
    ctx.dc_weekday = w
end

local function dc_build_recurrence(): any
    if ctx.dc_kind == "interval" then
        return { IntervalHours = math.max(1, tonumber(ctx.dc_hours) or 1) }
    elseif ctx.dc_kind == "weekly" then
        return {
            Weekly = {
                weekday = tonumber(ctx.dc_weekday) or 0,
                hour = tonumber(ctx.dc_hour) or 0,
                minute = tonumber(ctx.dc_minute) or 0,
            },
        }
    end
    return {
        Daily = {
            hour = tonumber(ctx.dc_hour) or 0,
            minute = tonumber(ctx.dc_minute) or 0,
        },
    }
end

function dc_save(): ()
    local api = State.api
    if not api then
        return
    end
    ctx.dc_msg = "salvando…"
    local ok, res = api:rpc_checked{
        DockerCleanupConfigSet = {
            config = {
                enabled = ctx.dc_enabled == "true",
                recurrence = dc_build_recurrence(),
                containers = ctx.dc_containers == "true",
                images = ctx.dc_images == "true",
                images_all = ctx.dc_images_all == "true",
                volumes = ctx.dc_volumes == "true",
                volumes_all = ctx.dc_volumes_all == "true",
                networks = ctx.dc_networks == "true",
                build_cache = ctx.dc_build_cache == "true",
                last_run_at = State.dc_last_run_at_raw,
            },
        },
    }
    if ok and res.DockerCleanupConfig then
        dc_apply_config(res.DockerCleanupConfig.config)
        ctx.dc_last_run_text = fmt.docker_cleanup_last_run_summary(res.DockerCleanupConfig.last_run)
        ctx.dc_msg = "configurações salvas"
        toast{ message = tostring("Limpeza automática salva"), kind = "success" }
    else
        ctx.dc_msg = "erro: " .. tostring(res)
        toast{ message = tostring(res), kind = "error" }
    end
end

-- Botão "Executar agora": roda os recursos marcados fora do horário
-- agendado, independente do interruptor geral (é um botão de teste). Roda em
-- background no daemon — `ctx.dc_running` volta a "false" quando
-- `Event::DockerCleanupCompleted` chega (stream.luau::apply_bus), não pela
-- resposta deste RPC.
function dc_run_now(): ()
    local api = State.api
    if not api then
        return
    end
    local any_selected = ctx.dc_containers == "true" or ctx.dc_images == "true"
        or ctx.dc_volumes == "true" or ctx.dc_networks == "true" or ctx.dc_build_cache == "true"
    if not any_selected then
        ctx.dc_msg = "marque pelo menos um recurso"
        return
    end
    local ok_confirm = confirm{
        title = "Executar limpeza agora?",
        message = "Roda a limpeza dos recursos marcados imediatamente, fora do horário agendado. Ação irreversível.",
        confirm_label = "Executar",
        destructive = true,
    }
    if not ok_confirm then
        return
    end
    ctx.dc_running = "true"
    ctx.dc_msg = "executando…"
    local ok, res = api:rpc_checked("DockerCleanupRunNow")
    if not ok then
        ctx.dc_running = "false"
        ctx.dc_msg = "erro: " .. tostring(res)
        toast{ message = tostring(res), kind = "error" }
    end
end

return M
