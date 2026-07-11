--!strict
-- fmt/dashboard.luau — builders de lista do dashboard (deployments/projects/
-- services/docker/ingress/monitoring/deploy engine), mesma forma JSON que
-- view.rs produzia. Iterado pelos ForEach dos templates via a fachada fmt.luau.

local Types = require("types")
type ServicePair = Types.ServicePair
type MetricsMap = Types.MetricsMap
type GridCard = Types.GridCard

local Time = require("time")
local Util = require("util")

local M = {}

local GRID_COLS = 3

local FILLER: GridCard = {
    filler = "1",
    id = nil,
    name = nil,
    project = nil,
    port = nil,
    status_label = nil,
    status_color = nil,
    cpu = nil,
    mem = nil,
}

-- Deployments (aba Deployments). `list` = Vec<DeploymentSummary>.
function M.deployments(list: {any}?, term: string?): string
    local rows = {}
    for _, s in ipairs(list or {}) do
        if Util.matches(term, { s.service_name, s.project_name }) then
            local d = s.deployment
            local label, _ = Util.state_label_color(d.state)
            table.insert(rows, {
                service = s.service_name,
                project = s.project_name,
                state_label = label,
                state_kind = Util.state_kind(d.state),
                duration = Time.fmt_duration(d),
                start = Time.time_hms(d.started_at),
            })
        end
    end
    return Util.enc_array(rows)
end

-- Escolhe o container "primário" de um serviço p/ exibir no card: o live, senão
-- o primeiro em execução, senão o primeiro da lista. Devolve (nome, id_curto,
-- extra) onde extra é "+N" quando há mais de um container.
local function primary_container(svc: Types.Service): (string, string, string)
    local list = svc.containers or {}
    if #list == 0 then return "—", "", "" end
    local chosen = list[1]
    for _, c in ipairs(list) do
        if svc.live_container_id and c.id == svc.live_container_id then chosen = c; break end
        if c.state == "running" and chosen.state ~= "running" then chosen = c end
    end
    local extra = #list > 1 and string.format("+%d", #list - 1) or ""
    return chosen.name or "—", string.sub(chosen.id or "", 1, 12), extra
end

-- Card de serviço (com CPU/mem mesclados de `metrics_by_id[svc.id]`).
local function service_card(svc: Types.Service, proj: string, metrics: MetricsMap?)
    local status_label, status_color = Util.status_label_color(svc.status)
    local cpu, mem = "—", "—"
    local m = metrics and metrics[svc.id]
    if m then
        cpu = string.format("%.1f%%", m.cpu_percent or 0)
        mem = Time.fmt_bytes(m.mem_used_bytes)
    end
    local cname, cid, cextra = primary_container(svc)
    return {
        filler = "0",
        id = svc.id,
        name = svc.spec.name,
        project = proj,
        port = tostring(svc.spec.port),
        status_label = status_label,
        status_color = status_color,
        cpu = cpu,
        mem = mem,
        container_name = cname,
        container_id = cid,
        container_extra = cextra,
    }
end

-- Array plano de cards de serviço (para seleção/contagem).
function M.services(pairs_: {ServicePair}, metrics: MetricsMap?, term: string?): string
    local rows = {}
    for _, p in ipairs(pairs_) do
        if Util.matches(term, { p.svc.spec.name, p.proj }) then
            table.insert(rows, service_card(p.svc, p.proj, metrics))
        end
    end
    return Util.enc_array(rows)
end

-- Cards de serviço em linhas de GRID_COLS, com filler (glacier não tem grid).
function M.service_rows(pairs_: {ServicePair}, metrics: MetricsMap?, term: string?): string
    local filtered: {GridCard} = {}
    for _, p in ipairs(pairs_) do
        if Util.matches(term, { p.svc.spec.name, p.proj }) then
            table.insert(filtered, service_card(p.svc, p.proj, metrics))
        end
    end
    local rows = {}
    local i = 1
    while i <= #filtered do
        local cards = {}
        for j = i, math.min(i + GRID_COLS - 1, #filtered) do
            table.insert(cards, filtered[j])
        end
        while #cards < GRID_COLS do table.insert(cards, FILLER) end
        table.insert(rows, { cards = cards })
        i = i + GRID_COLS
    end
    return Util.enc_array(rows)
end

-- Card de projeto, agregando contagem de serviços de `pairs_`.
local function project_card(p: any, pairs_: {ServicePair})
    local count, running = 0, 0
    for _, e in ipairs(pairs_) do
        if e.svc.spec.project_id == p.id then
            count = count + 1
            if e.svc.status == "Running" then running = running + 1 end
        end
    end
    return {
        filler = "0",
        id = p.id,
        name = p.name,
        description = p.description or "",
        service_count = tostring(count),
        running_count = tostring(running),
        can_delete = (count == 0) and "1" or "0",
    }
end

function M.projects(list: {any}?, pairs_: {ServicePair}, term: string?): string
    local rows = {}
    for _, p in ipairs(list or {}) do
        if Util.matches(term, { p.name, p.description or "" }) then
            table.insert(rows, project_card(p, pairs_))
        end
    end
    return Util.enc_array(rows)
end

function M.project_rows(list: {any}?, pairs_: {ServicePair}, term: string?): string
    local filtered: {GridCard} = {}
    for _, p in ipairs(list or {}) do
        if Util.matches(term, { p.name, p.description or "" }) then
            table.insert(filtered, project_card(p, pairs_))
        end
    end
    local rows = {}
    local i = 1
    while i <= #filtered do
        local cards = {}
        for j = i, math.min(i + GRID_COLS - 1, #filtered) do
            table.insert(cards, filtered[j])
        end
        while #cards < GRID_COLS do table.insert(cards, FILLER) end
        table.insert(rows, { cards = cards })
        i = i + GRID_COLS
    end
    return Util.enc_array(rows)
end

-- Linhas Docker (containers gerenciados) derivadas dos serviços, ordenadas.
function M.docker_rows(pairs_: {ServicePair}, term: string?): string
    local sorted: {ServicePair} = {}
    for _, p in ipairs(pairs_) do table.insert(sorted, p) end
    table.sort(sorted, function(a: ServicePair, b: ServicePair)
        return a.svc.spec.name < b.svc.spec.name
    end)
    local rows = {}
    for _, p in ipairs(sorted) do
        if Util.matches(term, { p.svc.spec.name, p.proj }) then
            local label, _ = Util.status_label_color(p.svc.status)
            local cid = "—"
            if p.svc.live_container_id and p.svc.live_container_id ~= "" then
                cid = string.sub(p.svc.live_container_id :: string, 1, 12)
            end
            table.insert(rows, {
                name = p.svc.spec.name,
                project = p.proj,
                image = Util.source_summary(p.svc.spec.source),
                container = cid,
                status_label = label,
                status_kind = Util.status_kind(p.svc.status),
            })
        end
    end
    return Util.enc_array(rows)
end

function M.docker_images(list: {any}?, term: string?): string
    local sorted = {}
    for _, img in ipairs(list or {}) do table.insert(sorted, img) end
    table.sort(sorted, function(a: any, b: any)
        return table.concat(a.tags or {}, ",") < table.concat(b.tags or {}, ",")
    end)
    local rows = {}
    for _, img in ipairs(sorted) do
        local tagstr = table.concat(img.tags or {}, " ")
        if Util.matches(term, { tagstr, img.project or "", img.service or "" }) then
            local in_use = (img.containers or 0) > 0
            -- Trunca por caractere (não por palavra): uma tag sem espaços mais
            -- longa que a coluna estoura e sobrepõe as vizinhas (o Row não clipa
            -- filhos) — mesmo caso do sha256 em docker_containers.
            local tags = (#(img.tags or {}) == 0) and "<none>"
                or Util.ellipsis(table.concat(img.tags, ", "), 34)
            local id = img.id or ""
            id = string.gsub(id, "^sha256:", "")
            table.insert(rows, {
                id = string.sub(id, 1, 12),
                id_full = id, -- sem "sha256:" (o `:` quebraria a ação nome:sufixo)
                tags = tags,
                size = Time.fmt_bytes(img.size_bytes),
                created = Time.date_dm_hm(img.created),
                project = img.project or "—",
                service = img.service or "—",
                in_use_label = in_use and "EM USO" or "SEM USO",
                in_use_kind = Util.in_use_kind(in_use),
            })
        end
    end
    return Util.enc_array(rows)
end

function M.docker_volumes(list: {any}?, term: string?): string
    local sorted = {}
    for _, v in ipairs(list or {}) do table.insert(sorted, v) end
    table.sort(sorted, function(a: any, b: any) return a.name < b.name end)
    local rows = {}
    for _, v in ipairs(sorted) do
        if Util.matches(term, { v.name, v.driver }) then
            table.insert(rows, {
                name = v.name,
                driver = v.driver,
                mountpoint = v.mountpoint,
                size = (v.size_bytes and v.size_bytes >= 0) and Time.fmt_bytes(v.size_bytes) or "—",
                in_use_label = v.in_use and "EM USO" or "SEM USO",
                in_use_kind = Util.in_use_kind(v.in_use),
            })
        end
    end
    return Util.enc_array(rows)
end

function M.docker_networks(list: {any}?, term: string?): string
    local sorted = {}
    for _, n in ipairs(list or {}) do table.insert(sorted, n) end
    table.sort(sorted, function(a: any, b: any) return a.name < b.name end)
    local rows = {}
    for _, n in ipairs(sorted) do
        if Util.matches(term, { n.name, n.project or "" }) then
            table.insert(rows, {
                name = n.name,
                id_full = n.id or "",
                driver = n.driver,
                scope = n.scope,
                project = n.project or "—",
                containers = tostring(n.container_count or 0),
                in_use_label = n.in_use and "EM USO" or "SEM USO",
                in_use_kind = Util.in_use_kind(n.in_use),
            })
        end
    end
    return Util.enc_array(rows)
end

-- Containers do host (DockerContainerInfo): todos, rodando + parados. `can_remove`
-- = "1" só nos parados (o Docker recusa remover um rodando sem force). `owner`
-- mostra projeto/serviço quando atribuído (label rustploy.service_id), senão
-- "rustploy" (managed) ou "—".
function M.docker_containers(list: {any}?, term: string?): string
    local sorted = {}
    for _, c in ipairs(list or {}) do table.insert(sorted, c) end
    table.sort(sorted, function(a: any, b: any) return (a.name or "") < (b.name or "") end)
    local rows = {}
    for _, c in ipairs(sorted) do
        if Util.matches(term, { c.name, c.image, c.state, c.project or "", c.service or "" }) then
            local label, kind = Util.container_state(c.state)
            local owner: string
            if c.project and c.service then
                owner = tostring(c.project) .. " / " .. tostring(c.service)
            elseif c.managed then
                owner = "rustploy"
            else
                owner = "—"
            end
            -- A referência da imagem vira `sha256:<digest>` quando a tag original
            -- some (imagem removida/reconstruída) — sem espaços, não quebra linha
            -- e, sem truncar, estoura a coluna e sobrepõe as vizinhas (o Row não
            -- clipa filhos). Tira o prefixo redundante (igual a docker_images) e
            -- trunca por caractere (Util.ellipsis, não por palavra).
            local image = c.image or "—"
            image = string.gsub(image, "^sha256:", "")
            table.insert(rows, {
                id_full = c.id or "",
                id = string.sub(c.id or "", 1, 12),
                name = c.name or "—",
                image = Util.ellipsis(image, 44),
                owner = owner,
                state_label = label,
                state_kind = kind,
                can_remove = Util.container_stopped(c.state) and "1" or "0",
            })
        end
    end
    return Util.enc_array(rows)
end

-- Ingress: uma linha por rota de domínio (não filtrado pela busca). Retorna (json, count).
function M.ingress(pairs_: {ServicePair}): (string, number)
    local rows = {}
    for _, p in ipairs(pairs_) do
        local spec = p.svc.spec
        local port = spec.port
        for _, r in ipairs(Util.domain_routes(spec)) do
            if r.domain and string.gsub(r.domain, "%s", "") ~= "" then
                local scheme = r.tls and "https" or "http"
                local cport = r.port or port
                table.insert(rows, {
                    domain = r.domain,
                    url = scheme .. "://" .. r.domain,
                    service = spec.name,
                    project = p.proj,
                    upstream = ":" .. tostring(cport),
                    tls = r.tls and "TLS" or "—",
                })
            end
        end
    end
    return Util.enc_array(rows), #rows
end

-- Monitoring: uma linha por serviço COM métricas vivas.
function M.monitoring(pairs_: {ServicePair}, metrics: MetricsMap?): string
    local rows = {}
    for _, p in ipairs(pairs_) do
        local m = metrics and metrics[p.svc.id]
        if m then
            table.insert(rows, {
                name = p.svc.spec.name,
                project = p.proj,
                cpu = string.format("%.1f%%", m.cpu_percent or 0),
                mem = Time.fmt_bytes(m.mem_used_bytes),
                rx = Time.fmt_bytes(m.net_rx_bytes),
                tx = Time.fmt_bytes(m.net_tx_bytes),
            })
        end
    end
    return Util.enc_array(rows)
end

-- Deploy Engine: "Executando agora".
function M.eng_active(active: {any}?): string
    local rows = {}
    for _, info in ipairs(active or {}) do
        local label, _ = Util.state_label_color(info.state)
        table.insert(rows, {
            service = info.service_name,
            project = info.project_name,
            state_label = label,
            state_kind = Util.state_kind(info.state),
            percent = tostring(info.percent) .. "%",
            bar = Time.progress_bar(info.percent, 12),
            total = Time.fmt_secs(info.elapsed_secs),
            phase = Time.fmt_secs(info.current_state_secs),
            service_id = info.service_id,
        })
    end
    return Util.enc_array(rows)
end

-- Deploy Engine: "Histórico 24h".
function M.eng_recent(recent: {any}?): string
    local rows = {}
    for _, info in ipairs(recent or {}) do
        local label, _ = Util.state_label_color(info.state)
        local icon = "○"
        if info.state == "Live" then icon = "✓"
        elseif info.state == "Failed" then icon = "✕" end
        table.insert(rows, {
            icon = icon,
            service = info.service_name,
            project = info.project_name,
            state_label = label,
            state_kind = Util.state_kind(info.state),
            duration = Time.fmt_secs(info.elapsed_secs),
            start = Time.time_hms(info.started_at),
        })
    end
    return Util.enc_array(rows)
end

return M
