# Índice: rustploy-gui: telas e componentes .gv, estilos

> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> `.gv`: `<screen>`/`<component>`, `props`, `imports` (componentes), `script`, `views` (valores de `if="{view}" equals=`) e `handlers` (chamados em `on_*=`).

## rustploy-gui/views/

### app.gv — Janela principal da GUI: mostra o Login ou o Shell (app conectado), com a titlebar customizada.
<screen "Rustploy">
imports: Login, Shell
script: scripts/app.luau
handlers: notifications, tray, window

## rustploy-gui/views/components/

### badge.gv — Variante "badge" da célula de estado (mesmo ponto + rótulo, mas com o espaçamento/estilo de crachá —…
<component>
props: color, label

### loading_row.gv — Linha "Carregando dados…" com spinner.
<component>
props: note

### nav_item.gv — Item de navegação da sidebar: ícone (sempre visível) + rótulo (some abaixo de 900px de largura — ver…
<component>
props: label, icon, target, action

### picker_row.gv — Linha de escolha do wizard "Novo serviço": título + subtítulo à esquerda e um botão de ação à direit…
<component>
props: title, subtitle, button_label, action

### project_card.gv — Template "fragment": dois nós de topo (o slot vazio e o card) — o glacier-ui (0.4.12+) embrulha múlt…
<component>
props: id, name, service_count, running_count, description, can_delete, filler
handlers: delete_project, open_project

### service_card.gv — Card de serviço da aba "Serviços" de um projeto — o análogo do ProjectCard.
<component>
props: id, name, port, status_color, status_label, cpu, mem, filler, container_name, container_id, container_extra
handlers: open_service, stop_delete_service, svc_stop_id

### stat_card.gv — Tile de KPI do cabeçalho (STATUS/UPTIME/SERVICES/CPU/…).
<component>
props: label, value, accent

### state_cell.gv — Célula de estado das tabelas: o ponto colorido "●" + o rótulo, ambos na mesma cor.
<component>
props: kind, label

### tab_button.gv — Botão de aba genérico.
<component>
props: label, current, target, action

### template_row.gv — Linha do catálogo de templates de aplicação: logo à esquerda (vetor ou raster conforme logo_kind), n…
<component>
props: name, description, logo, logo_kind, action

## rustploy-gui/views/

### home.gv — Telas globais da sidebar, cada uma numa seção por valor de view: Monitoring, Ingress, Deploy Engine,…
<component>
views: monitoring, ingress, deploy_engine, docker, settings, schedules, support
handlers: clipboard, dc_run_now, dc_save, docker_prune_containers, docker_prune_images, docker_prune_networks, docker_prune_volumes, docker_rm_container, docker_rm_image, docker_rm_network, docker_rm_volume, field, gp_connect, gp_delete, gp_refresh, iac_export, iac_import, job_del, job_run_cancel, job_run_now, job_toggle, open, open_edit_job_window, open_job_logs_window, open_new_job_window, queue_cancel, queue_promote, queue_reorder, queue_toggle_pause, registry_close_repo, registry_gc, registry_open_repo, registry_open_token_window, registry_rm_repo, registry_rm_tag, registry_rm_token, settings_save

### log_window.gv — Janela de LOGS AO VIVO (runtime OU build): motor Glacier próprio e ISOLADO do app principal, aberta …
<screen>
script: scripts/log_window.luau
handlers: clipboard, textarea_end, textarea_top, window

### login.gv — Tela de login: URL do daemon e token, com a lista de servidores lembrados.
<component>
handlers: connect, esquecer_servidor

### new_job_window.gv — Janela "Novo job": motor Glacier próprio, aberto por open_window a partir do app principal (handlers…
<screen "Novo job — Rustploy">
imports: PickerRow, TabButton
script: scripts/new_job_window.luau
handlers: cancel, field, njob_back, njob_create, njob_git_provider_pick, njob_git_repo_pick, njob_pick_no_service, window

### new_project_form.gv — Janela "Novo projeto": motor Glacier próprio, aberto por open_window a partir do app principal (hand…
<screen "Novo projeto — Rustploy">
script: scripts/new_project_window.luau
handlers: cancel, np_apontar, submit_project, window

### new_registry_token_window.gv — Janela "Novo token do registry": motor Glacier próprio, aberto por open_window a partir do app princ…
<screen "Novo token — Rustploy">
imports: TabButton
script: scripts/new_registry_token_window.luau
handlers: clipboard, ntok_apontar, ntok_create, window

### new_service.gv — Wizard "Novo serviço" (view=new_service): tipo → formulário por tipo, espelhando o fluxo do antigo r…
<component>
handlers: field, ns_back, ns_cancel, ns_create, ns_tsearch

### new_service_window.gv — Janela do wizard "Novo serviço": motor Glacier próprio, aberto por open_window (handlers/wizard.luau…
<screen "Novo serviço — Rustploy">
imports: NewServiceWizard, PickerRow, TemplateRow
script: scripts/new_service_window.luau
handlers: window

### service.gv — Detalhe de um serviço: cabeçalho com ações (deploy, stop, reload) e as abas General (fonte), Connect…
<component>
handlers: adv_save, archive_upload, clipboard, compose_cancel, compose_save, delete_deployment, dep_logs, dom_add, dom_del, dom_hostport_auto, dom_hostport_save, env_add, env_del, env_export, env_import, env_reorder, env_text_cancel, env_text_toggle, field, gen_save, gitea_provider_pick, gitea_repo_pick, hc_save, mdb_create, mdb_delete, mdb_show, mig_discard, mig_refresh, mig_rollback, mig_start, open_logs_window, open_project, pdc_add, pdc_del, pdc_reorder, regen_webhook, save_service_name, shared_disable, shared_enable, svc_deploy, svc_rebuild, svc_reload, svc_rename_apontar, svc_stop

### shell.gv — Casca do app conectado: sidebar, topbar e as views de projeto (Deployments, Projects, serviços de um…
<component>
imports: HomeViews, LoadingRow, NavItem, PickerRow, ProjectCard, ServiceCard, ServiceDetail, StatCard, StateCell, TabButton, TemplateRow
views: deployments, projects, project_services, service
handlers: app, cancel_project_edit, delete_project, deployments_clear_finished, disconnect, edit_project_toggle, job_del, job_run_cancel, job_run_now, job_toggle, nav_projects, open_edit_job_window, open_job_logs_window, open_new_job_window, open_new_project_window, open_new_service_window, penv_add, penv_apontar, penv_del, penv_export, penv_import, penv_reorder, penv_secret_toggle, penv_text_cancel, penv_text_toggle, proj_edit_apontar, save_project_edit, search_changed, secret_add, secret_add_apontar, secret_del, secret_use, stop_all

## rustploy-gui/views/styles/

### app.gss — Rustploy — glacier-ui stylesheet.
