# Índice: Command → handler no daemon → quem chama

> Gerado por `make index`; não editar à mão. Uma linha por variante do
> `enum Command` (`rustploy-shared/src/protocol/command.rs`), agrupada pelos comentários do enum.
> `daemon:` = `<assunto>/<arquivo>` sob `rustploy-daemon/crates/daemon/src/api/handlers/` (ou `features/…` quando o handler delega a lógica), `::fn` quando não é `handle`;
> `gui:` = arquivos sob `rustploy-gui/views/scripts/`; `web:` = sob `rustploy-daemon/crates/daemon/webui/`; `—` = ninguém;
> `agente:` (só quando há) = sob `rustploy-gui/src/agent/` (API de agente da GUI; `catalog` = só documentado).

## Projects
ProjectCreate | daemon: projects/project_create | gui: handlers/projects, new_project_window | web: app
ProjectDelete | daemon: projects/project_delete | gui: handlers/projects | web: app
ProjectUpdate | daemon: projects/project_update | gui: handlers/projects | web: app
ProjectList | daemon: projects/project_list | gui: handlers/projects, handlers/services | web: screens/service_detail
ProjectEnvSet | daemon: projects/project_env_set | gui: handlers/projects | web: app

## Services
ServiceCreate | daemon: features/services/create | gui: — | web: app
ServiceUpdate | daemon: features/services/update | gui: handlers/services | web: app
ServiceDelete | daemon: features/services/delete | gui: handlers/projects | web: app
ServiceList | daemon: services/service_list | gui: — | web: —
ServiceGet | daemon: services/service_get | gui: handlers/services | web: app

## Deployments
DeployStart | daemon: features/deploy/start | gui: handlers/services | web: app
DeployAbort | daemon: deploys/deploy_abort | gui: handlers/deploy_queue | web: app
DeployRollback | daemon: deploys/deploy_rollback | gui: — | web: app
DeployHistory | daemon: deploys/deploy_history | gui: handlers/services | web: app
DeployDelete | daemon: deploys/deploy_delete | gui: handlers/services, handlers/stream | web: app

## Service lifecycle
ServiceStop | daemon: features/deploy/stop | gui: handlers/projects, handlers/services | web: app
ServiceReload | daemon: services/service_reload | gui: handlers/services | web: app

## Global views
RecentDeployments | daemon: deploys/recent_deployments | gui: — | web: —
GetBuildLogs | daemon: deploys/get_build_logs | gui: handlers/services | web: screens/service_detail

## Observability
LogsGet | daemon: observability/logs_get | gui: handlers/services | web: app
LogsSubscribe | daemon: (inline em dispatch.rs) | gui: — | web: —
LogsUnsubscribe | daemon: (inline em dispatch.rs) | gui: — | web: —
MetricsSubscribe | daemon: (inline em dispatch.rs) | gui: — | web: —
MetricsUnsubscribe | daemon: (inline em dispatch.rs) | gui: — | web: —

## / daemon por `shared::connection`, para GUI e webui não divergirem).
ServiceConnectionInfo — Host e URL de conexão **dentro** da rede Docker do projeto (montada no… | daemon: services/service_connection_info | gui: handlers/services | web: screens/service_detail

## / Projetos autorizados a alcançar um servidor de banco compartilhado.
SharedAccessList — Projetos autorizados a alcançar um servidor de banco compartilhado. | daemon: databases/shared_access::list | gui: — | web: —

## / Autoriza um projeto: conecta o servidor à rede dele sob o alias global.
SharedAccessGrant — Autoriza um projeto: conecta o servidor à rede dele sob o alias global… | daemon: databases/shared_access::grant | gui: — | web: —
SharedAccessRevoke | daemon: databases/shared_access::revoke | gui: — | web: —

## / Databases gerenciados de um servidor compartilhado.
ManagedDatabaseList — Databases gerenciados de um servidor compartilhado. | daemon: databases/managed_database::list | gui: handlers/services | web: screens/service_detail

## / Cria database + usuário, autoriza o projeto e grava a env var nele.
ManagedDatabaseCreate — Cria database + usuário, autoriza o projeto e grava a env var nele. | daemon: databases/managed_database::create | gui: handlers/services | web: screens/service_detail

## / Remove database + usuário do servidor (**apaga os dados**).
ManagedDatabaseDelete — Remove database + usuário do servidor (**apaga os dados**). | daemon: databases/managed_database::delete | gui: handlers/services | web: screens/service_detail

## / Todos os databases gerenciados (de todos os servidores).
ManagedDatabaseListAll — Todos os databases gerenciados (de todos os servidores). | daemon: databases/managed_database::list_all | gui: handlers/services | web: screens/service_detail

## / Inicia a migração (corre em segundo plano; acompanhe com `MigrationGet`).
MigrationStart — Inicia a migração (corre em segundo plano; acompanhe com `MigrationGet… | daemon: databases/migration::start | gui: handlers/services | web: screens/service_detail
MigrationGet | daemon: databases/migration::get | gui: — | web: —
MigrationList | daemon: databases/migration::list | gui: handlers/services | web: screens/service_detail

## / Volta a app para o banco antigo (env var + serviços).
MigrationRollback — Volta a app para o banco antigo (env var + serviços). | daemon: databases/migration::rollback | gui: handlers/services | web: screens/service_detail

## / Remove o banco antigo (**apaga os dados dele**) após o período de observação.
MigrationDiscard — Remove o banco antigo (**apaga os dados dele**) após o período de obse… | daemon: databases/migration::discard | gui: handlers/services | web: screens/service_detail

## Webhooks
GetWebhookUrl | daemon: services/get_webhook_url | gui: handlers/services | web: screens/service_detail
RegenerateWebhookToken | daemon: services/regenerate_webhook_token | gui: handlers/services | web: screens/service_detail
GetDaemonSettings | daemon: settings/get_daemon_settings | gui: handlers/connection | web: app
SetDaemonSettings | daemon: settings/set_daemon_settings | gui: handlers/settings | web: app

## Secrets
SecretSet | daemon: secrets/secret_set | gui: handlers/secrets | web: app
SecretDelete | daemon: secrets/secret_delete | gui: handlers/secrets | web: app
SecretList | daemon: secrets/secret_list | gui: — | web: —

## / num lugar só, com `serde_yaml`. Ver `docs/infra-as-code.md`.
ManifestApply — Reconcilia projetos/serviços a partir de manifestos YAML já interpolad… | daemon: features/iac/manifest_apply | gui: — | web: —

## / Exporta o estado atual de um projeto como manifesto YAML (secrets redigidos).
ManifestExport — Exporta o estado atual de um projeto como manifesto YAML (secrets redi… | daemon: features/iac/manifest_export | gui: — | web: —

## / com os valores reais das vars `Plain`).
ManifestExportAll — Exporta TODOS os projetos+serviços num único manifesto raiz (`ServerMa… | daemon: features/iac/manifest_export_all | gui: handlers/settings | web: app

## / padrão; `prune`/`deploy` com a mesma semântica).
ManifestImport — Importa um manifesto raiz (`projects:`) ou de projeto único (`project:… | daemon: features/iac/manifest_import | gui: handlers/settings | web: app

## / `ServiceExportPlan`.
ServiceExportPlan — O que dá para levar do serviço: suas variáveis e as do projeto (com a … | daemon: features/iac/service_export/plan | gui: handlers/bundle | web: screens/service_bundle

## / `ServiceBundleYaml`.
ServiceExport — Gera o pacote (YAML) do serviço. | daemon: features/iac/service_export/export | gui: service_export_window | web: screens/service_bundle

## / `dry_run` só devolve o relatório. Resposta: `ServiceImportReport`.
ServiceImport — Cria UM serviço novo a partir de um pacote, sem sobrescrever nada. | daemon: features/iac/service_import | gui: handlers/bundle_import | web: screens/service_bundle

## Jobs (tarefas one-shot via docker-compose, agendadas ou manuais)
JobCreate | daemon: jobs/job_create | gui: new_job_window | web: app
JobUpdate | daemon: jobs/job_update | gui: handlers/jobs, new_job_window | web: app
JobDelete | daemon: jobs/job_delete | gui: handlers/jobs | web: app

## / Jobs de um único projeto (aba "Jobs" do projeto).
JobList — Jobs de um único projeto (aba "Jobs" do projeto). | daemon: jobs/job_list | gui: handlers/services | web: —

## / Todos os jobs, de todos os projetos (tela global da sidebar "Schedules").
JobListAll — Todos os jobs, de todos os projetos (tela global da sidebar "Schedules… | daemon: jobs/job_list_all | gui: — | web: —

## / Dispara o job imediatamente, fora do agendamento.
JobRunNow — Dispara o job imediatamente, fora do agendamento. | daemon: jobs/job_run_now | gui: handlers/jobs | web: app

## / (terminou, ou o id nunca existiu).
JobRunCancel — Cancela um `job_run` em andamento: mata o processo `docker compose up`… | daemon: jobs/job_run_cancel | gui: handlers/jobs | web: app
JobRunHistory | daemon: jobs/job_run_history | gui: — | web: —
GetJobLogs | daemon: jobs/get_job_logs | gui: handlers/jobs | web: app

## / e o cache de build dele ainda estão no host. Resposta: `JobWarmStatus`.
JobWarmStatus — Fonte única da verdade de "este job está quente?": confere se as image… | daemon: jobs/job_warm_status | gui: — | web: —

## / Resposta: `JobWarmRequested`.
JobWarm — Aquece o job (baixa as imagens que faltam e popula o cache de build) s… | daemon: jobs/job_warm | gui: handlers/jobs | web: app

## / fonte Git ou zip. Resposta: `ServiceWarmStatuses`.
ServiceWarmStatusList — O status de aquecimento (`ServiceWarmStatus`) de todos os serviços com… | daemon: docker/docker_cleanup::service_warm_status_list | gui: — | web: —

## / `Event::ServiceWarmChanged`. Resposta: `ServiceWarmRequested`.
ServiceWarm — Aquece o serviço: constrói a imagem dele na vaga única da fila, para o… | daemon: docker/docker_cleanup::service_warm | gui: — | web: —

## Docker cleanup
PruneContainers | daemon: docker/docker_prune::prune_containers | gui: handlers/docker | web: app

## / `docker volume prune --all`); `false` é o padrão do Docker.
PruneVolumes — `all=true` remove volumes mesmo que não sejam anônimos (equivalente ao… | daemon: docker/docker_prune::prune_volumes | gui: handlers/docker | web: app

## / (equivalente ao `docker image prune -a`); `false` é o padrão do Docker.
PruneImages — `all=true` remove toda imagem sem uso, não só as dangling/untagged (eq… | daemon: docker/docker_prune::prune_images | gui: handlers/docker | web: app
PruneBuildCache | daemon: docker/docker_prune::prune_build_cache | gui: — | web: —
PruneNetworks | daemon: docker/docker_prune::prune_networks | gui: handlers/docker | web: app

## "Executar agora" da tela).
DockerCleanupConfigGet | daemon: docker/docker_cleanup::get_config | gui: handlers/settings | web: app
DockerCleanupConfigSet | daemon: docker/docker_cleanup::set_config | gui: handlers/settings | web: app
DockerCleanupRunNow | daemon: docker/docker_cleanup::run_now | gui: handlers/settings | web: app

## rustploy-managed ones — see `shared::DockerImageInfo` etc.)
DockerImages | daemon: docker/docker_inventory::list_images | gui: — | web: —
DockerVolumes | daemon: docker/docker_inventory::list_volumes | gui: — | web: —
DockerNetworks | daemon: docker/docker_inventory::list_networks | gui: — | web: —

## / Todo container do host (rodando + parado), para a sub-aba Containers.
DockerContainers — Todo container do host (rodando + parado), para a sub-aba Containers. | daemon: docker/docker_inventory::list_containers | gui: — | web: —

## O Docker recusa remover recursos em uso (sem force) — o erro é propagado.
RemoveContainer | daemon: docker/docker_remove::remove_container | gui: handlers/docker | web: app
RemoveImage | daemon: docker/docker_remove::remove_image | gui: handlers/docker | web: app
RemoveVolume | daemon: docker/docker_remove::remove_volume | gui: handlers/docker | web: app
RemoveNetwork | daemon: docker/docker_remove::remove_network | gui: handlers/docker | web: app

## / touches unrelated containers on the same Docker host.
StopAllManaged — Stops every container labeled `rustploy.managed=true`, regardless of w… | daemon: docker/docker_inventory::stop_all_managed | gui: handlers/projects | web: app

## / 502: a rota existe? para qual `ip:porta` ela aponta?
IngressRoutes — Lê a tabela de rotas do ingress (domínios + portas). | daemon: observability/ingress::routes | gui: — | web: —

## / É o mesmo caminho do reconcile de boot, exposto sob demanda.
IngressReconcile — Recalcula as rotas a partir dos containers que realmente existem no Do… | daemon: observability/ingress::reconcile | gui: — | web: —

## / Lista os snapshots disponíveis (retorna Vec<String> com nomes de ficheiro).
EnvBackupList — Lista os snapshots disponíveis (retorna Vec<String> com nomes de fiche… | daemon: settings/env_backup::list | gui: — | web: —

## / Restaura o snapshot com o nome dado (caminho relativo ao backup_dir).
EnvBackupRestore — Restaura o snapshot com o nome dado (caminho relativo ao backup_dir). | daemon: settings/env_backup::restore | gui: — | web: —

## Infrastructure
Ping | daemon: observability/ping | gui: — | web: —
DaemonStatus | daemon: observability/daemon_status | gui: handlers/connection | web: app
DeployEngineStatus | daemon: deploys/deploy_engine_status | gui: — | web: —

## Git providers (Gitea OAuth2 / PAT)
GitProviderList | daemon: git/git_provider_list | gui: handlers/bundle_import, handlers/connection, handlers/jobs, handlers/services, handlers/settings, new_job_window | web: app, screens/service_bundle, screens/service_detail
GitProviderCreate | daemon: git/git_provider_create | gui: handlers/settings | web: app
GitProviderDelete | daemon: git/git_provider_delete | gui: handlers/settings | web: app

## / Returns the Gitea authorization URL for the client to open in a browser.
GitOAuthStart — Returns the Gitea authorization URL for the client to open in a browse… | daemon: git/git_oauth_start | gui: handlers/settings | web: app
GitRepoList | daemon: git/git_repo_list | gui: handlers/jobs, handlers/services, new_job_window | web: app, screens/service_detail
GitBranchList | daemon: git/git_branch_list | gui: handlers/jobs, handlers/services, new_job_window | web: app, screens/service_detail

## acesso aos blueprints de `templates`).
WizardCatalog | daemon: services/wizard::catalog | gui: handlers/wizard | web: app
WizardCreate | daemon: services/wizard::create | gui: handlers/wizard | web: app

## / mudança na hora, sem esperar o próximo tick. Resposta: `Snapshot(String)`.
Snapshot — Snapshot completo do dashboard como JSON (o mesmo que o SSE `/api/even… | daemon: (inline em dispatch.rs) | gui: handlers/stream | web: app

## delete; criar conteúdo só acontece via `docker push` externo).
RegistryStatus | daemon: registry/commands::status | gui: — | web: —
RegistryRepoList | daemon: registry/commands::repo_list | gui: — | web: —
RegistryTagList | daemon: registry/commands::tag_list | gui: handlers/registry | web: app

## / digest, não por tag).
RegistryTagDelete — Remove a tag; se outra tag apontar pro mesmo digest (mesmo manifest), … | daemon: registry/commands::tag_delete | gui: handlers/registry | web: app
RegistryRepoDelete | daemon: registry/commands::repo_delete | gui: handlers/registry | web: app

## / `RegistryGcResult`.
RegistryGc — Garbage collection do registry: remove manifests pendurados (sem tag e… | daemon: registry/commands::gc | gui: handlers/registry | web: app

## / Resposta traz o segredo em texto plano UMA ÚNICA VEZ.
RegistryTokenCreate — Cria um token de acesso (Basic auth) — `scope` é `"pull"` ou `"push"`. | daemon: registry/commands::token_create | gui: new_registry_token_window | web: app
RegistryTokenList | daemon: registry/commands::token_list | gui: handlers/registry | web: app
RegistryTokenRevoke | daemon: registry/commands::token_revoke | gui: handlers/registry | web: app

## / Resposta: `Ok`.
DeployQueuePromote — Move um deploy enfileirado para o início da fila ("furar fila"). | daemon: deploys/deploy_queue_promote | gui: handlers/deploy_queue | web: app
DeployQueueReorder — Reordena a fila para exatamente a ordem dada (ids de deployment). | daemon: deploys/deploy_queue_reorder | gui: handlers/deploy_queue | web: app
DeployQueuePause — Pausa (`true`) ou retoma (`false`) a fila global. | daemon: deploys/deploy_queue_pause | gui: handlers/deploy_queue | web: app

## Paridade

- Só GUI (1): JobList
- Só webui (2): ServiceCreate, DeployRollback
- Nem GUI nem webui (31): ServiceList, RecentDeployments, LogsSubscribe, LogsUnsubscribe, MetricsSubscribe, MetricsUnsubscribe, SharedAccessList, SharedAccessGrant, SharedAccessRevoke, MigrationGet, SecretList, ManifestApply, ManifestExport, JobListAll, JobRunHistory, JobWarmStatus, ServiceWarmStatusList, ServiceWarm, PruneBuildCache, DockerImages, DockerVolumes, DockerNetworks, DockerContainers, IngressRoutes, IngressReconcile, EnvBackupList, EnvBackupRestore, Ping, DeployEngineStatus, RegistryStatus, RegistryRepoList
