--!strict
-- fmt/util.luau — busca, codificação de array e mapas de estado (paleta =
-- view.rs). Usado pelos demais submódulos fmt/*.luau.

local Types = require("types")
type ServicePair = Types.ServicePair
type ServiceSpec = Types.ServiceSpec
type DomainRoute = Types.DomainRoute

local M = {}

-- ── Busca ────────────────────────────────────────────────────────────────
-- Termo vazio sempre casa (a busca é filtro, não requisito). `term` já deve
-- vir em minúsculas.
function M.matches(term: string?, fields: {any}): boolean
    if not term or term == "" then return true end
    for _, f in ipairs(fields) do
        if type(f) == "string" and string.find(string.lower(f), term, 1, true) then
            return true
        end
    end
    return false
end

-- ── Texto ────────────────────────────────────────────────────────────────
-- Trunca `s` para no máximo `max` caracteres, anexando "…" quando corta. Usado
-- em valores longos (ex.: URL de conexão) que senão empurram os controles ao
-- lado (o botão "Copiar") para fora da área visível. O valor completo continua
-- disponível noutra chave de contexto para o clipboard.
function M.ellipsis(s: string?, max: number): string
    local str = s or ""
    if utf8.len(str) and (utf8.len(str) :: number) > max then
        local cut = utf8.offset(str, max + 1)
        if cut then
            return string.sub(str, 1, cut - 1) .. "…"
        end
    end
    return str
end

-- Remove sequências de escape ANSI (cor/cursor/erase) das linhas de log. O
-- daemon repassa o stdout/stderr cru dos containers, cheio de CSI (`ESC[..m`
-- etc.); sem limpar, além de aparecerem como lixo (`[2m[32m…`) na UI, cada
-- linha vira uma string longa com bytes de controle que o text-shaping do iced
-- processa a um custo patológico — reconstruído a CADA re-render (por evento de
-- log), o que congela a janela. Também tira CR e NUL soltos.
function M.strip_ansi(s: string?): string
    local str = s or ""
    -- CSI: ESC [ params byte-final(letra) — cobre SGR (cor), erase (K/J), cursor.
    str = string.gsub(str, "\27%[[%d;?]*%a", "")
    -- OSC: ESC ] … BEL. E qualquer ESC + 1 byte remanescente (ESC c, ESC =, …).
    str = string.gsub(str, "\27%][^\7]*\7", "")
    str = string.gsub(str, "\27.", "")
    str = string.gsub(str, "[\r\0]", "")
    return str
end

-- ── Codificação de arrays ────────────────────────────────────────────────
-- CUIDADO: uma tabela Luau vazia vira `{}` (objeto) no json.encode, não `[]`.
-- Os templates iteram arrays, então forçamos "[]" quando vazio.
function M.enc_array(t: {any}): string
    if #t == 0 then return "[]" end
    return json.encode(t)
end

-- ── Mapas de estado (paleta = view.rs) ───────────────────────────────────

-- ServiceStatus vem como string ("Running") ou tabela ({Error="..."}).
function M.status_label_color(status: any): (string, string)
    if type(status) == "table" then
        if status.Error ~= nil then return "Error", "#F85149" end
    end
    if status == "Running" then return "Running", "#3FB950"
    elseif status == "Deploying" then return "Deploying", "#58A6FF"
    elseif status == "Degraded" then return "Degraded", "#D29922"
    elseif status == "Stopping" then return "Stopping", "#8B949E"
    elseif status == "Stopped" then return "Stopped", "#8B949E"
    end
    return "Error", "#F85149"
end

function M.status_kind(status: any): string
    if type(status) == "table" and status.Error ~= nil then return "bad" end
    if status == "Running" then return "ok"
    elseif status == "Deploying" then return "info"
    elseif status == "Degraded" then return "warn"
    elseif status == "Stopping" or status == "Stopped" then return "muted"
    end
    return "bad"
end

-- DeployState vem como string ("Live", "Failed", "BuildingImage", ...).
function M.state_label_color(state: string?): (string, string)
    if state == "Live" then return "LIVE", "#3FB950"
    elseif state == "Stopped" then return "STOPPED", "#8B949E"
    elseif state == "Failed" then return "FAILED", "#F85149"
    end
    return "BUILDING", "#58A6FF"
end

function M.state_kind(state: string?): string
    if state == "Live" then return "ok"
    elseif state == "Stopped" then return "muted"
    elseif state == "Failed" then return "bad"
    end
    return "info"
end

function M.in_use_kind(in_use: boolean): string
    if in_use then return "ok" end
    return "muted"
end

-- Estado bruto de um container Docker ("running"/"exited"/… ) → (rótulo, kind).
-- Só "running"/"restarting"/"paused" contam como "ativo" (não removíveis sem
-- force); o resto é parado e pode ser removido individualmente.
function M.container_state(state: string?): (string, string)
    local s = string.lower(state or "")
    if s == "running" then return "running", "ok"
    elseif s == "restarting" then return "restarting", "info"
    elseif s == "paused" then return "paused", "warn"
    elseif s == "created" then return "created", "info"
    elseif s == "dead" then return "dead", "bad"
    end
    -- exited, removing, ou desconhecido
    return (s ~= "" and s or "exited"), "muted"
end

-- `true` quando o container está PARADO (removível sem force). Espelha o daemon
-- (Docker recusa `rm` de um container rodando/reiniciando sem `-f`).
function M.container_stopped(state: string?): boolean
    local s = string.lower(state or "")
    return s ~= "running" and s ~= "restarting" and s ~= "paused"
end

-- ServiceSource → texto de "imagem/origem" (source_summary().1).
function M.source_summary(source: any): string
    if type(source) ~= "table" then return "—" end
    if source.Registry then return source.Registry.image or "—"
    elseif source.Git then
        local g = source.Git
        return (g.url or "") .. " @ " .. (g.branch or "")
    elseif source.Compose then return "docker-compose"
    end
    return "—"
end

-- domain_routes(): a lista `domains` nova, ou o legado `domain`/`tls_enabled`.
function M.domain_routes(spec: ServiceSpec): {DomainRoute}
    local routes: {DomainRoute} = {}
    if spec.domains and #spec.domains > 0 then
        for _, r in ipairs(spec.domains) do
            table.insert(routes, { domain = r.domain, port = r.port, tls = r.tls == true })
        end
    elseif spec.domain and spec.domain ~= "" then
        table.insert(routes, { domain = spec.domain :: string, port = nil, tls = spec.tls_enabled == true })
    end
    return routes
end

-- `services` do snapshot é uma lista de { project_name, service }.
-- Normaliza para pares { svc = Service, proj = "nome" }.
function M.pair_list(services: {any}?): {ServicePair}
    local out: {ServicePair} = {}
    for _, e in ipairs(services or {}) do
        table.insert(out, { svc = e.service, proj = e.project_name or "" })
    end
    return out
end

return M
