# Índice: rustploy-gui: handlers Luau (as ações que os .gv disparam)

> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Luau: `function x(a)` = global (handler que o `.gv` chama pelo nome), `local x(a)` = privada, `function M.x(a)` = exportada pelo módulo.

## crates/rustploy-gui/views/scripts/handlers/

### connection.luau — ciclo de vida da sessão: init (semeia o contexto), login/logout, configurações do daemon buscadas um…
local publish_saved_servers() — Publica só as URLs (o `<ComboEdit>` não precisa do token pra montar a lista — o token vem de `token_…
local token_for(url)
local remember_server(url, token) — Salva/atualiza um par e o move pro topo da lista — o mais usado por último fica em primeiro, e é tam…
function servidor_escolhido(value) — Handler do `<ComboEdit>` de servidor: dispara a cada tecla (`onChange`) e ao escolher um item já sal…
function esquecer_servidor() — Botão "Esquecer" ao lado do combo: remove o servidor atual da lista salva e limpa os campos — sem is…
function init() — ── init: semeia o contexto para a UI pintar antes de qualquer dado ─────────
local load_settings() — Configurações do daemon + provedores Git, buscados uma vez na conexão (não entram no snapshot para n…
function connect() — on_submit/on_click do formulário de login.
function disconnect() — Logout: nada da sessão anterior pode sobreviver em RAM — nem no `ctx` (o espelho do contexto do moto…

### deploy_queue.luau — gerência da fila global de deploys (um por vez) na tela Deploy Engine: cancelar/promover um item enf…
function queue_cancel(deployment_id) — Cancela um deploy que ainda está esperando na fila (não começou).
function queue_promote(deployment_id) — Move um deploy enfileirado para o topo da fila ("furar fila").
function queue_reorder(v) — Reordena a fila (arraste): `v` é um JSON array de deployment_ids na nova ordem (mesma forma que o en…
function queue_toggle_pause() — Pausa/retoma a fila.

### docker.luau — limpeza de recursos Docker sem uso (imagens, volumes, redes).
local prune(cmd, label)
function docker_prune_containers()
function docker_prune_images()
function docker_prune_volumes()
function docker_prune_networks()
local remove_one(cmd, doing)
function docker_rm_container(id) — Cada par confirm→do_* espelha o padrão de delete_deployment: o `confirm` suspende e, se o usuário co…
function docker_rm_image(id)
function docker_rm_volume(name)
function docker_rm_network(id)

### jobs.luau — ações da tela global "Schedules" (sidebar) e da aba "Jobs" do projeto: rodar agora, pausar/ativar, r…
function job_run_now(id) — Bloqueia o botão IMEDIATAMENTE ao clicar (antes de qualquer resposta do backend chegar) e trava doub…
function job_run_cancel(job_run_id) — Cancela um job_run em execução (ver docs/plano-cancelamento-de-jobs.md): mata o processo `docker com…
function job_toggle(id) — Pausa/ativa um job: `JobUpdate` substitui o registro inteiro, então reenviamos os campos como estão …
function open_new_job_window() — Abre a janela de cadastro de job (motor Glacier próprio, isolado).
function open_edit_job_window(id) — Abre a MESMA janela em modo edição: pula os passos 1/2 (projeto/serviço gatilho não são editáveis vi…
function open_job_logs_window(job_run_id) — Logs da última execução de um job — mesma janela genérica de logs (log_window.gv/.luau) usada por ru…
function job_del(id)

### nav.luau — navegação da sidebar/tabs e busca do topbar.
local goto_view(v)
function nav_deployments()
function nav_deploy_engine()
function nav_monitoring()
function nav_schedules()
function nav_ingress()
function nav_docker()
function nav_projects()
function nav_settings()
function nav_support()
function nav(v) — Genérico p/ NavItem que usa action="nav" com value=target (se algum usar).
function proj_tab(t) — Abas (TabButton dispara `proj_tab:services`, `docker_tab:images`, …).
function docker_tab(t)
function deploy_tab(t)
function tab(t) — Abas do service detail e do healthcheck kind.
function prov(t)
function hckind(k)
function field(key, v) — Setter genérico de campo (on_change="field:<chave>").
function search_changed(v)

### projects.luau — grade de projetos: criar/editar/remover projeto, variáveis de ambiente de projeto, ações de serviço …
function open_project(id) — Abrir a lista de serviços de um projeto (grade read-only via snapshot).
function open_new_project_window() — Abre a janela de cadastro de projeto (motor Glacier próprio, isolado).
function on_broadcast(event, payload) — Recebe mensagens de OUTRAS janelas (glacier broadcast).
function create_project() — NOTA: create_project (formulário inline, removido da UI em favor da janela acima) é mantido por ser …
function delete_project(id)
function edit_project_toggle()
function cancel_project_edit()
function proj_edit_apontar(_erros_json) — on_validation_error do <form name="edit_project"> — o motor já publicou {erro_edit_proj_name} e acen…
function save_project_edit() — on_submit: só roda com a validação (rules="required" no NOME) aprovada.
local with_project_env(mutate)
local parse_project_dotenv(text) — Parser .env (KEY=VALUE, # comentário, <secret:nome>) → (env_vars, comments).
function penv_apontar(_erros_json) — on_validation_error do <form name="penv_add"> (a chave é obrigatória via rules="required").
function penv_add()
function penv_del(key)
function penv_reorder(v)
function penv_text_toggle()
function penv_export()
function penv_text_cancel()
function penv_import()
function svc_stop_id(id)
function stop_delete_service(id)
function stop_all()

### registry.luau — sub-aba Docker > Registry: navega repo→tags (fetch sob demanda, tags não vêm no snapshot periódico),…
function registry_open_repo(name) — Abre a visão de tags de um repositório: fetch on-demand (tags não vêm no snapshot periódico, só a li…
function registry_close_repo()
function registry_rm_tag(tag)
function registry_rm_repo(name)
function registry_gc()
function registry_refresh_tokens() — ── Tokens de acesso (Basic auth) ──────────────────────────────────────── Lista não vem no snapshot …
function registry_open_token_window() — Abre a janela separada de criação (segredo só aparece lá, uma vez — ver new_registry_token_window.gv…
function registry_rm_token(name)

### secrets.luau — aba "Secrets" do projeto: criar/sobrescrever e apagar valores cifrados, e o atalho "usar secret" da …
local known_names() — Nomes atualmente conhecidos (do snapshot já renderizado), para saber se um SecretSet cria ou substit…
function secret_add_apontar(_erros_json) — on_validation_error do <form name="secret_add"> — NOME e valor são `rules="required"` no <form>; o m…
function secret_add()
function secret_del(name)
function secret_use(name) — Chip "usar secret" (aba Variáveis): arma o form de adicionar var em modo secret com o nome escolhido…
function penv_secret_toggle()

### services.luau — detalhe do serviço (service.gv): fetch completo, mutações de spec/env, ciclo de vida (deploy/reload/…
local pre_deploy_checks_ids(spec) — Fila efetiva de pré-deploy check: `pre_deploy_job_ids` quando não vazia, senão cai no `pre_deploy_jo…
function open_service(id) — Abrir o detalhe de um serviço: navega e dispara o fetch (spec + logs + deployments + provedores Git)…
function M.fetch_service_detail(sid) — One-shot: monta TODAS as chaves svc_*/f_* do painel de detalhe (equivalente a net/services.rs::fetch…
local with_spec(mutate) — Busca o spec fresco, aplica `mutate(spec)`, envia ServiceUpdate e refaz o fetch do detalhe.
function M.load_shared_state(spec, sid) — Preenche svc_can_share / svc_shared e, se compartilhado, a lista de databases e de projetos (para o …
function shared_enable()
function shared_disable()
function mdb_show(id)
function mdb_create()
function mdb_delete(id)
local default_source_db(spec, project_env)
function M.load_migration_state(spec, sid, project_env)
function mig_refresh()
function mig_start()
function mig_rollback()
function mig_discard()
function svc_rename_apontar(_erros_json) — on_validation_error do <form name="svc_rename">: o motor já acendeu o :invalid e publicou {erro_edit…
function save_service_name() — on_submit: só roda com a validação (rules="required") aprovada.
local materialize_domains(spec) — Move o domínio legado (domain/tls_enabled) para a lista `domains`.
function dom_add()
function dom_del(domain)
function dom_hostport_save()
function dom_hostport_auto() — Sentinela 0 = "aloque para mim": o daemon troca por uma porta livre da faixa [external_ports] e a li…
function compose_save()
function compose_cancel()
function hc_save()
function adv_save()
local materialize_pre_deploy(spec) — Move o `pre_deploy_job_id` legado pra `pre_deploy_job_ids` — mesmo idioma de `materialize_domains`.
function pdc_add() — Adiciona o job escolhido (ctx.f_pdc_add_job_id) ao FIM da fila — a ordem de adição É a ordem de exec…
function pdc_del(job_id)
function pdc_reorder(v) — Reordena a fila (arraste): `v` é um JSON array de job_ids na nova ordem (mesmo formato de `env_reord…
function gen_save()
function archive_upload()
local reorder_env(vars, comments, keys) — Aplica uma ordem de keys (vars + linhas `__c<idx>`) a vars/comments: vars reordenadas; cada comentár…
function env_add()
function env_del(key)
function env_import()
function env_export()
function env_text_toggle()
function env_text_cancel()
function env_reorder(v) — Reordena env (arraste): `value` é um JSON array de keys na nova ordem.
function dep_logs(id) — Abre os build logs de um deployment numa JANELA à parte (mesma janela genérica dos runtime logs — ve…
function delete_deployment(id)
function M.set_webhook_url(url) — A URL do webhook é longa (base + service_id + token de 48 hex).
function regen_webhook() — Regenerar invalida o token antigo NA HORA: qualquer webhook já cadastrado no GitHub/Gitea/Docker Hub…
local svc_lifecycle(cmd, msg)
local start_deploy(id) — Deploy de um serviço (a partir de um card / detalhe).
function svc_deploy()
function svc_rebuild()
function svc_reload()
function svc_stop()
function gitea_provider_pick(id)
function gitea_repo_pick(full_name)
function open_logs_window() — Abre os logs ao vivo do serviço numa JANELA à parte (motor Glacier isolado e leve — ver log_window.g…

### settings.luau — Settings (Web Server) e Settings → Git (provedores Gitea: conectar via OAuth/PAT, atualizar lista, r…
function settings_tab(t)
function settings_save()
function gp_mode(m)
function gp_kind(k) — Alterna o tipo de provedor (gitea | github) e recomputa a redirect URI, que tem path por provedor (/…
local gp_refresh_list()
function gp_refresh()
function gp_connect()
function gp_delete(id)
local do_export(api, zip_path) — O corpo do export DEPOIS do diálogo — separado só para o `iac_busy` ser ligado/desligado em volta se…
function iac_export()
local do_import(api, yaml, toml, prune, deploy)
function iac_import()
local dc_unpack_recurrence(r) — Recorrência (Option<Recurrence>, externally-tagged) → (kind, hours, hour, minute, weekday) — mesmo f…
local dc_apply_config(cfg)
function M.dc_load() — Carregado uma vez na conexão (handlers/connection.luau::load_settings) — não faz parte do snapshot d…
function dc_kind(k)
local dc_build_recurrence()
function dc_save()
function dc_run_now() — Botão "Executar agora": roda os recursos marcados fora do horário agendado, independente do interrup…

### stream.luau — consumidor do SSE de /api/events: aplica o snapshot periódico (2s) e os eventos vivos do bus (métric…
local rebuild_lists() — Reconstrói as listas filtradas pela busca a partir do último snapshot — sem rede, para a busca filtr…
local update_open_project() — Cabeçalho + grade de serviços do projeto aberto (view=project_services).
local refresh_jobs_summary_local() — Recomputa SÓ `ctx.jobs_summary`/`ctx.jobs_count` (tela global "Schedules") a partir do último snapsh…
local apply_snapshot(msg)
local refresh_now() — Refresh imediato após uma mutação: pega o snapshot completo num único RPC (o mesmo builder do SSE) e…
function deployments_clear_finished() — Limpeza em massa da tela Deployments: apaga (DeployDelete) todo deployment em estado terminal Stoppe…
local apply_bus(ev) — Eventos vivos do bus: métricas e transições de deploy.
function M.open_stream()

### wizard.luau — wizard "Novo serviço".
local token_urlsafe(n)
local ns_is_broker(id)
function open_new_service_window() — Botão "+ Novo serviço" (cabeçalho do projeto): abre o wizard numa JANELA à parte (motor Glacier próp…
function ns_cancel() — "Cancelar": fecha a janela do wizard.
function ns_back() — "‹ Voltar": num passo interno volta à escolha de tipo; no passo inicial (pick_type) fecha a janela.
function ns_kind(k)
function ns_db(id) — Banco escolhido: pré-preenche o formulário (senhas geradas no cliente).
function ns_broker(id) — Broker escolhido: compartilha o passo db_form (mesmas chaves ns_db_*).
function ns_pick_template(id) — Template escolhido: carrega nome/slug + variáveis editáveis (do catálogo).
function ns_tsearch(v) — Busca do catálogo de templates (filtra o cache no cliente).
function ns_create()
