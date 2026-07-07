--!strict
-- state.luau — estado mutável compartilhado entre todos os handlers_*.luau
-- (mesmo interpretador, mesma tabela: `require("state")` é cacheado por
-- identidade, então toda mutação feita por um handler é vista pelos demais).
--
-- NOTA sobre nomes de arquivo: os módulos deste diretório ficam FLAT (sem
-- subpastas como `handlers/`/`fmt/`) de propósito. O `require` do glacier
-- resolve sempre relativo ao diretório do SCRIPT DE ENTRADA (`app.luau`),
-- **não** ao arquivo que faz o require; já o luau-lsp resolve relativo ao
-- arquivo que chama `require`. As duas resoluções só coincidem quando todo
-- mundo é irmão no mesmo diretório — daí `handlers_stream.luau` em vez de
-- `handlers/stream.luau` (que faria o luau-lsp procurar
-- `handlers/handlers/stream.luau` e reportar erro falso). Ver o commit que
-- introduziu esta modularização para o registro completo dessa investigação.

local Api = require("net/api")

export type DeployTrack = { service_id: string, deployment_id: string, running: boolean }

export type State = {
    api: Api.Client?,                 -- cliente Api conectado
    state_conn: any,                  -- handle do stream SSE
    snap: any,                        -- último snapshot decodificado (para refiltrar)
    last_pairs: { any },               -- serviços normalizados { svc, proj } do snapshot
    metrics_by_id: { [string]: any },  -- service_id → ContainerMetricsPoint (bus)
    search_term: string,              -- termo de busca corrente (minúsculas)
    svc_logs_buf: { any },             -- ring de logs vivos do serviço aberto (LogLine)
    blog_buf: { any },                 -- ring de build logs do deployment selecionado
    deploy_track: DeployTrack,
    ns_templates_all: { any },         -- catálogo completo de templates (filtrar busca)
    LOG_RING: number,
    BUILD_RING: number,
}

local S: State = {
    api = nil,
    state_conn = nil,
    snap = nil,
    last_pairs = {},
    metrics_by_id = {},
    search_term = "",
    svc_logs_buf = {},
    blog_buf = {},
    deploy_track = { service_id = "", deployment_id = "", running = false },
    ns_templates_all = {},
    LOG_RING = 200,
    BUILD_RING = 2000,
}

return S
