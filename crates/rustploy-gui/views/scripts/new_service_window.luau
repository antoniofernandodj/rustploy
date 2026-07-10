--!strict
--!nolint FunctionUnused
-- new_service_window.luau — script da JANELA do wizard "Novo serviço", um motor
-- Glacier próprio e ISOLADO do app principal (aberto via open_window por
-- handlers/wizard.luau::open_new_service_window). Recebe a conexão
-- (api_url/api_token) e o projeto-alvo (selected_project_id/proj_name) semeados
-- no contexto por open_window({ data = ... }).
--
-- A lógica do wizard em si (ns_kind/ns_db/ns_create/…) vem de handlers/wizard.luau,
-- carregado abaixo por require (define as funções globais que o template usa e
-- expõe load_catalog). require resolve relativo a ESTE arquivo (scripts/): "state",
-- "net/api" e "handlers/wizard" — os mesmos módulos do app principal, mas neste
-- motor isolado (State.api é reconstruído da conexão semeada).

local State = require("state")
local Api = require("net/api")
local Wizard = require("handlers/wizard")

function init(): ()
    -- Reconstrói o cliente da API a partir da conexão semeada.
    local url = ctx.api_url or ""
    if url ~= "" then
        State.api = Api.new(url, ctx.api_token)
    end

    -- Semeia o estado inicial do wizard (mesmo que o antigo new_service_open fazia).
    ctx.ns_name = ""
    ctx.ns_app_name = ""
    ctx.ns_msg = ""
    ctx.erro_ns_name = ""
    ctx.ns_tsearch = ""
    ctx.ns_step = "pick_type"

    -- Busca o catálogo (dbs/brokers/templates) do daemon.
    Wizard.load_catalog()
end
