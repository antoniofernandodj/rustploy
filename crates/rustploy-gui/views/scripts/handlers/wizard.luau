--!strict
--!nolint FunctionUnused
-- handlers/wizard.luau — wizard "Novo serviço". A construção do ServiceSpec
-- (incl. compose de banco/broker e blueprints de template) vive no daemon: o
-- cliente só coleta os campos e delega via Command::WizardCatalog (catálogos)
-- + WizardCreate (montagem+criação).

local State = require("../state")
local Helpers = require("../helpers")

local NS_ALPHABET = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
math.randomseed(os.time())
local function token_urlsafe(n: number): string
    local t = {}
    for i = 1, n do
        local idx = math.random(1, #NS_ALPHABET)
        t[i] = string.sub(NS_ALPHABET, idx, idx)
    end
    return table.concat(t)
end

local function ns_is_broker(id: string): boolean
    return id == "kafka" or id == "rabbitmq" or id == "nats"
end

-- Botão "+ Novo serviço" (cabeçalho do projeto): abre o wizard numa JANELA à
-- parte (motor Glacier próprio). O catálogo (dbs/brokers/templates) é buscado
-- AQUI e passado via `data` — buscar no init da janela NÃO funciona: o `init`
-- roda no register do componente e o glacier descarta o `fetch` disparado por
-- ele (só efeitos/streams de AÇÕES despachadas rodam no executor). Este handler
-- é uma ação despachada, então o `fetch` do WizardCatalog resolve normalmente e
-- a janela abre depois, já com o catálogo. Ao criar, a janela envia
-- broadcast("service_created") e se fecha (ver ns_create). Substitui o antigo
-- fluxo inline (view=new_service), removido do shell.gv.
function open_new_service_window(): ()
    local pid = ctx.selected_project_id or ""
    local api = State.api
    if pid == "" or not api then
        return
    end
    local dbs, brokers, templates = "[]", "[]", "[]"
    local ok, res = api:rpc({ WizardCatalog = { search = "" } })
    if ok and res.WizardCatalog then
        dbs = res.WizardCatalog.dbs or "[]"
        brokers = res.WizardCatalog.brokers or "[]"
        templates = res.WizardCatalog.templates or "[]"
    end
    open_window({
        file = "crates/rustploy-gui/views/new_service_window.gv",
        title = "Novo serviço — Rustploy",
        width = 560,
        height = 700,
        resizable = false,
        data = {
            api_url = ctx.api_url or "",
            api_token = ctx.api_token or "",
            selected_project_id = pid,
            proj_name = ctx.proj_name or "",
            ns_dbs = dbs,
            ns_brokers = brokers,
            ns_templates = templates,
        },
    })
end

-- "Cancelar": fecha a janela do wizard.
function ns_cancel(): ()
    close_window()
end

-- "‹ Voltar": num passo interno volta à escolha de tipo; no passo inicial
-- (pick_type) fecha a janela.
function ns_back(): ()
    local step = ctx.ns_step
    if step == "pick_db" or step == "pick_broker" or step == "app_form"
        or step == "compose_form" or step == "pick_template" or step == "db_form"
        or step == "template_form" then
        ctx.ns_step = "pick_type"
        ctx.ns_msg = ""
    else
        close_window()
    end
end

function ns_kind(k: string): ()
    if k == "application" then
        ctx.ns_step = "app_form"
    elseif k == "compose" then
        ctx.ns_step = "compose_form"
    elseif k == "database" then
        ctx.ns_step = "pick_db"
    elseif k == "broker" then
        ctx.ns_step = "pick_broker"
    elseif k == "template" then
        ctx.ns_step = "pick_template"
    end
end

-- Banco escolhido: pré-preenche o formulário (senhas geradas no cliente).
function ns_db(id: string): ()
    local ok, list = pcall(json.decode, ctx.ns_dbs or "[]")
    if not ok or type(list) ~= "table" then
        return
    end
    for _, d in ipairs(list) do
        if d.id == id then
            ctx.ns_db_kind = id
            ctx.ns_db_label = d.label
            ctx.ns_image = d.image
            ctx.ns_db_name = d.has_db_name and "app" or ""
            ctx.ns_db_user = d.user or ""
            ctx.ns_db_password = token_urlsafe(22)
            ctx.ns_db_root_password = token_urlsafe(22)
            ctx.ns_use_replica = "false"
            ctx.ns_expose_external = "false"
            ctx.ns_db_has_dbname = d.has_db_name and "true" or "false"
            ctx.ns_db_has_user = d.has_user and "true" or "false"
            ctx.ns_db_has_rootpw = d.has_rootpw and "true" or "false"
            ctx.ns_db_has_replica = d.has_replica and "true" or "false"
            ctx.ns_name = ""
            ctx.ns_step = "db_form"
            return
        end
    end
end

-- Broker escolhido: compartilha o passo db_form (mesmas chaves ns_db_*).
function ns_broker(id: string): ()
    local ok, list = pcall(json.decode, ctx.ns_brokers or "[]")
    if not ok or type(list) ~= "table" then
        return
    end
    for _, b in ipairs(list) do
        if b.id == id then
            ctx.ns_db_kind = id
            ctx.ns_db_label = b.label
            ctx.ns_image = b.image
            ctx.ns_db_name = ""
            ctx.ns_db_user = b.user or ""
            ctx.ns_db_password = token_urlsafe(22)
            ctx.ns_db_root_password = ""
            ctx.ns_use_replica = "false"
            ctx.ns_expose_external = "false"
            ctx.ns_db_has_dbname = "false"
            ctx.ns_db_has_user = b.has_user and "true" or "false"
            ctx.ns_db_has_rootpw = "false"
            ctx.ns_db_has_replica = "false"
            ctx.ns_name = ""
            ctx.ns_step = "db_form"
            return
        end
    end
end

-- Template escolhido: carrega nome/slug + variáveis editáveis (do catálogo).
function ns_pick_template(id: string): ()
    for _, t in ipairs(State.ns_templates_all) do
        if t.id == id then
            ctx.ns_template_id = id
            ctx.ns_template_name = t.name
            ctx.ns_name = t.slug or ""
            ctx.ns_template_vars = (
                t.vars and #t.vars > 0
            ) and json.encode(t.vars) or "[]";

            for _, vv in ipairs(t.vars or {}) do
                ctx["ns_tv_" .. vv.idx] = ""
            end
            ctx.ns_step = "template_form"
            return
        end
    end
end

-- Busca do catálogo de templates (filtra o cache no cliente).
function ns_tsearch(v: string): ()
    ctx.ns_tsearch = v or ""
    local term = string.lower(v or "")
    local rows = {}
    for _, t in ipairs(State.ns_templates_all) do
        if term == ""
            or string.find(string.lower(t.name or ""), term, 1, true)
            or string.find(string.lower(t.description or ""), term, 1, true)
        then
            table.insert(rows, t)
        end
    end
    ctx.ns_templates = (#rows > 0) and json.encode(rows) or "[]"
end

function ns_create(): ()
    local pid = ctx.selected_project_id or ""
    local api = State.api
    if pid == "" or not api then
        return
    end
    local step = ctx.ns_step
    local kind, id
    if step == "app_form" then
        kind, id = "application", ""
    elseif step == "compose_form" then
        kind, id = "compose", ""
    elseif step == "db_form" then
        id = ctx.ns_db_kind or ""
        kind = ns_is_broker(id) and "broker" or "database"
    elseif step == "template_form" then
        kind, id = "template", ctx.ns_template_id or ""
    else
        ctx.ns_msg = "passo inválido";
        return
    end

    -- Nome obrigatório, exceto no template (tem slug default).
    local name = Helpers.trim(ctx.ns_name)
    if 
        step ~= "template_form" and
        Helpers.trim(ctx.ns_app_name) == ""
        and name == ""
    then
        ctx.erro_ns_name = "informe um nome";
        return
    end
    ctx.erro_ns_name = ""

    -- Valores das variáveis do template, na ordem declarada.
    local tvals = json.array({})
    if step == "template_form" then
        local okv, vars = pcall(json.decode, ctx.ns_template_vars or "[]")
        if okv and type(vars) == "table" then
            for _, vv in ipairs(vars) do
                table.insert(tvals, ctx["ns_tv_" .. vv.idx] or "")
            end
        end
    end

    local req = {
        kind = kind,
        id = id,
        project_id = pid,
        name = name,
        app_name = Helpers.trim(ctx.ns_app_name),
        db_name = Helpers.trim(ctx.ns_db_name),
        user = Helpers.trim(ctx.ns_db_user),
        password = Helpers.trim(ctx.ns_db_password),
        root_password = Helpers.trim(ctx.ns_db_root_password),
        image = Helpers.trim(ctx.ns_image),
        use_replica = (ctx.ns_use_replica == "true"),
        template_values = tvals,
        -- Porta externa automática + liberação no firewall (daemon aloca da
        -- faixa [external_ports]; sentinela host_port=0 no build_spec).
        expose_external = (ctx.ns_expose_external == "true"),
    }
    ctx.ns_msg = "criando…"
    local ok, res = api:rpc_checked({ WizardCreate = req })
    if ok and res.Service then
        -- Sucesso: avisa a janela principal (que atualiza a grade de serviços do
        -- projeto aberto) e fecha esta janela do wizard.
        local disp = (name ~= "" and name) or Helpers.trim(ctx.ns_app_name)
        broadcast("service_created", {
            project_id = pid,
            name = disp,
        })
        close_window()
    else
        ctx.ns_msg = ok and "resposta inesperada" or ("erro: " .. res)
        toast({ message = tostring(ok and "resposta inesperada do daemon" or res), kind = "error" })
    end
end
