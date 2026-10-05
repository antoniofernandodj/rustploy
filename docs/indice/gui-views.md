# Índice: rustploy-gui: telas e componentes .gvb, estilos

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> `.gvb`: `<app>`/`<screen>`/`<component>`, `props`, `imports` (`link(rel = import)`), `script`, `telas` (as `screen(name = …)` do app), `views` (valores de `@view ==`), `abas` (valores de `@tab`/`@*_tab`) e `handlers` (`on_*` e `action`).

## rustploy-gui/views/

### app.gvb — O app Rustploy: a raiz do arquivo é o `app(...)`, e as telas — a janela principal e as cinco janelas…
<app>
imports: Login, Shell
script: scripts/app.luau
telas: app, log, new_job, new_service, new_project, new_registry_token
handlers: notifications, tray, window

### home.gvb — Telas globais da sidebar, cada uma numa seção por valor de view: Monitoring, Ingress, Deploy Engine,…
<component>
imports: HomeDeployEngine, HomeDocker, HomeIngress, HomeMonitoring, HomeSchedules, HomeSettings
views: monitoring, ingress, deploy_engine, docker, settings, schedules

### log_window.gvb — Janela de LOGS AO VIVO (runtime OU build): motor Glacier próprio e ISOLADO do app principal, aberta …
<screen>
script: scripts/log_window.luau
handlers: clipboard, textarea_end, textarea_top, window

### login.gvb — Tela de login: URL do daemon e token, com a lista de servidores lembrados.
<component>
handlers: connect, esquecer_servidor

### new_job_window.gvb — Janela "Novo job": motor Glacier próprio, aberto por open_window a partir do app principal (handlers…
<screen "Novo job — Rustploy">
imports: PickerRow, TabButton
script: scripts/new_job_window.luau
handlers: cancel, field, njob_back, njob_create, njob_git_provider_pick, njob_git_repo_pick, njob_kind, njob_pick_no_service, njob_pick_project, njob_pick_service, njob_source, window

### new_project_form.gvb — Janela "Novo projeto": motor Glacier próprio, aberto por open_window a partir do app principal (hand…
<screen "Novo projeto — Rustploy">
script: scripts/new_project_window.luau
handlers: cancel, np_apontar, submit_project, window

### new_registry_token_window.gvb — Janela "Novo token do registry": motor Glacier próprio, aberto por open_window a partir do app princ…
<screen "Novo token — Rustploy">
imports: TabButton
script: scripts/new_registry_token_window.luau
handlers: clipboard, ntok_apontar, ntok_create, ntok_scope, window

### new_service.gvb — Wizard "Novo serviço" (view=new_service): tipo → formulário por tipo, espelhando o fluxo do antigo r…
<component>
handlers: field, ns_back, ns_broker, ns_cancel, ns_create, ns_db, ns_kind, ns_pick_template, ns_tsearch

### new_service_window.gvb — Janela do wizard "Novo serviço": motor Glacier próprio, aberto por open_window (handlers/wizard.luau…
<screen "Novo serviço — Rustploy">
imports: NewServiceWizard, PickerRow, TemplateRow
script: scripts/new_service_window.luau
handlers: window

### service.gvb — Detalhe de um serviço: cabeçalho com ações (deploy, stop, reload) e as abas General (fonte), Connect…
<component>
imports: ServiceAdvancedTab, ServiceConnectionTab, ServiceDatabasesTab, ServiceDeploymentsTab, ServiceDomainsTab, ServiceEnvironmentTab, ServiceGeneralTab, ServiceHealthcheckTab, ServiceLogsTab, ServiceMigrarTab
abas: general, connection, environment, domains, migrar, databases, deployments, healthcheck, logs, advanced
handlers: open_logs_window, open_project, svc_deploy, svc_rebuild, svc_reload, svc_stop, tab

### shell.gvb — Casca do app conectado: sidebar, topbar e as views de projeto (Deployments, Projects, serviços de um…
<component>
imports: DeploymentsView, HomeViews, LoadingRow, NavItem, PickerRow, ProjectCard, ProjectServicesView, ProjectsView, ServiceCard, ServiceDetail, StatCard, StateCell, TabButton, TemplateRow, rp-badge
views: deployments, projects, project_services, service
handlers: disconnect, drawer, nav_deploy_engine, nav_deployments, nav_docker, nav_ingress, nav_monitoring, nav_projects, nav_schedules, nav_settings, nav_support, search_changed, stop_all

## rustploy-gui/views/components/

### badge.gvb — Variante "badge" da célula de estado (mesmo ponto + rótulo, mas com o espaçamento/estilo de crachá —…
<component>
props: color, label

### loading_row.gvb — Linha "Carregando dados…" com spinner.
<component>
props: note

### nav_item.gvb — Item de navegação da sidebar: ícone (sempre visível) + rótulo (some abaixo de 900px de largura — ver…
<component>
props: label, icon, target, action

### picker_row.gvb — Linha de escolha do wizard "Novo serviço": título + subtítulo à esquerda e um botão de ação à direit…
<component>
props: title, subtitle, button_label, action

### project_card.gvb — Template "fragment": dois nós de topo (o slot vazio e o card) — o glacier-ui (0.4.12+) embrulha múlt…
<component>
props: id, name, service_count, running_count, description, can_delete, filler
handlers: delete_project, open_project

### service_card.gvb — Card de serviço da aba "Serviços" de um projeto — o análogo do ProjectCard.
<component>
props: id, name, port, status_color, status_label, cpu, mem, filler, container_name, container_id, container_extra
handlers: open_service, stop_delete_service, svc_stop_id

### stat_card.gvb — Tile de KPI do cabeçalho (STATUS/UPTIME/SERVICES/CPU/…).
<component>
props: label, value, accent

### state_cell.gvb — Célula de estado das tabelas: o ponto colorido "●" + o rótulo, ambos na mesma cor.
<component>
props: kind, label

### tab_button.gvb — Botão de aba genérico.
<component>
props: label, current, target, action

### template_row.gvb — Linha do catálogo de templates de aplicação: logo à esquerda (vetor ou raster conforme logo_kind), n…
<component>
props: name, description, logo, logo_kind, action

## rustploy-gui/views/home/

### deploy_engine.gvb — Seção `deploy_engine` (view = deploy_engine) das telas globais; importada por home.gvb.
<component>
handlers: queue_cancel, queue_promote, queue_reorder, queue_toggle_pause

### docker.gvb — Seção `docker` (view = docker) das telas globais; importada por home.gvb.
<component>
imports: DockerContainersTab, DockerImagesTab, DockerNetworksTab, DockerRegistryTab, DockerVolumesTab
abas: containers, images, volumes, networks, registry
handlers: docker_tab

### ingress.gvb — Seção `ingress` (view = ingress) das telas globais; importada por home.gvb.
<component>

### monitoring.gvb — Seção `monitoring` (view = monitoring) das telas globais; importada por home.gvb.
<component>

### schedules.gvb — Seção `schedules` (view = schedules) das telas globais; importada por home.gvb.
<component>
handlers: job_del, job_run_cancel, job_run_now, job_toggle, open_edit_job_window, open_job_logs_window, open_new_job_window

### settings.gvb — Seção `settings` (view = settings) das telas globais; importada por home.gvb.
<component>
imports: SettingsGitTab, SettingsIacTab, SettingsMaintenanceTab, SettingsWebTab
abas: web, git, iac, maintenance
handlers: settings_tab

## rustploy-gui/views/home/docker/

### containers.gvb — Containers — um por serviço gerido pelo Rustploy (ligação com projeto/serviço é direta, já que o con…
<component>
handlers: docker_prune_containers, docker_rm_container

### images.gvb — Images — projeto/serviço é melhor esforço (inferido pela tag; imagens manuais/de terceiros ficam com…
<component>
handlers: docker_prune_images, docker_rm_image, field

### networks.gvb — Networks — projeto reconhecido pela convenção rp_net_&lt;id curto&gt;.
<component>
handlers: docker_prune_networks, docker_rm_network, field

### registry.gvb — Registry — repositórios/tags do registry OCI embutido (Fase 1: só push/pull via docker CLI; sem auth…
<component>
handlers: registry_close_repo, registry_gc, registry_open_repo, registry_open_token_window, registry_rm_repo, registry_rm_tag, registry_rm_token

### volumes.gvb — Volumes — Rustploy só usa bind mounts, então volumes nomeados aqui são sempre externos ao Rustploy (…
<component>
handlers: docker_prune_volumes, docker_rm_volume, field

## rustploy-gui/views/home/settings/

### git.gvb — Git: contas conectadas + formulário de conexão
<component>
handlers: clipboard, field, gp_connect, gp_delete, gp_kind, gp_mode, gp_refresh, open

### iac.gvb — Infra as Code: o manifesto é um `.zip` com exatamente rustploy.yml (projetos/serviços, env vars semp…
<component>
handlers: field, iac_export, iac_import

### maintenance.gvb — Manutenção: limpeza automática de recursos Docker sem uso (ver docs/plano-limpeza-automatica-docker.…
<component>
handlers: dc_kind, dc_run_now, dc_save, field

### web.gvb — Web Server (default)
<component>
handlers: clipboard, settings_save

## rustploy-gui/views/service/

### advanced.gvb
<component>
handlers: adv_save, field, pdc_add, pdc_del, pdc_reorder

### connection.gvb — Connection (valores copiáveis)
<component>
handlers: clipboard, shared_disable, shared_enable

### databases.gvb — Databases (servidor de banco compartilhado entre projetos)
<component>
handlers: clipboard, field, mdb_create, mdb_delete, mdb_show

### deployments.gvb — Deployments
<component>
handlers: clipboard, delete_deployment, dep_logs, regen_webhook

### domains.gvb — Domains (editável): lista de rotas HTTP (domínio → porta de container, TLS por rota), mais o form de…
<component>
handlers: dom_add, dom_del, dom_hostport_auto, dom_hostport_save, field

### environment.gvb — Environment
<component>
handlers: env_add, env_del, env_export, env_import, env_reorder, env_text_cancel, env_text_toggle

### general.gvb — General (source / build, editável)
<component>
imports: GeneralCompose, GeneralGit, GeneralGitea, GeneralZip
abas: git, zip, gitea
handlers: gen_save, prov, save_service_name, svc_rename_apontar

### general_compose.gvb — Editor do YAML de um serviço Compose (bancos/brokers); o corpo do `if @svc_source_kind == "Compose"`…
<component>
handlers: compose_cancel, compose_save

### general_git.gvb — Sub-aba Git do provider: URL/imagem crua; corpo do `if @prov_tab == "git"` de general.gvb.
<component>
handlers: field, gen_save

### general_gitea.gvb — Sub-aba conta conectada (Gitea/GitHub): picker conta/repo/branch; corpo do `if @prov_tab == "gitea"`…
<component>
handlers: field, gen_save, gitea_provider_pick, gitea_repo_pick

### general_zip.gvb — Sub-aba Zip do provider: upload local com Dockerfile na raiz; corpo do `if @prov_tab == "zip"` de ge…
<component>
handlers: archive_upload, field, gen_save

### healthcheck.gvb — Healthcheck (editável)
<component>
handlers: field, hc_save, hckind

### logs.gvb — Aba Logs do detalhe do serviço (importada por service.gvb).
<component>
handlers: open_logs_window

### migrar.gvb — Migrar este banco para um servidor compartilhado
<component>
handlers: field, mig_discard, mig_refresh, mig_rollback, mig_start

## rustploy-gui/views/shell/

### deployments.gvb — Lista de deployments (view = deployments); importada por shell.gvb.
<component>
handlers: deployments_clear_finished

### project_env.gvb — Aba Variáveis do projeto.
<component>
handlers: penv_add, penv_apontar, penv_del, penv_export, penv_import, penv_reorder, penv_secret_toggle, penv_text_cancel, penv_text_toggle, secret_use

### project_jobs.gvb — Aba Jobs do projeto.
<component>
handlers: job_del, job_run_cancel, job_run_now, job_toggle, open_edit_job_window, open_job_logs_window, open_new_job_window

### project_secrets.gvb — Aba Secrets do projeto; a condição de carregamento vai na chamada.
<component>
handlers: secret_add, secret_add_apontar, secret_del

### project_services.gvb — Projeto aberto (view = project_services): cabeçalho/edição, sub-abas e grade de serviços; cada sub-a…
<component>
imports: ProjectEnvTab, ProjectJobsTab, ProjectSecretsTab
abas: env, secrets, jobs, services
handlers: cancel_project_edit, delete_project, edit_project_toggle, nav_projects, open_new_service_window, proj_edit_apontar, proj_tab, save_project_edit

### projects.gvb — Lista de projetos (view = projects); importada por shell.gvb.
<component>
handlers: open_new_project_window

## rustploy-gui/views/styles/

### app.gss — Rustploy — glacier-ui stylesheet.
