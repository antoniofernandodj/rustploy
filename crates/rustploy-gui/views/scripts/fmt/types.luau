--!strict
-- fmt/types.luau — tipos compartilhados entre os submódulos fmt_*.luau (as
-- formas dos modelos que chegam via `json.decode` do snapshot da API).
-- `require("types")` (de dentro de fmt/) + `type X = Types.X` para importar cada tipo.

export type EnvValue = {
    Secret: string?,
    Plain: string?,
}

export type EnvVar = {
    key: string,
    value: EnvValue,
}

export type EnvComment = {
    before_key: string?,
    text: string,
}

export type DomainRoute = {
    domain: string,
    port: number?,
    tls: boolean,
}

export type ServiceSpec = {
    name: string,
    port: number,
    source: any,
    domain: string?,
    tls_enabled: boolean?,
    domains: { { domain: string, port: number?, tls: boolean? } }?,
    project_id: string,
}

export type Service = {
    id: string,
    status: string | { Error: string? },
    spec: ServiceSpec,
    live_container_id: string?,
}

export type MetricEntry = {
    cpu_percent: number?,
    mem_used_bytes: number?,
    net_rx_bytes: number?,
    net_tx_bytes: number?,
}

export type ServicePair = {
    svc: Service,
    proj: string,
}

export type GridCard = { filler: string, [string]: any }

export type MetricsMap = { [string]: MetricEntry }

return {}
