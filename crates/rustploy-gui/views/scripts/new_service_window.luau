--!strict
--!nolint FunctionUnused
-- new_service_window.luau — script da JANELA do wizard "Novo serviço", um motor
-- Glacier próprio e ISOLADO do app principal (aberto via open_window por
-- handlers/wizard.luau::open_new_service_window). Recebe a conexão
-- (api_url/api_token) e o projeto-alvo (selected_project_id/proj_name) semeados
-- no contexto por open_window({ data = ... }).
--
-- A lógica do wizard em si (ns_kind/ns_db/ns_create/…) vem de handlers/wizard.luau,
-- carregado abaixo por require (define as funções globais que o template usa).
-- require resolve relativo a ESTE arquivo (scripts/): "state", "net/api" e
-- "handlers/wizard" — os mesmos módulos do app principal, mas neste motor isolado
-- (State.api é reconstruído da conexão semeada).
--
-- O catálogo (ns_dbs/ns_brokers/ns_templates) vem SEMEADO via open_window({data})
-- — a janela principal já o buscou. NÃO buscamos aqui no init: o `fetch` de um
-- init é descartado pelo motor (só ações despachadas rodam no executor); por isso
-- o antigo `load_catalog()` no init deixava as listas vazias.

local State = require("state")
local Api = require("net/api")
require("handlers/wizard")

function init(): ()
    -- Reconstrói o cliente da API a partir da conexão semeada (usado pelo
    -- ns_create, que é uma ação despachada — aí o fetch funciona).
    local url = ctx.api_url or ""
    if url ~= "" then
        State.api = Api.new(url, ctx.api_token)
    end

    -- Decodifica os templates semeados para State (usado por ns_pick_template/
    -- ns_tsearch, que leem State.ns_templates_all, não o contexto).
    local okt, all = pcall(json.decode, ctx.ns_templates or "[]")
    State.ns_templates_all = (okt and type(all) == "table") and all or {}

    -- Estado inicial do wizard (o catálogo já veio no contexto via data).
    ctx.ns_name = ""
    ctx.ns_app_name = ""
    ctx.ns_msg = ""
    ctx.erro_ns_name = ""
    ctx.ns_tsearch = ""
    ctx.ns_step = "pick_type"
end
