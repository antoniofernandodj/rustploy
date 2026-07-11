--!strict
-- state.luau — estado mutável compartilhado entre todos os handlers/*.luau
-- (mesmo interpretador, mesma tabela: `require("../state")` é cacheado por
-- identidade — pelo CAMINHO resolvido do arquivo, não pela string pedida —
-- então toda mutação feita por um handler é vista pelos demais).
--
-- Convenção de `require` neste projeto (glacier-ui 0.22.0+: resolução
-- relativa ao ARQUIVO que chama, como Node.js/Lua padrão — ver
-- CHANGELOG do glacier-ui e docs/luau-modularizacao-pacotes.md):
--   - Irmão no MESMO diretório/pacote → nome nu: `require("stream")` de
--     dentro de `handlers/` acha `handlers/stream.luau`.
--   - Pacote PAI (este arquivo, `helpers.luau`, `fmt.luau`, `net/api.luau`,
--     todos na raiz de `views/scripts/`) a partir de dentro de `handlers/`
--     ou `fmt/` → prefixo explícito `../`: `require("../state")`.
--   - Do script de ENTRADA (`app.luau`, que fica na raiz) → caminho completo
--     nu: `require("handlers/connection")`, `require("fmt/time")`.
-- Ver `docs/luau-modularizacao-pacotes.md` (raiz do repo rustploy) para a
-- investigação completa que motivou essa convenção.

local Api = require("net/api")

export type DeployTrack = { service_id: string, deployment_id: string, running: boolean }

export type State = {
    api: Api.Client?,                 -- cliente Api conectado
    state_conn: any,                  -- handle do stream SSE
    snap: any,                        -- último snapshot decodificado (para refiltrar)
    last_pairs: { any },               -- serviços normalizados { svc, proj } do snapshot
    metrics_by_id: { [string]: any },  -- service_id → ContainerMetricsPoint (bus)
    search_term: string,              -- termo de busca corrente (minúsculas)
    -- Logs (runtime/build) NÃO vivem mais aqui: cada tipo roda numa janela
    -- isolada (log_window.luau) com seu próprio ring e SSE dedicada, para não
    -- pesar na reavaliação da árvore da janela principal. Ver handlers/services
    -- (open_logs_window / dep_logs) e o daemon (endpoints /api/.../logs).
    deploy_track: DeployTrack,
    ns_templates_all: { any },         -- catálogo completo de templates (filtrar busca)
}

local S: State = {
    api = nil,
    state_conn = nil,
    snap = nil,
    last_pairs = {},
    metrics_by_id = {},
    search_term = "",
    deploy_track = { service_id = "", deployment_id = "", running = false },
    ns_templates_all = {},
}

return S
