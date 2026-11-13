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

## / que não é do projeto ou repetido é erro. Response: `Ok`.
ProjectServicesReorder — Define a ordem de exibição dos serviços do projeto: `service_ids` na o… | daemon: projects/project_services_reorder | gui: services_reorder_window | web: app
ProjectEnvSet | daemon: projects/project_env_set | gui: handlers/projects | web: app

## Services
ServiceCreate | daemon: features/services/create | gui: — | web: app
ServiceUpdate | daemon: features/services/update | gui: handlers/services | web: app
ServiceDelete | daemon: features/services/delete | gui: handlers/projects | web: app
ServiceList | daemon: services/service_list | gui: handlers/services | web: —
ServiceGet | daemon: services/service_get | gui: handlers/services | web: app

## Deployments
DeployStart | daemon: features/deploy/start | gui: handlers/services | web: app

## / `skip_running`, as dependências que já estão no ar não entram na fila. Resposta: `DeployBatch`.
DeployWithDependencies — Faz o deploy de um serviço **e da árvore de dependências dele**: enfil… | daemon: features/deploy/tree/deploy_with_dependencies | gui: handlers/services | web: app

## / inteiro). Resposta: `DeployBatch`.
ProjectDeploy — Faz o deploy de **todos os serviços do projeto**, cada um com as suas … | daemon: features/deploy/tree/project_deploy | gui: handlers/services | web: app

## / É o que a tela mostra antes de a pessoa confirmar. Resposta: `DeployBatch` (sem `deployment_id`).
DeployWithDependenciesPlan — **Só mostra** o plano de `DeployWithDependencies`, sem enfileirar nada… | daemon: features/deploy/tree/plan_for_service | gui: handlers/services | web: app

## / Como `DeployWithDependenciesPlan`, para o `ProjectDeploy` (o projeto inteiro).
ProjectDeployPlan — Como `DeployWithDependenciesPlan`, para o `ProjectDeploy` (o projeto i… | daemon: features/deploy/tree/plan_for_project | gui: handlers/services | web: app
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

## / serviços Compose (só ao vivo, por `LogsGet`).
LogsHistory — Log de runtime **persistido** do serviço (sobrevive à troca de contain… | daemon: observability/logs_history | gui: handlers/services, log_window | web: app

## / Não cobre serviços Compose.
LogsExecutions — As execuções (um container = uma execução) com log guardado, da mais n… | daemon: observability/logs_executions::list | gui: handlers/services | web: app

## / recusada (`ExecutionRunning`); id inexistente é ignorado. Resposta: `LogsExecutionsDeleted`.
LogsExecutionDelete — Apaga o log guardado de execuções (irreversível; vários de uma vez). | daemon: observability/logs_executions::delete | gui: handlers/services | web: app
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

## não mandam o campo, continuam falando só do escopo de projeto.
SecretSet | daemon: secrets/secret_set | gui: handlers/secrets, handlers/services, job_env_core | web: app, screens/job_env, screens/service_detail
SecretDelete | daemon: secrets/secret_delete | gui: handlers/secrets, handlers/services, job_env_core | web: app, screens/job_env, screens/service_detail
SecretList | daemon: secrets/secret_list | gui: handlers/services, job_env_core | web: screens/job_env, screens/service_detail

## / grava uma linha de auditoria (quem, qual secret, quando — nunca o valor).
SecretReveal — Devolve o **valor** de um secret (botão "ver valor"). | daemon: secrets/secret_reveal | gui: handlers/secrets, handlers/services, job_env_core | web: screens/job_env, screens/project_detail, screens/service_detail

## / sincronização com git, e o guia de "o que ainda falta mover para secret". Nunca devolve valores.
InfraAudit — Lista o que está em claro (ou parece estar) no estado de infra — a tra… | daemon: secrets/infra_audit | gui: — | web: —

## / (ver `InfraAudit`): o download não pode vazar o que o git não aceitaria.
InfraDownload — Baixa a infra: os mesmos arquivos que o `state_git` grava no repositór… | daemon: secrets/infra_download | gui: handlers/settings | web: app

## / Situação da sincronização do estado de infra com o git.
StateGitStatus — Situação da sincronização do estado de infra com o git. | daemon: (inline em dispatch.rs) | gui: — | web: —

## / `https://` (a credencial vem do `provider_id`) ou `file://` (testes/cópia local).
StateGitConfigure — Liga/desliga/configura a sincronização. | daemon: (inline em dispatch.rs) | gui: handlers/state_git | web: app

## / fora e o `push`.
StateGitSyncNow — "Sincronizar agora": faz o commit pendente e, em `sync`, o `fetch`, a … | daemon: (inline em dispatch.rs) | gui: handlers/state_git | web: app

## / Retoma a sincronização pausada por histórico divergido (depois de resolver no repositório).
StateGitResume — Retoma a sincronização pausada por histórico divergido (depois de reso… | daemon: (inline em dispatch.rs) | gui: handlers/state_git | web: app

## / do que for descartado (ver [`StateGitResolveChoice`]). Só vale com a sincronização pausada.
StateGitResolve — Resolve a divergência **sem usar o terminal**: descarta um dos lados, … | daemon: (inline em dispatch.rs) | gui: handlers/state_git | web: app

## / Esquece uma marca de "removido no git" (`id` vem em `StateGitStatus.removed_in_git`).
StateGitDismissRemoval — Esquece uma marca de "removido no git" (`id` vem em `StateGitStatus.re… | daemon: (inline em dispatch.rs) | gui: handlers/state_git | web: app

## / Certezas (senha de banco, webhook…) não podem ser liberadas.
InfraAuditAllow — Marca (ou desmarca) uma **suspeita** da auditoria como "não é segredo"… | daemon: (inline em dispatch.rs) | gui: handlers/state_git | web: app

## / ("removido no git"): parar aqui e depois `RemoveContainer`.
StopContainer — Para um container (`id` completo, vindo de `DockerContainers`). | daemon: docker/docker_remove::stop_container | gui: handlers/docker | web: app

## / Falha se o destino já tem um item com o mesmo nome. Ver [`SecretMoveOp`].
SecretMove — Move um item entre env e secret, ou entre escopos. | daemon: secrets/secret_move | gui: handlers/secrets, handlers/services, job_env_core | web: screens/job_env, screens/project_detail, screens/service_detail

## / Resposta: `Ok`.
EnvKeyRename — Renomeia a **chave** de uma env (do projeto, ou do serviço se `service… | daemon: secrets/env_key_rename | gui: handlers/secrets, handlers/services, job_env_core | web: screens/job_env, screens/project_detail, screens/service_detail
EnvValueSet — Troca só o **valor** de uma env existente (do projeto, ou do serviço s… | daemon: secrets/env_value_set | gui: handlers/secrets, job_env_core | web: screens/job_env, screens/project_detail, screens/service_detail

## / Falha se `from` não existe ou `to` já existe no escopo. Resposta: `Ok`.
SecretRename — Renomeia um **secret** (do projeto, ou do serviço se `service_id` vier… | daemon: secrets/secret_rename | gui: handlers/secrets, handlers/services, job_env_core | web: screens/job_env, screens/project_detail, screens/service_detail

## / `ServiceExportPlan`.
ServiceExportPlan — O que dá para levar do serviço: suas variáveis e as do projeto (com a … | daemon: features/iac/service_export/plan | gui: handlers/bundle | web: screens/service_bundle

## / `ServiceBundleYaml`.
ServiceExport — Gera o pacote (YAML) do serviço. | daemon: features/iac/service_export/export | gui: service_export_window | web: screens/service_bundle

## / Resposta: `ServiceDataPlan`.
ServiceDataPlan — Dados do serviço (volumes): o que um export levaria. | daemon: (inline em dispatch.rs) | gui: handlers/services | web: screens/service_detail

## / O destino S3 dos backups de dados (sem a chave secreta). Resposta: `BackupDestination`.
BackupDestinationGet — O destino S3 dos backups de dados (sem a chave secreta). | daemon: data_backup/backup_destination_get | gui: handlers/data_backup | web: app

## / conecta (use `BackupDestinationTest`). Resposta: `BackupDestination`.
BackupDestinationSet — Grava o destino S3. | daemon: data_backup/backup_destination_set | gui: handlers/data_backup | web: app

## / Apaga o destino gravado (os backups já enviados ficam no S3). Resposta: `Ok`.
BackupDestinationClear — Apaga o destino gravado (os backups já enviados ficam no S3). | daemon: data_backup/backup_destination_clear | gui: handlers/data_backup | web: app

## / valor, testa o rascunho da tela (e `secret_key: None` usa a chave gravada). Resposta: `Ok`.
BackupDestinationTest — Testa a conexão: grava, lê e apaga um objeto pequeno. | daemon: data_backup/backup_destination_test | gui: handlers/data_backup | web: app

## / Os backups de dados de um serviço no S3 e a última operação. Resposta: `ServiceBackups`.
ServiceBackupList — Os backups de dados de um serviço no S3 e a última operação. | daemon: data_backup/service_backup_list | gui: handlers/service_backup | web: screens/service_detail

## / `true`; banco não para). `keep`: depois de enviar, manter só os `keep` mais novos.
ServiceBackupNow — Faz o backup dos dados do serviço para o S3 **em segundo plano** (volu… | daemon: data_backup/service_backup_now | gui: handlers/service_backup | web: screens/service_detail

## / caminho e a mesma rede de segurança da importação). Exige `confirm: true`. Resposta: `Ok`.
ServiceBackupRestore — Restaura um backup do S3 **substituindo** os dados atuais do serviço, … | daemon: data_backup/service_backup_restore | gui: handlers/service_backup | web: screens/service_detail

## / Apaga um backup do S3. Resposta: `Ok`.
ServiceBackupDelete — Apaga um backup do S3. | daemon: data_backup/service_backup_delete | gui: handlers/service_backup | web: screens/service_detail

## / disparo).
ServiceBackupPolicySet — Grava (ou, com `None`, apaga) a política de backup automático do servi… | daemon: data_backup/service_backup_policy_set | gui: handlers/service_backup | web: screens/service_detail

## / `ServerRecover` (`phase: "ready"`, com o que foi achado e os backups por serviço).
ServerRecoverStart — **Recuperar este servidor** (passo 1): num servidor **limpo**, traz a … | daemon: data_backup/server_recover::start | gui: handlers/recover | web: app

## / Em que ponto a recuperação está. Resposta: `ServerRecover`.
ServerRecoverStatus — Em que ponto a recuperação está. | daemon: data_backup/server_recover::status | gui: handlers/recover | web: app

## / Resposta: `Ok`.
ServerRecoverRun — Recuperação (passo 2): restaura os dados e sobe cada serviço na ordem … | daemon: data_backup/server_recover::run | gui: handlers/recover | web: app

## / Resposta: `ServerRecover`.
ServerRecoverCancel — Desiste: esquece a chave de recuperação da memória. | daemon: data_backup/server_recover::cancel | gui: handlers/recover | web: app

## / Plano: `docs/plano-chave-de-recuperacao.md`.
RecoveryKeyStatus — A chave **pública** de recuperação deste servidor (`[secrets] recovery… | daemon: data_backup/recovery_key_status | gui: handlers/recovery_key | web: app

## / `RecoveryKeyGenerated`.
RecoveryKeyGenerate — **Gera** a chave de recuperação: grava só a pública no `config.toml` e… | daemon: data_backup/recovery_key_generate | gui: handlers/recovery_key | web: app

## / `dry_run` só devolve o relatório. Resposta: `ServiceImportReport`.
ServiceImport — Cria UM serviço novo a partir de um pacote, sem sobrescrever nada. | daemon: features/iac/service_import | gui: handlers/bundle_import | web: screens/service_bundle

## Jobs (tarefas one-shot via docker-compose, agendadas ou manuais)
JobCreate | daemon: jobs/job_create | gui: new_job_window | web: app
JobUpdate | daemon: jobs/job_update | gui: handlers/jobs, job_env_core, new_job_window | web: app, screens/job_env
JobDelete | daemon: jobs/job_delete | gui: handlers/jobs | web: app

## / Jobs de um único projeto (aba "Jobs" do projeto).
JobList — Jobs de um único projeto (aba "Jobs" do projeto). | daemon: jobs/job_list | gui: handlers/services, job_env_core | web: —

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
ServiceWarm — Aquece o serviço: constrói a imagem dele na vaga única da fila, para o… | daemon: docker/docker_cleanup::service_warm | gui: handlers/services | web: app

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

## / Resposta: `UpdateInfo` (ou `Err` com `UpdateCheckFailed` se o GitHub não responder).
CheckUpdate — Pergunta ao GitHub se há release mais nova que o daemon. | daemon: observability/check_update | gui: handlers/settings | web: app

## / O agente MCP só pode usar se `[update] allow_agent_apply = true`. Resposta: `Ok`.
ApplyUpdate — Atualiza o daemon para a última release **estável** (a que o `CheckUpd… | daemon: observability/apply_update | gui: handlers/settings | web: app

## / Estado da atualização em curso, ou o resultado da última. Resposta: `UpdateProgress`.
UpdateStatus — Estado da atualização em curso, ou o resultado da última. | daemon: observability/update_status | gui: handlers/settings | web: app

## / uma versão menor). Mesma trava do agente MCP que `ApplyUpdate`. Resposta: `Ok`.
UpdateRollback — Volta os binários para os do backup da última atualização (o único cam… | daemon: observability/update_status::rollback | gui: handlers/settings | web: app

## Git providers (Gitea OAuth2 / PAT)
GitProviderList | daemon: git/git_provider_list | gui: handlers/bundle_import, handlers/connection, handlers/jobs, handlers/services, handlers/settings, new_job_window | web: app, screens/service_bundle, screens/service_detail
GitProviderCreate | daemon: git/git_provider_create | gui: handlers/settings | web: app

## / Os clientes avisam o usuário antes e só então mandam `force`. Resposta: `Ok`.
GitProviderDelete — Remove uma conta git. | daemon: git/git_provider_delete | gui: handlers/settings | web: app

## / Returns the Gitea authorization URL for the client to open in a browser.
GitOAuthStart — Returns the Gitea authorization URL for the client to open in a browse… | daemon: git/git_oauth_start | gui: handlers/settings | web: app
GitRepoList | daemon: git/git_repo_list | gui: handlers/jobs, handlers/services, new_job_window | web: app, screens/service_detail
GitBranchList | daemon: git/git_branch_list | gui: handlers/jobs, handlers/services, new_job_window | web: app, screens/service_detail

## acesso aos blueprints de `templates`).
WizardCatalog | daemon: services/wizard::catalog | gui: handlers/wizard | web: app
WizardCreate | daemon: services/wizard::create | gui: handlers/wizard | web: app

## / mudança na hora, sem esperar o próximo tick. Resposta: `Snapshot(String)`.
Snapshot — Snapshot completo do dashboard como JSON (o mesmo que o SSE `/api/even… | daemon: (inline em dispatch.rs) | gui: handlers/history, handlers/stream | web: app

## delete; criar conteúdo só acontece via `docker push` externo).
RegistryStatus | daemon: registry/commands::status | gui: — | web: —
RegistryRepoList | daemon: registry/commands::repo_list | gui: — | web: —
RegistryTagList | daemon: registry/commands::tag_list | gui: handlers/registry, handlers/services | web: app, screens/service_detail

## / digest, não por tag).
RegistryTagDelete — Remove a tag; se outra tag apontar pro mesmo digest (mesmo manifest), … | daemon: registry/commands::tag_delete | gui: handlers/registry | web: app
RegistryRepoDelete | daemon: registry/commands::repo_delete | gui: handlers/registry | web: app

## / `RegistryGcResult`.
RegistryGc — Garbage collection do registry: remove manifests pendurados (sem tag e… | daemon: registry/commands::gc | gui: handlers/registry | web: app

## / A leitura vem em `RegistryStatus(.retention)`. Resposta: `Ok`.
RegistryRetentionSet — Grava a política de retenção de tags (ligar/desligar e quantas manter)… | daemon: registry/commands::retention_set | gui: handlers/registry | web: app

## / Resposta traz o segredo em texto plano UMA ÚNICA VEZ.
RegistryTokenCreate — Cria um token de acesso (Basic auth) — `scope` é `"pull"` ou `"push"`. | daemon: registry/commands::token_create | gui: new_registry_token_window | web: app
RegistryTokenList | daemon: registry/commands::token_list | gui: handlers/registry | web: app
RegistryTokenRevoke | daemon: registry/commands::token_revoke | gui: handlers/registry | web: app

## / Resposta: `Ok`.
DeployQueuePromote — Move um deploy enfileirado para o início da fila ("furar fila"). | daemon: deploys/deploy_queue_promote | gui: handlers/deploy_queue | web: app
DeployQueueReorder — Reordena a fila para exatamente a ordem dada (ids de deployment). | daemon: deploys/deploy_queue_reorder | gui: handlers/deploy_queue | web: app
DeployQueuePause — Pausa (`true`) ou retoma (`false`) a fila global. | daemon: deploys/deploy_queue_pause | gui: handlers/deploy_queue | web: app

## Paridade

- Só GUI (2): ServiceList, JobList
- Só webui (2): ServiceCreate, DeployRollback
- Nem GUI nem webui (28): RecentDeployments, LogsSubscribe, LogsUnsubscribe, MetricsSubscribe, MetricsUnsubscribe, SharedAccessList, SharedAccessGrant, SharedAccessRevoke, MigrationGet, InfraAudit, StateGitStatus, JobListAll, JobRunHistory, JobWarmStatus, ServiceWarmStatusList, PruneBuildCache, DockerImages, DockerVolumes, DockerNetworks, DockerContainers, IngressRoutes, IngressReconcile, EnvBackupList, EnvBackupRestore, Ping, DeployEngineStatus, RegistryStatus, RegistryRepoList
