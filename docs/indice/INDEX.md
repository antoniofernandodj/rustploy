# Índice do repositório

> Gerado por `make index`; não editar à mão. Sem números de linha:
> ache o arquivo aqui, o símbolo no índice da área e a linha com
> `grep -n "fn nome" <arquivo>`. Leia só o trecho, nunca o arquivo grande inteiro.

## Índices de símbolos por área

- `shared.md` — rustploy-shared — domínio, protocolo e nomes (rustploy-shared/)
- `daemon-api.md` — daemon: API HTTP, web UI, agente MCP e handlers de Command (rustploy-daemon/crates/daemon/src/api/)
- `daemon-deploy.md` — daemon: deploy, jobs, manutenção e ciclo de vida do serviço (rustploy-daemon/crates/daemon/src/features/deploy/, rustploy-daemon/crates/daemon/src/features/jobs/, rustploy-daemon/crates/daemon/src/features/maintenance/, rustploy-daemon/crates/daemon/src/features/services/)
- `daemon-store.md` — daemon: persistência (store/) (rustploy-daemon/crates/daemon/src/store/)
- `daemon-engine.md` — daemon: motor de containers (engine/) e ingress (proxy, TLS) (rustploy-daemon/crates/daemon/src/engine/, rustploy-daemon/crates/daemon/src/features/ingress/)
- `daemon-registry.md` — daemon: registry embutido e provedores git (rustploy-daemon/crates/daemon/src/features/registry/, rustploy-daemon/crates/daemon/src/features/git_providers/)
- `daemon-features.md` — daemon: bancos, IaC, observabilidade, backup, templates e wizard (rustploy-daemon/crates/daemon/src/features/)
- `webui.md` — webui (HTML + Alpine.js) servida pelo daemon (rustploy-daemon/crates/daemon/webui/)
- `daemon-core.md` — daemon: main, bootstrap, platform (config, eventos, segredos) e o resto (rustploy-daemon/crates/daemon/)
- `gui-handlers.md` — rustploy-gui: handlers Luau (as ações que os .gvb disparam) (rustploy-gui/views/scripts/handlers/)
- `gui-scripts.md` — rustploy-gui: demais scripts Luau (estado, rede, formatação, janelas) (rustploy-gui/views/scripts/)
- `gui-views.md` — rustploy-gui: telas e componentes .gvb, estilos (rustploy-gui/views/)
- `gui.md` — rustploy-gui (Rust) (rustploy-gui/)
- `importer.md` — importer (rustploy-daemon/crates/importer/)
- `tools.md` — ferramentas do repositório (tools/)
- `comandos.md` — cada `Command`: handler no daemon + quem chama na GUI e na webui, e a paridade

## Árvore

### (raiz)
- .gitignore
- .gitmodules
- AGENTS.md — Rustploy — guia do projeto
- CHANGELOG.md — Changelog
- CLAUDE.md — CLAUDE.md
- CONTINUE.md — rustploy-gui — plano de continuação
- GEMINI.md — Rustploy
- Makefile
- README.md — Rustploy
- TODO.md — TODO
- install.ps1
- install.sh

### .claude/
- settings.json

### .github/workflows/
- ci.yml
- release.yml

### .vscode/
- settings.json

### docs/
- api-agente-no-gui.md — A API de agente vive no app, não no daemon
- conectar-agente-mcp.md — Conectar um agente (Claude Code) ao MCP de um rustploy
- correcao-persistencia-geometria-janela.md — Correção: a janela não reabria no último tamanho (glacier 0.49.0 → 0.49.1)
- empacotamento-styles-e-cross-compile-windows.md — Empacotamento do rustploy-gui: mover styles/ e cross-compile Windows (cargo-xwin…
- env-backup.md — Backup automático de env vars
- ingress-proxy.md — Como funciona o ingress proxy e o zero-downtime deploy
- internal-networking.md — Comunicação interna entre serviços do projeto
- inventario-paridade-telas.md — Inventário de paridade — GUI iced × webui
- licoes-aprendidas.md — Lições aprendidas — erros, causas e soluções
- limites-dos-testes-luau.md — Limites dos testes Luau do rustploy-gui
- memoria-threads-e-runtime.md — Memória de threads e runtime async — base teórica
- plano-acao-banco-compartilhado.md — Plano de ação: implementar o banco compartilhado (Postgres) e migrar rdo-itemize…
- plano-aquecer-job.md — Aquecer job: saber se o cache existe e deixá-lo pronto antes de rodar
- plano-backend-podman.md — Plano: backend Podman (alternativa ao Docker)
- plano-backup-estado-em-git.md — Plano: YAML como fonte da verdade da infra, com git e religamento em ordem
- plano-banco-compartilhado.md — Plano: banco compartilhado entre projetos + migração dos bancos antigos
- plano-cancelamento-de-jobs.md — Cancelamento de `job_run` em andamento
- plano-chave-de-acesso-e-mcp.md — Plano: chave de acesso única + API de agente no daemon (via MCP)
- plano-colisao-nome-container.md — Plano: colisão de nome de container entre projetos
- plano-convergencia-templates-gui-webui.md — Plano: convergência dos templates (GUI iced ↔ web UI)
- plano-copiar-servico-entre-servidores.md — Copiar um serviço de um servidor para outro
- plano-credenciais-cifradas-e-storage.md — Credenciais cifradas por padrão, renomear chaves e o serviço Storage
- plano-dashboard-servicos-em-execucao.md — Plano: dashboard "o que está rodando agora"
- plano-dependencias-e-autostart.md — Dependências entre serviços + auto-restart no boot do daemon
- plano-erro-de-deploy-invisivel.md — Erro de deploy invisível: a causa da falha é gravada, mas nunca chega no log que…
- plano-evolucao-do-backend-grafico.md — Plano (preliminar): tirar o glacier-ui de dentro do iced, sem reescrever o que é…
- plano-exportar-importar-dados-de-servico.md — Exportar e importar os dados de um serviço
- plano-fila-deploys.md — Plano: fila global de deploys (um por vez), visível e gerenciável
- plano-file-io-luau-e-geometria.md — Plano: I/O de arquivo no Luau + geometria da janela fora do Rust
- plano-geracao-e-rotacao-de-segredos.md — Geração e rotação de segredos (senha de banco, broker, storage, token JWT, par V…
- plano-indice-de-codigo.md — Plano: índice de código para busca barata
- plano-jobs-na-fila-de-deploy.md — Plano: jobs e deploys na mesma fila (uma coisa por vez)
- plano-jobs-visiveis-na-fila.md — Plano: a execução de um job entra na fila de deploy de verdade
- plano-limites-do-agente-mcp.md — Limites do agente (MCP): o que ele pode editar, e como fechar os buracos
- plano-limpeza-automatica-docker.md — Limpeza automática do Docker: liberar espaço em disco sozinho, todos os dias
- plano-log-de-runtime-persistido.md — Plano: persistir o log de runtime dos containers
- plano-multi-login-clients.md — Plano: login multi-servidor no client iced + login simplificado na webui
- plano-nome-gravado-rede-e-stack.md — Plano: nome de rede e de stack Compose gravados, não derivados
- plano-nomes-descritivos.md — Plano: nomes descritivos em toda a base (GUI glacier + webui)
- plano-pre-deploy-gate.md — Pré-deploy gate: rodar um check antes do deploy, e só prosseguir se ele passar
- plano-reforma-gui-glacier-0.102.md — Plano: reforma do `rustploy-gui` sobre o glacier-ui moderno (0.87 → 0.102)
- plano-registry-embutido.md — Registry Docker embutido no rustployd
- plano-reordenar-servicos.md — Reordenar os serviços de um projeto
- plano-reorganizacao-daemon-e-shared.md — Plano (preliminar): reorganizar o daemon e o shared
- plano-site-documentacao.md — Plano: site de documentação do Rustploy (SSG, SEO e GEO)
- plano-tray-bandeja-e-ciclo-de-vida.md — Plano — ícone de bandeja e o app que sobrevive à última janela
- plano-unificacao-webhook-api.md — Plano: unificar o webhook na porta da API
- plano-updaters-daemon-e-gui.md — Plano: updaters (daemon e GUI) — um instalador root minúsculo para cada app
- plano-webui-janelas-e-polimento.md — Plano: webui com acabamento da GUI, janelas arrastáveis e paridade total
- plano-widgets-glacier-0.68.md — Widgets novos do glacier-ui (0.63 → 0.68): o que dá pra aproveitar no rustploy
- prompt-depurar-infra-com-agente.md — Prompt: depurar a infra pela API de agente
- relatorio-porta-externa-automatica.md — Relatório: URL de conexão externa sem burocracia — porta automática + firewall g…
- release.md — Como publicar uma release
- secrets.md — Secrets — Gerenciamento de Credenciais
- services.md
- status-2026-05-26.md — Status do Projeto — 26 de maio de 2026
- tls-acme.md — TLS automático via ACME (Let's Encrypt)
- webhooks-exemplos.md — Webhooks: exemplos de requisição e resposta
- webhooks.md — Webhooks de Deploy
- windows-code-signing.md — Assinatura de código no Windows (rustploy-gui)

### docs/arquivo/
- 2026-07-13-remocao-tui-e-registry-fase2.md — 2026-07-13 — Remoção do TUI + Registry Docker embutido (Fase 2)
- compressao-gzip-api.md — Compressão gzip da API (daemon → GUI)
- conceitos-tls-csr-acme.md — Conceitos: TLS, Certificados, CSR, CN, SANs, ACME e rcgen
- embedded-docker-registry.md — Rustploy — Registry Docker embutido, Fase 3 (integração com o deploy executor)
- example-iced-system-tray.md
- infra-as-code-organizacao-repo.md — Infra-as-Code — Onde versionar os manifestos
- infra-as-code.md — Infra-as-Code — Manifestos YAML declarativos
- luau-modularizacao-pacotes.md — Modularização da camada Luau em pacotes (`fmt/`, `handlers/`, `net/`)
- migracao-http-luau.md — Plano: migrar RWP → HTTP/SSE + lógica de rede em Luau (rustploy.chiquitos.tech)
- migration.md — Migração de Outras Plataformas
- notificacao-so.md — Notificação nativa do SO ao concluir um deploy

### keys/
- rustploy-update-maquina-2.pub
- rustploy-update.pub

### rustploy-daemon/
- .gitignore
- Cargo.lock
- Cargo.toml
- README.md — Rustploy
- docker-compose.test.yml
- install.sh

### rustploy-daemon/crates/daemon/ → daemon-core.md
- Cargo.toml — Rustploy PaaS daemon — lightweight self-hosted deployment platform
- build.rs — Gera, em tempo de compilação, os assets estáticos da web UI/PWA (`crates/daemon/…

### rustploy-daemon/crates/daemon/build/ → daemon-core.md
- templates_catalog.rs — Gera, em tempo de compilação, o catálogo estático de templates a partir dos blue…

### rustploy-daemon/crates/daemon/src/ → daemon-core.md
- main.rs — Ponto de entrada do `rustployd`: delega tudo à montagem do sistema em `bootstrap…

### rustploy-daemon/crates/daemon/src/api/ → daemon-api.md
- dispatch.rs — `dispatch`: o `match` que manda cada `Command` para o seu handler em `handlers/`…
- mod.rs — API do daemon: adaptadores (HTTP, web UI, agente MCP) que traduzem pedidos em ch…

### rustploy-daemon/crates/daemon/src/api/agent/ → daemon-api.md
- access_key.rs — Token de conexão: o endereço do daemon e o token da API numa string só.
- manual.md — Rustploy — manual do agente
- mcp.rs — Servidor MCP em `POST /mcp` — transporte Streamable HTTP, sempre respondendo `ap…
- mod.rs — Acesso de agente servido pelo próprio daemon: a chave de acesso, o `POST /api/lo…
- policy.rs — Níveis da chave de **visitante** (docs/plano-limites-do-agente-mcp.md, itens 0 e…
- redigir.rs — Redação de segredos nas respostas que vão para o **agente** (MCP/`rpc`).
- tools.rs — As ferramentas do agente: os caminhos frequentes já resolvidos ("deployou? por q…

### rustploy-daemon/crates/daemon/src/api/handlers/ → daemon-api.md
- 2 arquivos, um arquivo por `Command` (exceto `mod.rs`); o que cada um faz está em `comandos.md` e `daemon-api.md`: mod, state_git

### rustploy-daemon/crates/daemon/src/api/handlers/databases/ → daemon-api.md
- managed_database.rs — `Command::ManagedDatabase{List,Create,Delete}`: databases (com usuário próprio) …
- migration.rs — `Command::Migration*`: assistente de migração para banco compartilhado (ver `cra…
- mod.rs — Handlers de databases: o `dispatch` recebe os `Command` da seção e chama o handl…
- shared_access.rs — `Command::SharedAccess{List,Grant,Revoke}`: quais projetos alcançam um servidor …

### rustploy-daemon/crates/daemon/src/api/handlers/deploys/ → daemon-api.md
- deploy_abort.rs — `Command::DeployAbort`: aborta um deploy.
- deploy_delete.rs — `Command::DeployDelete`: apaga um deployment terminado e seus logs de build; rec…
- deploy_engine_status.rs — `Command::DeployEngineStatus`: estado da tela Deploy Engine: fila (na ordem da f…
- deploy_history.rs — `Command::DeployHistory`: últimos deployments de um serviço, com o `live` velho …
- deploy_queue_pause.rs — `Command::DeployQueuePause`: pausa/retoma a fila global de deploys.
- deploy_queue_promote.rs — `Command::DeployQueuePromote`: fura a fila (move um enfileirado para o início).
- deploy_queue_reorder.rs — `Command::DeployQueueReorder`: reordena a fila global (arrastar na GUI).
- deploy_rollback.rs — `Command::DeployRollback`: acha o deploy Live anterior de um serviço (só consult…
- get_build_logs.rs — `Command::GetBuildLogs`: linhas do log de build de um deployment.
- mod.rs — Handlers de deploys: o `dispatch` recebe os `Command` da seção e chama o handler…
- recent_deployments.rs — `Command::RecentDeployments`: deployments recentes de todos os serviços, com o L…
- reconcile.rs — Corrige deployments marcados Live que já não estão no ar (`fix_stale_live`), sem…

### rustploy-daemon/crates/daemon/src/api/handlers/docker/ → daemon-api.md
- docker_cleanup.rs — `Command::DockerCleanupConfig*` e `DockerCleanupRunNow`: config e disparo manual…
- docker_inventory.rs — Docker-wide inventory for the Docker tab: every image/volume/network on the host…
- docker_prune.rs — `Command::Prune*`: limpeza manual de containers, volumes, imagens, build cache e…
- docker_remove.rs — Remoção INDIVIDUAL de um recurso Docker (o par por-item dos `docker_prune`): um …
- mod.rs — Handlers de docker: o `dispatch` recebe os `Command` da seção e chama o handler …

### rustploy-daemon/crates/daemon/src/api/handlers/git/ → daemon-api.md
- git_branch_list.rs — `Command::GitBranchList`: branches de um repositório num provedor git conectado.
- git_oauth_start.rs — `Command::GitOAuthStart`: monta a URL de autorização OAuth do provedor git e gua…
- git_provider_create.rs — `Command::GitProviderCreate`: conecta um provedor git (Gitea/GitHub) por OAuth o…
- git_provider_delete.rs — `Command::GitProviderDelete`: desconecta (apaga) um provedor git.
- git_provider_list.rs — `Command::GitProviderList`: provedores git conectados, sem os tokens.
- git_repo_list.rs — `Command::GitRepoList`: repositórios de um provedor git, com um refresh OAuth e …
- mod.rs — Handlers de git: o `dispatch` recebe os `Command` da seção e chama o handler cer…

### rustploy-daemon/crates/daemon/src/api/handlers/iac/ → daemon-api.md
- mod.rs — `Command`s de IaC (manifesto e pacote de serviço): só roteia para `features::iac…

### rustploy-daemon/crates/daemon/src/api/handlers/jobs/ → daemon-api.md
- get_job_logs.rs — `Command::GetJobLogs`: linhas do log de uma execução de job.
- job_create.rs — `Command::JobCreate`: cria um job one-shot (Schedules) de um projeto.
- job_delete.rs — `Command::JobDelete`: apaga um job (e o registro de aquecimento dele) e limpa o …
- job_list.rs — `Command::JobList`: jobs (Schedules) de um projeto.
- job_list_all.rs — `Command::JobListAll`: jobs de todos os projetos, com nomes de projeto/serviço r…
- job_run_cancel.rs — `Command::JobRunCancel`: sinaliza o cancelamento de uma execução de job em andam…
- job_run_history.rs — `Command::JobRunHistory`: últimas execuções de um job.
- job_run_now.rs — `Command::JobRunNow`: dispara uma execução avulsa de um job (`jobs::runner::spaw…
- job_update.rs — `Command::JobUpdate`: edita um job (compose ou fonte git, serviço principal, env…
- job_warm.rs — `Command::JobWarm`: aquece um job (baixa as imagens que faltam e popula o cache …
- job_warm_status.rs — `Command::JobWarmStatus`: se as imagens e o cache de build de um job já estão no…
- mod.rs — Handlers de jobs: o `dispatch` recebe os `Command` da seção e chama o handler ce…

### rustploy-daemon/crates/daemon/src/api/handlers/observability/ → daemon-api.md
- apply_update.rs — `Command::ApplyUpdate`: atualiza o daemon para a última release estável.
- check_update.rs — `Command::CheckUpdate`: há release mais nova que este daemon? Só consulta, nunca…
- daemon_status.rs — `Command::DaemonStatus`: versão, uptime e contagem de serviços rodando/total.
- ingress.rs — Leitura e conserto da tabela de rotas do ingress proxy.
- logs_executions.rs — `Command::LogsExecutions` e `Command::LogsExecutionDelete`: as execuções (um con…
- logs_get.rs — `Command::LogsGet`: últimas linhas de log de um serviço, do container ou (Compos…
- logs_history.rs — `Command::LogsHistory`: o log de runtime **persistido** do serviço (sobrevive à …
- mod.rs — Handlers de observability: o `dispatch` recebe os `Command` da seção e chama o h…
- ping.rs — `Command::Ping`: responde `Pong` com o uptime.
- update_status.rs — `Command::UpdateStatus`: o passo da atualização em curso, ou o resultado da últi…

### rustploy-daemon/crates/daemon/src/api/handlers/projects/ → daemon-api.md
- mod.rs — Handlers de projects: o `dispatch` recebe os `Command` da seção e chama o handle…
- project_create.rs — `Command::ProjectCreate`: cria um projeto.
- project_delete.rs — `Command::ProjectDelete`: apaga um projeto; recusa se ainda tiver serviços ou jo…
- project_env_set.rs — `Command::ProjectEnvSet`: substitui as env vars (e comentários) de nível de proj…
- project_list.rs — `Command::ProjectList`: todos os projetos.
- project_services_reorder.rs — `Command::ProjectServicesReorder`: ordem de exibição dos serviços de um projeto.
- project_update.rs — `Command::ProjectUpdate`: renomeia/redescreve um projeto.

### rustploy-daemon/crates/daemon/src/api/handlers/registry/ → daemon-api.md
- commands.rs — Sub-aba Docker > Registry: navegação (repositórios/tags), delete e GC do registr…
- mod.rs — Handlers de registry: o `dispatch` recebe os `Command` da seção e chama o handle…

### rustploy-daemon/crates/daemon/src/api/handlers/secrets/ → daemon-api.md
- env_key_rename.rs — `Command::EnvKeyRename`: renomeia a chave de uma env (projeto ou serviço) manten…
- env_value_set.rs — `Command::EnvValueSet`: troca só o valor de uma env (projeto ou serviço), na mes…
- infra_audit.rs — `Command::InfraAudit`: o que está em claro no estado de infra (sem os valores).
- infra_download.rs — `Command::InfraDownload`: a infra como os arquivos que o `state_git` grava no re…
- mod.rs — Handlers de secrets: o `dispatch` recebe os `Command` da seção e chama o handler…
- secret_delete.rs — `Command::SecretDelete`: apaga um secret (do projeto, ou de um serviço).
- secret_list.rs — `Command::SecretList`: nomes (nunca valores) dos secrets de um projeto, ou de um…
- secret_move.rs — `Command::SecretMove`: mover entre env e secret, ou entre escopos (R5 do plano).
- secret_rename.rs — `Command::SecretRename`: renomeia um secret e reaponta as envs que o referenciam…
- secret_reveal.rs — `Command::SecretReveal`: o valor de um secret ("ver valor").
- secret_set.rs — `Command::SecretSet`: cria/sobrescreve um secret cifrado (do projeto, ou de um s…

### rustploy-daemon/crates/daemon/src/api/handlers/services/ → daemon-api.md
- get_webhook_url.rs — `Command::GetWebhookUrl`: URL de webhook de deploy de um serviço; `build_url` mo…
- mod.rs — Handlers de services: o `dispatch` recebe os `Command` da seção e chama o handle…
- regenerate_webhook_token.rs — `Command::RegenerateWebhookToken`: gera um token novo de webhook para o serviço …
- service_archive_upload.rs — Upload de `.zip` para serviço com fonte Archive: valida o tamanho e extrai com p…
- service_connection_info.rs — `Command::ServiceConnectionInfo`: host e connection string internos de um serviç…
- service_get.rs — `Command::ServiceGet`: um serviço pelo id.
- service_list.rs — `Command::ServiceList`: serviços de um projeto.
- service_reload.rs — `Command::ServiceReload`: reinicia o container live de um serviço (stop gracioso…
- wizard.rs — Wizard "Novo serviço" server-side: catálogos (`WizardCatalog`) e criação (`Wizar…

### rustploy-daemon/crates/daemon/src/api/handlers/settings/ → daemon-api.md
- env_backup.rs — `Command::EnvBackupList`/`EnvBackupRestore`: lista e restaura snapshots de env v…
- get_daemon_settings.rs — `Command::GetDaemonSettings`: e-mail ACME, domínio do registry e a URL pública d…
- mod.rs — Handlers de settings: o `dispatch` recebe os `Command` da seção e chama o handle…
- set_daemon_settings.rs — `Command::SetDaemonSettings`: grava e-mail ACME e domínio do registry, ligando/d…

### rustploy-daemon/crates/daemon/src/api/http/ → daemon-api.md
- dashboard.rs — O painel "Em execução" (`dashboard` no snapshot): só os projetos que têm serviço…
- mod.rs — HTTP/JSON + SSE control API — the daemon's remote administrative channel.
- public_routes.rs — Rotas HTTP **públicas** (sem Bearer): o webhook de deploy e o callback OAuth do …

### rustploy-daemon/crates/daemon/src/api/web_ui/ → daemon-api.md
- headless_tests.rs — Testes da web UI num Chrome sem cabeça (exigem `google-chrome` instalado): login…
- mod.rs — Servidor de estáticos da web UI/PWA (`crates/daemon/webui/`) — alternativa ao cl…

### rustploy-daemon/crates/daemon/src/bin/ → daemon-core.md
- rustploy-cli.rs — `rustploy-cli`: ferramentas de linha de comando que **não precisam do daemon rod…
- rustployd-fw.rs — `rustployd-fw` — helper privilegiado de firewall do rustploy.

### rustploy-daemon/crates/daemon/src/bin/rustployd-update/ → daemon-core.md
- main.rs — `rustployd-update` — o helper root que instala pacotes **assinados** do rustploy…
- protocol.rs — O protocolo do socket: uma linha JSON por pedido, uma de resposta.

### rustploy-daemon/crates/daemon/src/bootstrap/ → daemon-core.md
- config.rs — Resolução de diretórios (banco, chave age) com fallback para o home e inicializa…
- listeners.rs — Listeners do daemon: registry OCI embutido (storage e rota), proxy de ingress e …
- mod.rs — Montagem do `rustployd`: config e diretórios, banco, Docker, ingress, `AppState`…
- state.rs — Estado compartilhado do daemon (`AppState`) e os caches do inventário Docker.
- workers.rs — Tarefas de fundo do daemon: backup de env, métricas, logs, GC do registry, watch…

### rustploy-daemon/crates/daemon/src/engine/ → daemon-engine.md
- images.rs — Imagens: pull (com credenciais) e build a partir de um contexto empacotado em ta…
- inventory_cache.rs — Cache do inventário do host (imagens, volumes, redes), que é lento de consultar.
- mod.rs — Cliente Docker (`DockerClient`, via bollard) e os submódulos por recurso.
- networks.rs — Rede Docker por projeto: nome e criação sob demanda.

### rustploy-daemon/crates/daemon/src/engine/compose/ → daemon-engine.md
- down.rs — `compose down` de uma stack.
- jobs.rs — Execução de jobs one-shot via compose: arquivos, imagens, aquecimento e cancelam…
- logs.rs — Leitura e limpeza da saída do compose, linha a linha, para o log do job/deploy.
- mod.rs — Serviços e jobs Docker Compose: `up` de stack com a rede do projeto injetada, ex…
- up.rs — `compose up` de uma stack de serviço: login no registry, rede do projeto e volum…

### rustploy-daemon/crates/daemon/src/engine/containers/ → daemon-engine.md
- inspect.rs — Inspeção de um container: IP, logs e metadados.
- lifecycle.rs — Ciclo de vida de um container: criar o staging, start, stop, rename, remover e a…
- mod.rs — Containers de serviços Application: nomes (live, staging, réplicas, legado), bus…
- naming.rs — Nomes dos containers de serviços Application: live, staging, réplicas e o format…
- query.rs — Busca de containers por label, nome e prefixo, e o índice dos gerenciados.

### rustploy-daemon/crates/daemon/src/features/ → daemon-features.md
- mod.rs — Um diretório por assunto de produto; a camada `api` só adapta pedidos para estas…
- wizard.rs — Lógica do wizard "Novo serviço" (Application / Database / Broker / Storage / Com…

### rustploy-daemon/crates/daemon/src/features/backup/ → daemon-features.md
- mod.rs — Backup periódico das env vars de projetos e serviços em snapshots JSON, com list…

### rustploy-daemon/crates/daemon/src/features/databases/ → daemon-features.md
- context.rs — O que a migração de banco precisa do resto do daemon.
- env_switch.rs — Troca de uma env var de conexão (`DATABASE_URL`…) num projeto, com o valor antig…
- managed_view.rs — Visão de um banco gerenciado: monta a connection URL como o projeto consumidor a…
- migration.rs — Assistente de migração: banco antigo (serviço Compose do projeto) → database ger…
- mod.rs — Bancos de dados gerenciados: assistente de migração, acesso compartilhado e troc…
- shared_db.rs — Provisionamento de database + usuário dentro de um servidor de banco compartilha…

### rustploy-daemon/crates/daemon/src/features/deploy/ → daemon-deploy.md
- autostart.rs — Orquestrador de **subida em ordem**: religa serviços respeitando as dependências…
- context.rs — O que o motor de deploy (fila, executor, start, stop) precisa do resto do daemon…
- deps.rs — Grafo de dependências entre serviços: ordenação em **ondas**, detecção de ciclo …
- env_resolve.rs — Resolução de env vars com secrets decifradas — extraído de `DeployExecutor::reso…
- git.rs — Clone de repositório git para build, com progresso, credenciais de provedor cone…
- mod.rs — Motor de deploy: fila global, executor, recuperação no boot, clone git e resoluç…
- queue.rs — Fila **global** de deploys: no máximo um deploy rodando por vez no daemon.
- recovery.rs — Recuperação no boot: aborta deploys interrompidos, reconcilia status com o Docke…
- shared_net.rs — Servidor de banco compartilhado: conecta o container dele às redes dos projetos …
- start.rs — Iniciar um deploy (`Command::DeployStart`): valida a fonte do serviço, cria o de…
- stop.rs — Parar um serviço (`Command::ServiceStop`): para um serviço (todas as réplicas li…
- tree.rs — Deploy com a árvore de dependências (`Command::DeployWithDependencies` e `Comman…

### rustploy-daemon/crates/daemon/src/features/deploy/executor/ → daemon-deploy.md
- compose_flow.rs — Caminho Compose: `compose up` da stack e espera do healthcheck.
- drain.rs — Estágio `Draining`: espera as conexões do container antigo terminarem.
- healthcheck.rs — Estágio de healthcheck: espera o staging (ou a stack Compose) ficar saudável.
- mod.rs — `DeployExecutor`: roda um deployment pela máquina de estados (clone/pull/build, …
- promote.rs — Estágios de promoção: troca do tráfego para o container novo e promoção a live.
- resolve.rs — Estágios iniciais: rede do projeto, pré-deploy checks e escolha do caminho pela …
- rollback.rs — Estágio `RollingBack`: remove o staging e restaura o estado anterior, guardando …
- source.rs — Estágios de obtenção da imagem: pull do registry, clone do Git e build.
- staging.rs — Estágio `Staging`: sobe o container novo ao lado do live.
- state_machine.rs — Máquina de estados do deploy: o laço `execute` e o despacho de cada `DeployState…
- tests.rs

### rustploy-daemon/crates/daemon/src/features/git_providers/ → daemon-registry.md
- gitea.rs — Minimal Gitea API client: OAuth2 token exchange/refresh plus the few REST endpoi…
- github.rs — Minimal GitHub API client: OAuth2 token exchange/refresh plus the few REST endpo…
- mod.rs — Clients for hosted Git providers.

### rustploy-daemon/crates/daemon/src/features/iac/ → daemon-features.md
- context.rs — O que o manifesto e o pacote de serviço (export/import) precisam do resto do dae…
- manifest.rs — Infra-as-Code: structs do manifesto declarativo (`rustploy.yml`).
- mod.rs — Infraestrutura como código: manifesto de projeto/servidor e pacote de serviço (e…
- service_bundle.rs — Pacote de serviço: UM serviço (mais as variáveis do projeto que o usuário escolh…
- service_export.rs — `Command::ServiceExportPlan` e `Command::ServiceExport`: o lado de cá de copiar …
- service_import.rs — `Command::ServiceImport`: cria UM serviço novo a partir de um [`crate::features:…

### rustploy-daemon/crates/daemon/src/features/ingress/ → daemon-engine.md
- external_ports.rs — Alocação automática de portas externas (`ServiceSpec.host_port`).
- firewall.rs — Cliente do helper privilegiado de firewall (`rustployd-fw`).
- mod.rs — Ingress: proxy reverso HTTP/HTTPS embutido, tabela de rotas e TLS/ACME.
- proxy.rs — Proxy reverso HTTP/1.1 embutido, construído sobre hyper.
- router.rs — Tabela de rotas do ingress (domínio → backends, porta → backends) com round-robi…
- tls.rs — Certificados TLS: resolução por SNI, emissão e renovação ACME (Let's Encrypt) e …

### rustploy-daemon/crates/daemon/src/features/jobs/ → daemon-deploy.md
- context.rs — O que os jobs (runner, agendador, aquecimento) precisam do resto do daemon.
- mod.rs — Jobs one-shot (Schedules): execução (`runner`) e agendamento (`scheduler`).
- runner.rs — Execução de um `Job` (tarefa one-shot via docker-compose): resolve rede + env va…
- scheduler.rs — Ticker de agendamento dos jobs one-shot — mesmo formato de `metrics.rs`/`env_bac…
- source.rs — Checkout do repositório de um job git-sourced, compartilhado pela execução (`run…
- warm.rs — Aquecimento de jobs: deixar no host as imagens e o cache de build que um job usa…

### rustploy-daemon/crates/daemon/src/features/maintenance/ → daemon-deploy.md
- context.rs — O que a limpeza do Docker precisa do resto do daemon.
- mod.rs — Limpeza automática (agendada) de recursos Docker não usados — ver `docs/plano-li…
- prune.rs — Limpeza de containers, volumes, imagens, build cache e redes do Docker: o miolo …
- run.rs — Execução de uma limpeza (agendada ou "Executar agora"): roda os recursos marcado…
- scheduler.rs — Ticker que verifica se a limpeza automática de Docker está devida — mesmo idioma…
- service_warm.rs — Aquecimento de serviços: deixar no host o cache de build da imagem de um serviço…

### rustploy-daemon/crates/daemon/src/features/observability/ → daemon-features.md
- container_logs.rs — Streaming dos logs dos containers gerenciados para o event bus, reconectando e e…
- context.rs — O que o watchdog precisa do resto do daemon.
- metrics.rs — Coleta periódica de métricas de CPU/memória/disco do host e por container, publi…
- mod.rs — Observabilidade: watchdog dos serviços, métricas do host e transmissão de logs d…
- watchdog.rs — Watchdog dos serviços no ar: checa se o container roda e passa no healthcheck, r…

### rustploy-daemon/crates/daemon/src/features/registry/ → daemon-registry.md
- auth.rs — Basic auth do registry OCI embutido — checada em TODA rota (inclusive `GET /v2/`…
- error.rs — Envelope de erro da OCI Distribution Spec: `{"errors":[{"code","message","detail…
- gc.rs — Garbage collection do registry: libera do disco o que nenhuma tag alcança.
- http.rs — Rotas HTTP da OCI Distribution API v2 — dispatch manual (hyper cru não tem route…
- internal_token.rs — Token interno usado pelo próprio deploy executor pra puxar imagens do registry e…
- mod.rs — Registry Docker OCI Distribution API v2 embutido — push/pull, GC e Basic auth po…
- name.rs — Validação de `<name>`, `<reference>` (tag) e `<digest>` da OCI Distribution Spec…
- push_deploy.rs — Push-to-deploy: cada tag publicada no registry embutido redeploya os serviços qu…
- retention.rs — Retenção de tags do registry embutido: antes de cada GC, apaga as tags mais anti…
- storage.rs — CAS (content-addressable store) do registry: blobs em disco, sessões de upload e…

### rustploy-daemon/crates/daemon/src/features/service_data/ → daemon-features.md
- database.rs — Dump lógico de banco (Postgres, MySQL/MariaDB, MongoDB), feito **dentro do conta…
- mod.rs — Exportar e importar os **dados** de um serviço (volumes).
- pack.rs — O pacote `.rpdata`: um tar aberto com `manifest.json` **primeiro**, depois `part…
- parts.rs — Quais volumes pertencem a um serviço.
- stream.rs — Roda um comando do motor ligando o stdin/stdout a **arquivos por cópia explícita…
- transfer.rs — Mover os bytes: tirar o tar de cada volume (exportar) e devolvê-los (importar), …

### rustploy-daemon/crates/daemon/src/features/services/ → daemon-deploy.md
- context.rs — O que criar, atualizar e remover um serviço precisam do resto do daemon.
- create.rs — `Command::ServiceCreate`: cria um serviço, alocando a porta externa automática e…
- delete.rs — `Command::ServiceDelete`: para e REMOVE os containers do serviço (ou a stack Com…
- mod.rs — Ciclo de vida de um serviço: criar, atualizar (com a sincronização do firewall) …
- update.rs — `Command::ServiceUpdate`: grava um ServiceSpec novo (porta externa, TLS dos domí…

### rustploy-daemon/crates/daemon/src/features/state_git/ → daemon-features.md
- git.rs — Invólucro fino do binário `git` do sistema (decisão da R9.0: nada de `git2`/`req…
- mod.rs — Sincronização do estado de infra com um repositório git (Fase 6 do plano `docs/p…

### rustploy-daemon/crates/daemon/src/features/templates/ → daemon-features.md
- mod.rs — Catálogo de templates de aplicações (formato Dokploy), lido dos blueprints em `t…

### rustploy-daemon/crates/daemon/src/features/update/ → daemon-features.md
- apply.rs — `ApplyUpdate`: baixa o pacote assinado e o entrega ao helper root `rustployd-upd…
- checker.rs — `UpdateChecker`: compara a versão do daemon com a última release, com cache.
- github_release.rs — A última release estável de um repositório do GitHub (`GET /repos/{repo}/release…
- mod.rs — Verificação de versão: pergunta ao GitHub qual é a última release e compara com …

### rustploy-daemon/crates/daemon/src/platform/ → daemon-core.md
- age_format.rs — O formato `ENC[age:<base64>]` dos segredos, **sem nenhuma dependência do daemon*…
- config.rs — Configuração do daemon (`config.toml`): structs de cada seção com defaults e o s…
- connection.rs — Connection string de banco/broker, montada num lugar só.
- env_validation.rs — Validação das env vars gravadas por serviço, projeto e job: nome válido e sem ch…
- errors.rs — Mensagens de erro amigáveis para o usuário.
- event_bus.rs — Bus de eventos em memória (broadcast): o que alimenta o SSE `/api/events` dos cl…
- health.rs — Checagens de saúde HTTP e TCP usadas pelo healthcheck do deploy e pelo watchdog.
- https_client.rs — Cliente HTTPS de saída mínimo: um GET, hyper cru sobre `tokio-rustls`.
- log_sink.rs — Destino das linhas de log de build/job: o motor de containers escreve aqui sem s…
- log_store.rs — Armazenamento do **log de runtime** dos containers (plano: `docs/plano-log-de-ru…
- mod.rs — Infraestrutura transversal (barramento de eventos, segredos, health checks), sem…
- secrets.rs — `SecretsManager`: cifra e decifra os segredos do daemon.
- ttl_cache.rs — Cache de um valor só, com TTL, que junta chamadas concorrentes numa só ida à fon…

### rustploy-daemon/crates/daemon/src/store/ → daemon-store.md
- build_logs.rs — Tabela `build_log`: linhas do log de build de cada deployment.
- daemon_settings.rs — Tabela chave-valor de configurações do daemon editáveis pela UI (ACME, registry,…
- deployments.rs — Tabela `deployment`: criar, transicionar de estado, histórico por serviço e esta…
- git_providers.rs — Persistence for connected Git providers (Gitea OAuth2 / PAT).
- job.rs — Tabela `job`: jobs one-shot (Schedules), incluindo quais estão vencidos para o a…
- job_log.rs — Tabela `job_log`: linhas de log (stdout/stderr) de cada execução de job.
- job_run.rs — Tabela `job_run`: cada execução de um job e seu exit code.
- log_sink.rs — Grava as linhas de log de build/job nas tabelas `build_log` e `job_log`.
- managed_database.rs — Tabela `managed_database`: databases criados em servidores compartilhados.
- migration.rs — Tabela `migration`: estado do assistente de migração (passos, log, o que foi par…
- mod.rs — Conexão SQLite (`Db`) e as migrações do schema, feitas à mão com `add_column_if_…
- projects.rs — Tabela `project`: CRUD de projetos e suas env vars de nível de projeto.
- registry.rs — Wrappers SQL do registry OCI embutido (metadados; os bytes de blob/manifest vive…
- registry_tokens.rs — Tokens de acesso do registry OCI embutido (Basic auth — ver `crate::features::re…
- services.rs — Serviços: o `ServiceSpec` e o nome da stack Compose moram no **YAML de infra** (…
- shared_access.rs — Tabela `shared_access`: quais projetos podem alcançar cada servidor de banco com…
- state_git_marks.rs — Marcas de runtime da sincronização com o git (Fase 7): o que o `pull` deixou **p…
- webhook_tokens.rs — Tabela de tokens de webhook de deploy, um por serviço.

### rustploy-daemon/crates/daemon/src/store/infra/ → daemon-store.md
- audit.rs — **Auditoria de valores em claro** no estado de infra (plano `docs/plano-backup-e…
- migrate.rs — A migração **única** SQL → YAML (Fase 4 do plano, seção R4).
- mod.rs — Camada de repositório da **infraestrutura** declarada (projetos, serviços, jobs……
- model.rs — O **estado de infra tipado** (Fase 4 do plano `docs/plano-backup-estado-em-git.m…
- repo.rs — O **repositório** do estado de infra: o `InfraState` em memória (as leituras vêm…
- seal.rs — Cifra no lugar o que a migração SQL → YAML deixou em claro.
- txn.rs — Transação de "tudo ou nada" sobre um conjunto de arquivos num diretório.
- yaml.rs — Serialização YAML **determinística** do estado de infra.

### rustploy-daemon/crates/daemon/templates/
- logos.txt
- blueprints/ — catálogo de templates de app (formato Dokploy), compilado pelo build.rs do daemon (775 arquivos, não indexados)

### rustploy-daemon/crates/daemon/tests/e2e-podman-ufw/
- lab.sh
- run.sh

### rustploy-daemon/crates/daemon/tests/e2e-updater/
- Containerfile
- run.sh

### rustploy-daemon/crates/daemon/webui/ → webui.md
- app.css — Rustploy — web UI stylesheet.
- app.js — único <script type="module"> carregado por index.html.
- busy.js — feedback imediato e trava de clique para toda ação assíncrona.
- directives.js — diretivas Alpine próprias da webui.
- format.js
- i18n.js — idioma da webui: inglês por padrão, português (pt-BR) a um clique, escolha lembr…
- icons.js — conjunto de ícones SVG inline (traço 1.75, estilo Lucide) e a diretiva `x-icon="…
- index.html — Casca única da webui (Alpine.js): login, shell e todas as telas, uma seção por v…
- manifest.webmanifest
- sw.js — service worker do PWA Rustploy.
- wm.js
- zip.js — monta um `.zip` no navegador, sem biblioteca.

### rustploy-daemon/crates/daemon/webui/locales/ → webui.md
- en.js — catálogo em inglês, por texto de origem (ver i18n.js).
- pt-BR.js — catálogo em português (pt-BR), por texto de origem (ver i18n.js).

### rustploy-daemon/crates/daemon/webui/net/ → webui.md
- api.js
- sse.js

### rustploy-daemon/crates/daemon/webui/screens/ → webui.md
- dashboard.js — tela "Deployments" (view padrão do shell).
- deploy_engine.js — tela "Deploy Engine": fila global (um deploy por vez), execução em andamento e h…
- docker.js — tela "Docker": containers/imagens/volumes/networks do host inteiro (não só recur…
- ingress.js — tela "Ingress": rotas ativas no reverse proxy (por domínio) e portas TCP de host…
- login.js — tela de login.
- monitoring.js — tela "Monitoring": uso de CPU/memória do host e por container.
- new_service.js
- project_detail.js — projeto aberto (view=project_services no client iced): sub-abas Serviços/Variáve…
- projects.js — tela "Projects": grid de cards + criar/editar/remover.
- running.js — tela "Em execução": só os projetos com serviço rodando e, dentro de cada um, só …
- schedules.js — tela "Schedules": jobs one-shot (docker-compose) agendados ou manuais, de todos …
- service_bundle.js
- service_detail.js — detalhe de um serviço.
- settings.js — tela "Settings": Web Server / Git / Sync / Manutenção / Updates.

### rustploy-daemon/crates/importer/
- Cargo.toml — Migration importer tool for the Rustploy PaaS platform

### rustploy-daemon/crates/importer/src/ → importer.md
- main.rs — CLI do importer: migra projetos e serviços de outra plataforma (hoje, Dokploy) p…
- warnings.rs — `Report` do importer: problemas encontrados na migração, por severidade (bloquea…

### rustploy-daemon/crates/importer/src/sink/ → importer.md
- mod.rs — Grava os dados transformados no banco do rustploy (ou num arquivo SQL), casando …

### rustploy-daemon/crates/importer/src/source/ → importer.md
- dokploy.rs — Leitura dos projetos, aplicações, stacks Compose e domínios direto do Postgres d…
- mod.rs — Fontes de dados do importer, uma por plataforma de origem.

### rustploy-daemon/crates/importer/src/transform/ → importer.md
- dokploy.rs — Converte os dados do Dokploy em projetos e serviços do rustploy, anotando o que …
- mod.rs — Conversão dos dados de origem para os modelos do rustploy (`TransformedData`).

### rustploy-daemon/packaging/
- config.toml
- rustploy-podman-ufw
- rustployd-fw.service
- rustployd-fw.socket
- rustployd-update.service
- rustployd-update.socket
- rustployd.service

### rustploy-daemon/packaging/debian/
- postinst
- prerm

### rustploy-daemon/scripts/
- migrate_id_prefixes.sh

### rustploy-docs/
- .gitignore
- .nvmrc
- README.md — rustploy-docs
- astro.config.mjs
- package-lock.json
- package.json
- wrangler.jsonc

### rustploy-docs/public/
- robots.txt
- (+3 imagens/fontes)

### rustploy-docs/scripts/
- check-links.mjs
- gen-faq.mjs
- gen-manifest.mjs
- gen-protocol.mjs
- gen-templates.mjs
- sync-manual.mjs

### rustploy-docs/scripts/data/
- faq.mjs
- protocol-en.mjs
- template-descriptions-en.tsv

### rustploy-docs/scripts/lib/
- i18n.mjs

### rustploy-docs/src/
- content.config.ts

### rustploy-docs/src/components/
- Shot.astro

### rustploy-docs/src/content/docs/
- faq.mdx
- index.mdx

### rustploy-docs/src/content/docs/agentes/
- conectar-claude-code.mdx
- ferramentas.mdx
- manual.md
- receitas.mdx
- seguranca.mdx
- visao-geral.mdx

### rustploy-docs/src/content/docs/comece/
- docker-para-rustploy.mdx
- glossario.mdx
- o-que-e.mdx

### rustploy-docs/src/content/docs/comparativos/
- rustploy-vs-caprover.mdx
- rustploy-vs-coolify.mdx
- rustploy-vs-dokploy.mdx
- visao-geral.mdx

### rustploy-docs/src/content/docs/conceitos/
- ciclo-de-vida.mdx
- projeto.mdx
- rede-e-nomes.mdx
- servico-e-fontes.mdx
- variaveis-de-ambiente.mdx

### rustploy-docs/src/content/docs/en/
- faq.mdx
- index.mdx

### rustploy-docs/src/content/docs/en/agentes/
- conectar-claude-code.mdx
- ferramentas.mdx
- receitas.mdx
- seguranca.mdx
- visao-geral.mdx

### rustploy-docs/src/content/docs/en/comece/
- docker-para-rustploy.mdx
- glossario.mdx
- o-que-e.mdx

### rustploy-docs/src/content/docs/en/comparativos/
- rustploy-vs-caprover.mdx
- rustploy-vs-coolify.mdx
- rustploy-vs-dokploy.mdx
- visao-geral.mdx

### rustploy-docs/src/content/docs/en/conceitos/
- ciclo-de-vida.mdx
- projeto.mdx
- rede-e-nomes.mdx
- servico-e-fontes.mdx
- variaveis-de-ambiente.mdx

### rustploy-docs/src/content/docs/en/guias/
- backup.mdx
- banco-compartilhado.mdx
- bancos-e-brokers.mdx
- chaves-e-segredos.mdx
- compose.mdx
- copiar-servico.mdx
- dados-do-servico.mdx
- dependencias-e-religar.mdx
- diagnosticar-502.mdx
- dominios-e-tls.mdx
- fila-de-deploys.mdx
- git.mdx
- healthcheck.mdx
- infra-as-code.mdx
- jobs.mdx
- limpeza-docker.mdx
- migrar-banco.mdx
- pre-deploy.mdx
- registry.mdx
- sincronizacao-git.mdx
- templates.mdx
- variaveis-e-secrets.mdx
- webhooks.mdx
- zip.mdx

### rustploy-docs/src/content/docs/en/instalacao/
- atualizar.mdx
- config-toml.mdx
- instalacao-rapida.mdx
- instalar-clientes.mdx
- instalar-daemon.mdx
- primeiro-acesso.mdx
- requisitos.mdx

### rustploy-docs/src/content/docs/en/interface/
- configuracoes.mdx
- deploy-engine.mdx
- docker-e-registry.mdx
- em-execucao.mdx
- entrar.mdx
- idioma.mdx
- ingress.mdx
- logs.mdx
- monitoring.mdx
- novo-servico.mdx
- projetos.mdx
- schedules.mdx
- servico-configuracao.mdx
- servico.mdx
- visao-geral.mdx
- web-ui.mdx

### rustploy-docs/src/content/docs/en/migracao/
- dokploy.mdx

### rustploy-docs/src/content/docs/en/operacao/
- monitoramento-e-saude.mdx
- seguranca-do-servidor.mdx

### rustploy-docs/src/content/docs/en/referencia/
- api-http.mdx
- comandos.md — no fields
- eventos.md
- manifesto.mdx
- paridade.md
- respostas.md

### rustploy-docs/src/content/docs/en/templates/
- ackee.md
- activepieces.md
- actualbudget.md
- adguardhome.md
- adminer.md
- adventurelog.md
- affinepro.md
- agentdvr.md
- akaunting.md
- alist.md
- alltube.md
- ampache.md
- anonupload.md
- anse.md
- answer.md
- anubis.md — Unconmment in case you want to connect the anubis container to dokploy-network
- anythingllm.md
- anytype.md — Example: Any-Sync-Bundle with embedded MongoDB and Redis (all-in-one image)
- appflowy.md
- apprise-api.md
- appsmith.md
- appwrite.md
- aptabase.md
- arangodb.md
- argilla.md
- audiobookshelf.md
- authelia.md
- authentik.md
- authorizer.md
- autobase.md
- automatisch.md
- azuracast.md
- babybuddy.md
- backrest.md
- baikal.md
- barrage.md
- baserow.md
- bazarr.md
- bentopdf.md
- beszel.md
- bigcapital.md
- blender.md
- blinko.md
- bluesky-pds.md
- bolt-diy.md
- booklore.md
- bookstack.md
- borgitory.md
- botpress.md
- browserless.md
- budget-board.md
- budibase.md
- bugsink.md
- bytebase.md
- bytestash.md
- calcom.md
- calibre-web.md
- calibre.md
- capso.md
- carbone.md
- casdoor.md
- changedetection.md
- chatwoot.md
- checkcle.md
- checkmate.md
- chevereto.md
- chibisafe.md — https://chibisafe.app/docs/intro
- chiefonboarding.md
- chirpstack.md
- chromium.md
- classicpress.md
- clickhouse.md
- cloud9.md
- cloudcommander.md
- cloudflare-ddns.md — For more details, see:
- cloudflared.md
- cloudreve.md
- cockpit.md
- coder.md
- codex-docs.md
- colanode.md
- collabora-office.md
- commafeed.md
- commento.md
- commentoplusplus.md
- conduit.md — From Conduit's official documentation: https://docs.conduit.rs/deploying/docker.…
- conduwuit.md — conduwuit
- confluence.md
- convertx.md
- convex.md
- cookie-cloud.md
- coralproject.md
- couchdb.md
- crawl4ai.md
- crowdsec.md — --------------------------------------------------------------------------------
- cup.md
- cyberchef.md
- dashy.md
- datalens.md
- directory-lister.md
- directus.md
- discord-tickets.md
- discourse.md
- docling-serve.md
- docmost.md
- documenso.md
- docuseal.md
- dokploy-prom-monitoring-extension.md
- dolibarr.md
- domain-locker.md
- doublezero.md
- dozzle.md
- dragonfly-db.md
- drawio.md
- drawnix.md
- drizzle-gateway.md
- dumbassets.md
- dumbbudget.md
- dumbdrop.md
- dumbpad.md
- easyappointments.md
- elastic-search.md
- emby.md
- emqx.md — This templates requires additional traefik port mapping and entry point
- enshrouded.md
- erpnext.md
- etherpad.md
- evershop.md
- evolutionapi.md
- excalidraw.md
- ezbookkeeping.md
- filebrowser.md
- filestash.md
- firecrawl.md
- fivem.md — docker-compose.yml
- flagsmith.md — See https://docs.flagsmith.com/deployment/docker for more information on running…
- flaresolverr.md
- flatnotes-totp.md
- flatnotes.md
- flowise.md
- fmd-server.md
- focalboard.md
- fonoster.md
- forgejo.md
- formbricks.md
- frappe-hr.md
- freescout.md
- freshrss.md
- garage-with-ui.md
- garage.md
- ghost.md
- gitea-mirror.md
- gitea-mysql.md
- gitea-postgres.md
- gitea-sqlite.md
- gitingest.md
- gitlab-ce.md
- glance.md
- glitchtip.md
- glpi.md
- go-whatsapp-web-multidevice.md
- gotenberg.md
- grafana.md
- grimoire.md
- grist.md
- habitica.md
- heyform.md
- hi-events.md
- hoarder.md
- homarr.md
- homeassistant.md
- homebridge.md
- hoppscotch.md
- hortusfox.md
- huly.md
- i18n-blog.md
- ihatemoney.md
- imgproxy.md
- immich.md
- index.md
- infisical.md
- influxdb.md
- inngest.md
- instantdb.md
- invoiceshelf.md
- ipfs.md
- it-tools.md
- java.md
- jellyfin.md
- jenkins.md
- kaneo.md
- karakeep.md
- kener.md
- kestra.md
- keycloak.md
- kimai.md
- kitchenowl.md
- kokoro-tts.md
- kokoro-web.md
- komari-monitor.md
- kutt.md
- langflow.md
- lavalink.md
- letterfeed.md
- librechat.md — LibreChat Docker Compose for Dokploy Template
- libredesk.md
- libretranslate.md
- linkding.md
- linkstack.md
- linkwarden.md
- listmonk.md
- litellm.md
- livekit.md
- lobe-chat.md
- lodestone.md
- logto.md
- lowcoder.md
- macos.md
- mage-ai.md — https://docs.mage.ai/getting-started/setup#docker-compose
- mailpit.md
- mattermost.md
- mautic.md
- maybe.md
- mazanoke.md
- mcsmanager.md
- mealie.md
- mediacms.md
- mediafetch.md
- meilisearch.md
- memos.md
- metabase.md
- metube.md
- minepanel.md
- minio.md — comment the line below if you specified a local directory in the volumes section…
- misaka-danmu-server.md
- mixpost.md
- moltbot.md
- morphos.md
- movary.md
- mulesoft-esb.md
- mumble.md
- n8n-runner-postgres-ollama.md
- n8n-with-postgres.md
- n8n.md
- navidrome.md
- neko.md
- netdata.md
- networking-toolbox.md
- nextcloud-aio.md
- nginx.md
- nocodb.md
- notifuse.md
- ntfy.md
- obsidian-livesync.md
- odoo_17.md
- odoo_18.md
- odoo_19.md
- ojs.md
- omni-tools.md
- onedev.md
- onetimesecret.md
- ontime.md
- open-fiesta.md
- open-webui.md — This is the main service for the Open WebUI interface.
- open_notebook.md
- openclaw.md
- opengist.md — docker-compose.yml
- openhands.md — The 'version' attribute is obsolete and has been removed.
- openinary.md
- openpanel.md
- openresty-manager.md
- openspeedtest.md
- otterwiki.md
- outline.md
- owncast.md
- palmr.md
- parseable.md
- passbolt.md — =============================================================================
- pastefy.md
- paymenter.md
- peerdb.md
- penpot.md — demo-users
- peppermint.md
- pgadmin.md
- photoprism.md
- phpmyadmin.md
- picsur.md
- pinchflat.md
- plane.md
- plark.md
- plausible.md
- plunk.md — IMPORTANT: Plunk requires HTTPS to work properly
- pocket-id.md
- pocketbase.md — IMPORTANT: Please update the admin credentials in your .env file
- poke.md
- portainer.md
- poste-io.md
- postgres-pgdog.md
- postgresus.md
- postiz.md
- prometheus.md
- pterodactyl.md
- pulse.md
- pyrodactyl.md
- qbittorrent.md — docker-compose.yml
- qbitwebui.md
- qdrant.md
- quant-ux.md
- rabbitmq.md
- reactive-resume.md
- registry.md
- rocketchat.md
- rote.md
- roundcube.md
- rss-bridge.md
- rsshub.md
- rustdesk.md
- rustfs.md
- rutorrent.md
- rybbit.md — https://www.rybbit.io/docs/self-hosting-advanced
- ryot.md
- scrutiny.md — PROXMOX USERS: Proxmox LXCs don't have access to S.M.A.R.T data, while this may …
- scrypted.md
- seafile.md — https://manual.seafile.com/12.0/setup/setup_ce_by_docker
- searxng.md
- seaweedfs.md
- shlink.md
- signoz.md
- silverbullet.md
- slash.md
- snapp.md
- soketi.md
- spacedrive.md
- stack-auth.md
- stalwart.md
- statping-ng.md
- stirling.md
- storyden.md
- strapi.md — Self-host guide:
- streamflow.md
- supabase.md — Self-hosted Supabase, tracking the official compose from
- superset.md — This is an UNOFFICIAL production docker image build for Superset:
- surrealdb.md
- syncthing.md
- tailscale-exitnode.md
- teable.md
- tianji.md
- tolgee.md
- tooljet.md
- tor-browser.md
- trailbase.md — IMPORTANT: The initial admin credentials will be printed in the logs after the c…
- triggerdotdev.md
- trilium-next.md — Running `docker-compose up` will create/use the "trilium-data" directory in the …
- trilium.md
- trmnl-byos-laravel.md
- tuwunel.md
- twenty.md
- typebot.md
- typecho.md
- typesense.md
- umami.md
- unifi.md
- unleash.md — The default users credentials are:
- upsnap.md
- uptime-kuma.md
- usesend.md
- valkey.md
- vault.md
- vaultwarden.md — the webserver can take a while to start up, so don't be alarmed if it takes a fe…
- verdaccio.md
- vikunja.md
- wallos.md
- wanderer.md
- web-check.md
- wg-easy.md
- wikijs.md
- windmill.md
- windows.md
- wordpress.md
- wuzapi.md
- xsshunter.md
- yamtrack.md
- yourls.md
- yt-dlp-webui.md
- zabbix.md
- zipline.md
- zitadel.md

### rustploy-docs/src/content/docs/en/tutorial/
- primeiro-deploy.mdx

### rustploy-docs/src/content/docs/guias/
- backup.mdx
- banco-compartilhado.mdx
- bancos-e-brokers.mdx
- chaves-e-segredos.mdx
- compose.mdx
- copiar-servico.mdx
- dados-do-servico.mdx
- dependencias-e-religar.mdx
- diagnosticar-502.mdx
- dominios-e-tls.mdx
- fila-de-deploys.mdx
- git.mdx
- healthcheck.mdx
- infra-as-code.mdx
- jobs.mdx
- limpeza-docker.mdx
- migrar-banco.mdx
- pre-deploy.mdx
- registry.mdx
- sincronizacao-git.mdx
- templates.mdx
- variaveis-e-secrets.mdx
- webhooks.mdx
- zip.mdx

### rustploy-docs/src/content/docs/instalacao/
- atualizar.mdx
- config-toml.mdx
- instalacao-rapida.mdx
- instalar-clientes.mdx
- instalar-daemon.mdx
- primeiro-acesso.mdx
- requisitos.mdx

### rustploy-docs/src/content/docs/interface/
- configuracoes.mdx
- deploy-engine.mdx
- docker-e-registry.mdx
- em-execucao.mdx
- entrar.mdx
- idioma.mdx
- ingress.mdx
- logs.mdx
- monitoring.mdx
- novo-servico.mdx
- projetos.mdx
- schedules.mdx
- servico-configuracao.mdx
- servico.mdx
- visao-geral.mdx
- web-ui.mdx

### rustploy-docs/src/content/docs/migracao/
- dokploy.mdx

### rustploy-docs/src/content/docs/operacao/
- monitoramento-e-saude.mdx
- seguranca-do-servidor.mdx

### rustploy-docs/src/content/docs/referencia/
- api-http.mdx
- comandos.md — sem campos
- eventos.md
- manifesto.mdx
- paridade.md
- respostas.md

### rustploy-docs/src/content/docs/templates/
- ackee.md
- activepieces.md
- actualbudget.md
- adguardhome.md
- adminer.md
- adventurelog.md
- affinepro.md
- agentdvr.md
- akaunting.md
- alist.md
- alltube.md
- ampache.md
- anonupload.md
- anse.md
- answer.md
- anubis.md — Unconmment in case you want to connect the anubis container to dokploy-network
- anythingllm.md
- anytype.md — Example: Any-Sync-Bundle with embedded MongoDB and Redis (all-in-one image)
- appflowy.md
- apprise-api.md
- appsmith.md
- appwrite.md
- aptabase.md
- arangodb.md
- argilla.md
- audiobookshelf.md
- authelia.md
- authentik.md
- authorizer.md
- autobase.md
- automatisch.md
- azuracast.md
- babybuddy.md
- backrest.md
- baikal.md
- barrage.md
- baserow.md
- bazarr.md
- bentopdf.md
- beszel.md
- bigcapital.md
- blender.md
- blinko.md
- bluesky-pds.md
- bolt-diy.md
- booklore.md
- bookstack.md
- borgitory.md
- botpress.md
- browserless.md
- budget-board.md
- budibase.md
- bugsink.md
- bytebase.md
- bytestash.md
- calcom.md
- calibre-web.md
- calibre.md
- capso.md
- carbone.md
- casdoor.md
- changedetection.md
- chatwoot.md
- checkcle.md
- checkmate.md
- chevereto.md
- chibisafe.md — https://chibisafe.app/docs/intro
- chiefonboarding.md
- chirpstack.md
- chromium.md
- classicpress.md
- clickhouse.md
- cloud9.md
- cloudcommander.md
- cloudflare-ddns.md — For more details, see:
- cloudflared.md
- cloudreve.md
- cockpit.md
- coder.md
- codex-docs.md
- colanode.md
- collabora-office.md
- commafeed.md
- commento.md
- commentoplusplus.md
- conduit.md — From Conduit's official documentation: https://docs.conduit.rs/deploying/docker.…
- conduwuit.md — conduwuit
- confluence.md
- convertx.md
- convex.md
- cookie-cloud.md
- coralproject.md
- couchdb.md
- crawl4ai.md
- crowdsec.md — --------------------------------------------------------------------------------
- cup.md
- cyberchef.md
- dashy.md
- datalens.md
- directory-lister.md
- directus.md
- discord-tickets.md
- discourse.md
- docling-serve.md
- docmost.md
- documenso.md
- docuseal.md
- dokploy-prom-monitoring-extension.md
- dolibarr.md
- domain-locker.md
- doublezero.md
- dozzle.md
- dragonfly-db.md
- drawio.md
- drawnix.md
- drizzle-gateway.md
- dumbassets.md
- dumbbudget.md
- dumbdrop.md
- dumbpad.md
- easyappointments.md
- elastic-search.md
- emby.md
- emqx.md — This templates requires additional traefik port mapping and entry point
- enshrouded.md
- erpnext.md
- etherpad.md
- evershop.md
- evolutionapi.md
- excalidraw.md
- ezbookkeeping.md
- filebrowser.md
- filestash.md
- firecrawl.md
- fivem.md — docker-compose.yml
- flagsmith.md — See https://docs.flagsmith.com/deployment/docker for more information on running…
- flaresolverr.md
- flatnotes-totp.md
- flatnotes.md
- flowise.md
- fmd-server.md
- focalboard.md
- fonoster.md
- forgejo.md
- formbricks.md
- frappe-hr.md
- freescout.md
- freshrss.md
- garage-with-ui.md
- garage.md
- ghost.md
- gitea-mirror.md
- gitea-mysql.md
- gitea-postgres.md
- gitea-sqlite.md
- gitingest.md
- gitlab-ce.md
- glance.md
- glitchtip.md
- glpi.md
- go-whatsapp-web-multidevice.md
- gotenberg.md
- grafana.md
- grimoire.md
- grist.md
- habitica.md
- heyform.md
- hi-events.md
- hoarder.md
- homarr.md
- homeassistant.md
- homebridge.md
- hoppscotch.md
- hortusfox.md
- huly.md
- i18n-blog.md
- ihatemoney.md
- imgproxy.md
- immich.md
- index.md
- infisical.md
- influxdb.md
- inngest.md
- instantdb.md
- invoiceshelf.md
- ipfs.md
- it-tools.md
- java.md
- jellyfin.md
- jenkins.md
- kaneo.md
- karakeep.md
- kener.md
- kestra.md
- keycloak.md
- kimai.md
- kitchenowl.md
- kokoro-tts.md
- kokoro-web.md
- komari-monitor.md
- kutt.md
- langflow.md
- lavalink.md
- letterfeed.md
- librechat.md — LibreChat Docker Compose for Dokploy Template
- libredesk.md
- libretranslate.md
- linkding.md
- linkstack.md
- linkwarden.md
- listmonk.md
- litellm.md
- livekit.md
- lobe-chat.md
- lodestone.md
- logto.md
- lowcoder.md
- macos.md
- mage-ai.md — https://docs.mage.ai/getting-started/setup#docker-compose
- mailpit.md
- mattermost.md
- mautic.md
- maybe.md
- mazanoke.md
- mcsmanager.md
- mealie.md
- mediacms.md
- mediafetch.md
- meilisearch.md
- memos.md
- metabase.md
- metube.md
- minepanel.md
- minio.md — comment the line below if you specified a local directory in the volumes section…
- misaka-danmu-server.md
- mixpost.md
- moltbot.md
- morphos.md
- movary.md
- mulesoft-esb.md
- mumble.md
- n8n-runner-postgres-ollama.md
- n8n-with-postgres.md
- n8n.md
- navidrome.md
- neko.md
- netdata.md
- networking-toolbox.md
- nextcloud-aio.md
- nginx.md
- nocodb.md
- notifuse.md
- ntfy.md
- obsidian-livesync.md
- odoo_17.md
- odoo_18.md
- odoo_19.md
- ojs.md
- omni-tools.md
- onedev.md
- onetimesecret.md
- ontime.md
- open-fiesta.md
- open-webui.md — This is the main service for the Open WebUI interface.
- open_notebook.md
- openclaw.md
- opengist.md — docker-compose.yml
- openhands.md — The 'version' attribute is obsolete and has been removed.
- openinary.md
- openpanel.md
- openresty-manager.md
- openspeedtest.md
- otterwiki.md
- outline.md
- owncast.md
- palmr.md
- parseable.md
- passbolt.md — =============================================================================
- pastefy.md
- paymenter.md
- peerdb.md
- penpot.md — demo-users
- peppermint.md
- pgadmin.md
- photoprism.md
- phpmyadmin.md
- picsur.md
- pinchflat.md
- plane.md
- plark.md
- plausible.md
- plunk.md — IMPORTANT: Plunk requires HTTPS to work properly
- pocket-id.md
- pocketbase.md — IMPORTANT: Please update the admin credentials in your .env file
- poke.md
- portainer.md
- poste-io.md
- postgres-pgdog.md
- postgresus.md
- postiz.md
- prometheus.md
- pterodactyl.md
- pulse.md
- pyrodactyl.md
- qbittorrent.md — docker-compose.yml
- qbitwebui.md
- qdrant.md
- quant-ux.md
- rabbitmq.md
- reactive-resume.md
- registry.md
- rocketchat.md
- rote.md
- roundcube.md
- rss-bridge.md
- rsshub.md
- rustdesk.md
- rustfs.md
- rutorrent.md
- rybbit.md — https://www.rybbit.io/docs/self-hosting-advanced
- ryot.md
- scrutiny.md — PROXMOX USERS: Proxmox LXCs don't have access to S.M.A.R.T data, while this may …
- scrypted.md
- seafile.md — https://manual.seafile.com/12.0/setup/setup_ce_by_docker
- searxng.md
- seaweedfs.md
- shlink.md
- signoz.md
- silverbullet.md
- slash.md
- snapp.md
- soketi.md
- spacedrive.md
- stack-auth.md
- stalwart.md
- statping-ng.md
- stirling.md
- storyden.md
- strapi.md — Self-host guide:
- streamflow.md
- supabase.md — Self-hosted Supabase, tracking the official compose from
- superset.md — This is an UNOFFICIAL production docker image build for Superset:
- surrealdb.md
- syncthing.md
- tailscale-exitnode.md
- teable.md
- tianji.md
- tolgee.md
- tooljet.md
- tor-browser.md
- trailbase.md — IMPORTANT: The initial admin credentials will be printed in the logs after the c…
- triggerdotdev.md
- trilium-next.md — Running `docker-compose up` will create/use the "trilium-data" directory in the …
- trilium.md
- trmnl-byos-laravel.md
- tuwunel.md
- twenty.md
- typebot.md
- typecho.md
- typesense.md
- umami.md
- unifi.md
- unleash.md — The default users credentials are:
- upsnap.md
- uptime-kuma.md
- usesend.md
- valkey.md
- vault.md
- vaultwarden.md — the webserver can take a while to start up, so don't be alarmed if it takes a fe…
- verdaccio.md
- vikunja.md
- wallos.md
- wanderer.md
- web-check.md
- wg-easy.md
- wikijs.md
- windmill.md
- windows.md
- wordpress.md
- wuzapi.md
- xsshunter.md
- yamtrack.md
- yourls.md
- yt-dlp-webui.md
- zabbix.md
- zipline.md
- zitadel.md

### rustploy-docs/src/content/docs/tutorial/
- primeiro-deploy.mdx

### rustploy-docs/src/styles/
- custom.css

### rustploy-gui/ → gui.md
- .gitignore
- .luaurc
- Cargo.lock
- Cargo.toml — Rustploy — desktop client (glacier-ui) for the Rustploy PaaS daemon
- README.md — rustploy-gui por dentro
- build.rs — Build script: logos dos blueprints para o release, `.desktop` no `cargo install`…
- glacier.jsonc
- stylua.toml

### rustploy-gui/assets/
- application.manifest
- rustploy.rc
- (+2 imagens/fontes)

### rustploy-gui/locales/
- en.json
- pt-BR.json

### rustploy-gui/packaging/
- br.com.rustploy.gui-update.policy
- postinst
- rustploy-gui.desktop

### rustploy-gui/src/ → gui.md
- embedded.rs — Assets embutidos no binário — modo standalone (só em builds de release).
- main.rs — Rustploy (glacier-ui) — desktop client whose UI is described in XML templates an…

### rustploy-gui/src/app/ → gui.md
- mod.rs — Rustploy (glacier-ui) — desktop client whose UI is described in XML templates an…
- updater.rs — A ponte nativa do "Atualizar app": o que o Luau não consegue fazer sozinho (baix…

### rustploy-gui/src/bin/rustploy-gui-update/ → gui.md
- helper.rs — O helper de verdade (só Unix: `geteuid`, `getpwuid_r`, dpkg).
- main.rs — `rustploy-gui-update` — o helper root que instala o `.deb` **assinado** do app d…
- portable.rs — O helper no Windows: troca a pasta do app portátil (`%LOCALAPPDATA%\Rustploy`) p…

### rustploy-gui/tests/ → gui.md
- app_manifest.rs — O que o Luau não alcança: o manifesto do `app(...)` (título, tamanho e fontes da…
- locales.rs — Os catálogos de idioma (`locales/en.json` e `locales/pt-BR.json`) e as views que…

### rustploy-gui/tests/e2e-updater-gui/
- Containerfile
- run.sh

### rustploy-gui/tests/fixtures/ → gui.md
- access_key_fixture.gv
- access_key_fixture.luau — Fixture do teste `access_key.rs` (nome diferente do módulo, senão o `require` re…
- compose_host.gv
- compose_host.luau — Fixture do teste `format_service_detail.rs`: exercita `compose_host` e `internal…
- saved_action.gv
- saved_action.luau — Fixture do teste `busy.rs`.
- time.gv
- time.luau — Fixture do teste `format_time.rs`: exercita o `format/time.luau` de verdade, atr…

### rustploy-gui/tests/fixtures/windows-update/
- arm.zip.minisig
- good.zip
- good.zip.minisig
- noexe.zip
- noexe.zip.minisig
- otherpkg.zip.minisig
- outside.zip
- outside.zip.minisig
- slip.zip
- slip.zip.minisig
- test.pub

### rustploy-gui/tests/format/ → gui.md
- dashboard.test.luau
- git_registry.test.luau
- jobs.test.luau
- service_detail.test.luau
- state_git.test.luau
- time.test.luau
- util.test.luau

### rustploy-gui/tests/handlers/ → gui.md
- connection.test.luau — Sessão e login (handlers/connection.luau).
- dados.test.luau — Aba Dados: sem conexão os handlers são inertes (o botão pode ser clicado quando …
- env_edit.test.luau — Edição no lugar da chave/valor de uma variável (handlers/secrets.luau), sem cone…
- misc.test.luau — Handlers que só mexem no ctx, ou que não fazem nada sem conexão: deploy_queue, s…
- nav.test.luau — Navegação da sidebar, abas e busca (handlers/nav.luau), sem conexão.
- registry_picker.test.luau — Sub-aba Registry do serviço: o select de repositório/tag monta a imagem do regis…
- registry_retention.test.luau — Retenção de tags do registry: o botão Salvar da sub-aba Docker > Registry.

### rustploy-gui/tests/lib/ → gui.md
- access_key.test.luau
- api.test.luau
- busy.test.luau — busy.luau envolve handlers globais numa trava: enquanto um roda, a chave `busy_<…
- busy_actions.test.luau — A lista de ações com trava é mantida por `tools/busy_actions.py --check` contra …
- helpers.test.luau

### rustploy-gui/tests/tela/ → gui.md
- missing.test.luau — Toda chave que uma tela LÊ tem de ser publicada por alguém.
- render.test.luau — O que a tela MOSTRA, não só o estado que os handlers escrevem.
- running.test.luau

### rustploy-gui/tests/windows/ → gui.md
- new_job.test.luau — Janela "Novo job" (scripts/new_job_window.luau): passos do assistente, modo ediç…
- windows.test.luau — Scripts das janelas auxiliares (cada uma é um motor isolado, aberto por open_win…

### rustploy-gui/vendor/
- README.md — vendor/

### rustploy-gui/vendor/iced_tiny_skia/
- Cargo.toml — A software renderer for iced on top of tiny-skia

### rustploy-gui/vendor/iced_tiny_skia/src/ → gui.md
- engine.rs
- geometry.rs
- layer.rs
- lib.rs
- primitive.rs
- raster.rs
- settings.rs
- text.rs
- vector.rs
- window.rs

### rustploy-gui/vendor/iced_tiny_skia/src/window/ → gui.md
- compositor.rs

### rustploy-gui/views/ → gui-views.md
- app.gv
- home.component.gv
- log_window.screen.gv
- login.component.gv
- new_job_window.screen.gv
- new_project_form.screen.gv
- new_registry_token_window.screen.gv
- new_service.component.gv
- new_service_window.screen.gv
- service.component.gv
- service_export_window.screen.gv
- services_reorder_window.screen.gv
- shell.component.gv

### rustploy-gui/views/components/ → gui-views.md
- badge.component.gv
- job_actions.component.gv
- loading_row.component.gv
- nav_item.component.gv
- picker_row.component.gv
- project_card.component.gv
- running_card.component.gv
- service_card.component.gv
- stat_card.component.gv
- state_cell.component.gv
- tab_button.component.gv
- table_link.component.gv
- template_row.component.gv

### rustploy-gui/views/home/ → gui-views.md
- deploy_engine.component.gv
- docker.component.gv
- ingress.component.gv
- monitoring.component.gv
- running.component.gv
- schedules.component.gv
- settings.component.gv

### rustploy-gui/views/home/deploy_engine/ → gui-views.md
- executando.component.gv
- fila.component.gv
- historico.component.gv

### rustploy-gui/views/home/docker/ → gui-views.md
- containers.component.gv
- images.component.gv
- networks.component.gv
- registry.component.gv
- volumes.component.gv

### rustploy-gui/views/home/settings/ → gui-views.md
- git.component.gv
- gitsync.component.gv
- language.component.gv
- maintenance.component.gv
- updates.component.gv
- web.component.gv

### rustploy-gui/views/scripts/ → gui-scripts.md
- access_key.luau — o token de conexão: endereço do daemon + token da API numa string só, para o log…
- app.luau — ponto de entrada do `<script>` de app.gv.
- busy.luau — trava de "ação em andamento" para os handlers de on_click/on_submit.
- busy_actions.luau — ações (on_click/on_submit) que ganham a trava de busy.luau.
- format.luau — fachada: reexporta os builders de views/scripts/fmt_*.luau sob um único módulo, …
- glacier.d.luau — Definições dos globais que o motor glacier-ui injeta no interpretador Luau em ru…
- helpers.luau — utilitários puros compartilhados pelos handlers_*.luau (sem estado, sem I/O).
- log_window.luau — script da JANELA de logs (runtime OU build) de um serviço/deployment, um motor G…
- new_job_window.luau — script da JANELA "Novo job", um motor Glacier próprio e ISOLADO do app principal…
- new_project_window.luau — script da JANELA de "Novo projeto", um motor Glacier próprio e ISOLADO do app pr…
- new_registry_token_window.luau — script da JANELA "Novo token do registry", um motor Glacier próprio e ISOLADO do…
- new_service_window.luau — script da JANELA do wizard "Novo serviço", um motor Glacier próprio e ISOLADO do…
- service_export_window.luau — script da JANELA "Exportar serviço", um motor Glacier próprio e ISOLADO do app p…
- services_reorder_window.luau — script da JANELA "Reordenar serviços", um motor Glacier próprio e ISOLADO do app…
- state.luau — estado mutável compartilhado entre todos os handlers/*.luau (mesmo interpretador…
- stylua.toml

### rustploy-gui/views/scripts/format/ → gui-scripts.md
- dashboard.luau — builders de lista do dashboard (deployments/projects/ services/docker/ingress/mo…
- docker_cleanup.luau — resumo textual da limpeza automática de Docker (Settings → Manutenção).
- git.luau — builders dos provedores/repositórios/branches Git conectados (Gitea/GitHub) — Se…
- jobs.luau — formata Job/JobSummary/JobRun (tarefas one-shot via docker-compose) pra exibição…
- registry.luau — formata repositórios/tags do registry OCI embutido pra exibição na sub-aba Docke…
- running.luau — painel "Em execução".
- service_detail.luau — builders da tela de detalhe de serviço (source, healthcheck, env vars, logs, dep…
- state_git.luau — a sincronização do estado com o git (Fases 6 e 7 de docs/plano-backup-estado-em-…
- time.luau — timestamps e durações.
- types.luau — tipos compartilhados entre os submódulos fmt_*.luau (as formas dos modelos que c…
- util.luau — busca, codificação de array e mapas de estado (paleta = view.rs).

### rustploy-gui/views/scripts/handlers/ → gui-handlers.md
- bundle.luau — copiar um serviço para outro servidor (lado do app principal): o botão "Exportar…
- bundle_import.luau — passo "Importar" do wizard "Novo serviço": recria aqui um serviço exportado de o…
- connection.luau — ciclo de vida da sessão: init (semeia o contexto), login/logout, configurações d…
- deploy_queue.luau — gerência da fila global de deploys (um por vez) na tela Deploy Engine: cancelar/…
- docker.luau — limpeza de recursos Docker sem uso (imagens, volumes, redes).
- history.luau — a pilha do "voltar" (Backspace, Alt+← e o botão "voltar" do mouse; o motor chama…
- jobs.luau — ações da tela global "Schedules" (sidebar) e da aba "Jobs" do projeto: rodar ago…
- language.luau — idioma da interface: restaura o escolhido da última vez, troca ao vivo e lembra.
- nav.luau — navegação da sidebar/tabs e busca do topbar.
- projects.luau — grade de projetos: criar/editar/remover projeto, variáveis de ambiente de projet…
- registry.luau — sub-aba Docker > Registry: navega repo→tags (fetch sob demanda, tags não vêm no …
- secrets.luau — aba "Secrets" do projeto: criar/sobrescrever e apagar valores cifrados, e o atal…
- services.luau — detalhe do serviço (service.component.gv): fetch completo, mutações de spec/env,…
- settings.luau — Settings (Web Server) e Settings → Git (provedores Gitea: conectar via OAuth/PAT…
- state_git.luau — Settings → Sync: a sincronização do estado de infra com um repositório git (Fase…
- stream.luau — consumidor do SSE de /api/events: aplica o snapshot periódico (2s) e os eventos …
- wizard.luau — wizard "Novo serviço".

### rustploy-gui/views/scripts/net/ → gui-scripts.md
- api.luau — cliente HTTP/JSON da API do daemon.

### rustploy-gui/views/service/ → gui-views.md
- advanced.component.gv
- connection.component.gv
- dados.component.gv
- databases.component.gv
- deployments.component.gv
- domains.component.gv
- environment.component.gv
- general.component.gv
- general_compose.component.gv
- general_git.component.gv
- general_gitea.component.gv
- general_registry.component.gv
- general_zip.component.gv
- healthcheck.component.gv
- logs.component.gv
- migrar.component.gv

### rustploy-gui/views/shell/ → gui-views.md
- deployments.component.gv
- project_env.component.gv
- project_jobs.component.gv
- project_secrets.component.gv
- project_services.component.gv
- projects.component.gv

### rustploy-gui/views/styles/ → gui-views.md
- app.gss — Rustploy — glacier-ui stylesheet.
- theme.json

### rustploy-shared/
- .gitignore
- Cargo.toml — Shared types and protocol definitions for the Rustploy PaaS platform

### rustploy-shared/src/ → shared.md
- lib.rs — Tipos compartilhados entre o daemon e os clientes: domínio, protocolo e nomes de…
- naming.rs — Nomes derivados de um projeto ou serviço: containers, redes, stacks Compose e al…

### rustploy-shared/src/domain/ → shared.md
- audit.rs — Auditoria de valores em claro no estado de infra (plano `docs/plano-backup-estad…
- database.rs
- deployment.rs
- docker_inventory.rs
- env.rs
- git.rs
- ingress.rs
- job.rs
- metrics.rs
- mod.rs — Modelos de domínio: projeto, serviço e suas fontes, deployment, jobs, bancos, re…
- project.rs
- registry.rs
- service.rs
- service_bundle.rs
- service_data.rs — Dados de um serviço (volumes) que podem ser exportados e importados.
- source.rs
- state_git.rs — Sincronização do estado de infra com um repositório git (plano `docs/plano-backu…
- update.rs — Verificação de versão: o que o daemon responde a `Command::CheckUpdate`.
- wizard.rs

### rustploy-shared/src/protocol/ → shared.md
- command.rs — `Command`: tudo o que o cliente pede ao daemon.
- event.rs — `Event`: o que o SSE entrega ao cliente.
- logs.rs — Linhas de log: stream de origem e entradas de container/build.
- mod.rs — Protocolo da API: `Command` (o que o cliente pede), `Response` e `Event` (o que …
- response.rs — `Response`: a resposta a cada `Command`.

### rustploy-shared/update-core/
- Cargo.toml — Shared update logic for the Rustploy daemon and desktop app: release version com…

### rustploy-shared/update-core/src/ → shared.md
- dpkg_version.rs — Comparação de versões de **pacote Debian** (`1:0.1.10-1`), pelo algoritmo da pol…
- installer.rs — A instalação de um pacote assinado, em duas fases:
- lib.rs — Lógica de atualização comum ao daemon e ao app: um só lugar decide se uma versão…
- outcome.rs — O resultado da última atualização, gravado pelo updater (root) e lido pelo daemo…
- package.rs — A decisão "este pacote assinado pode ser instalado aqui?", como função pura: cru…
- release.rs — Os arquivos de uma release do GitHub e a escolha do pacote certo para esta máqui…
- signature.rs — Verificação da assinatura minisign de um pacote e leitura do comentário confiáve…
- staging.rs — Cópia segura do pacote para um diretório que só o root controla.
- system.rs — Tudo que o updater pede ao sistema (rodar comando, esperar) passa por aqui, para…
- version.rs — Comparação de versões de release (semver), tolerante ao `v` das tags.

### rustploy-shared/update-core/tests/fixtures/
- other.pub
- payload.bin
- payload.bin.minisig
- payload.noclaims.minisig
- test.pub

### tools/
- busy_actions.py
- sign-release.sh

### tools/indexer/
- Cargo.lock
- Cargo.toml — Gera docs/indice/: índice de arquivos e símbolos para busca barata por agentes

### tools/indexer/src/ → tools.md
- commands.rs — `comandos.md`: uma linha por variante de `Command` ligando as três pontas de uma…
- main.rs — Gera `docs/indice/`: um mapa de arquivos e símbolos pensado para um agente achar…
- script.rs — Índice dos arquivos que não são Rust: scripts Luau e JS, templates `.gv`/`.gvb`,…
