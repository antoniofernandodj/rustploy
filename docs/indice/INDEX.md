# Índice do repositório

> Gerado por `make index`; não editar à mão. Sem números de linha:
> ache o arquivo aqui, o símbolo no índice da área e a linha com
> `grep -n "fn nome" <arquivo>`. Leia só o trecho, nunca o arquivo grande inteiro.

## Índices de símbolos por área

- `shared.md` — rustploy-shared — modelos, protocolo, manifest, templates (rustploy-shared/)
- `daemon-api.md` — daemon: API HTTP e handlers de Command (rustploy-daemon/crates/daemon/src/api/)
- `daemon-deploy.md` — daemon: deploy, jobs e manutenção (rustploy-daemon/crates/daemon/src/deploy/, rustploy-daemon/crates/daemon/src/jobs/, rustploy-daemon/crates/daemon/src/maintenance/)
- `daemon-db.md` — daemon: persistência (db/) (rustploy-daemon/crates/daemon/src/db/)
- `daemon-docker.md` — daemon: Docker/Compose e ingress (proxy, TLS) (rustploy-daemon/crates/daemon/src/docker/, rustploy-daemon/crates/daemon/src/ingress/)
- `daemon-registry.md` — daemon: registry embutido e provedores git (rustploy-daemon/crates/daemon/src/registry/, rustploy-daemon/crates/daemon/src/git_providers/)
- `webui.md` — webui (HTML + Alpine.js) servida pelo daemon (rustploy-daemon/crates/daemon/webui/)
- `daemon-core.md` — daemon: main, bins, event bus, secrets, métricas e o resto (rustploy-daemon/crates/daemon/)
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
- CLAUDE.md — CLAUDE.md
- CONTINUE.md — rustploy-gui — plano de continuação
- GEMINI.md — Rustploy
- Makefile
- README.md — Rustploy
- TODO.md — TODO

### .claude/
- settings.json

### .github/workflows/
- release.yml

### .vscode/
- settings.json

### docs/
- api-agente-no-gui.md — A API de agente vive no app, não no daemon
- correcao-persistencia-geometria-janela.md — Correção: a janela não reabria no último tamanho (glacier 0.49.0 → 0.49.1)
- empacotamento-styles-e-cross-compile-windows.md — Empacotamento do rustploy-gui: mover styles/ e cross-compile Windows (cargo-xwin…
- env-backup.md — Backup automático de env vars
- ingress-proxy.md — Como funciona o ingress proxy e o zero-downtime deploy
- internal-networking.md — Comunicação interna entre serviços do projeto
- inventario-paridade-telas.md — Inventário de paridade — GUI iced × webui
- licoes-aprendidas.md — Lições aprendidas — erros, causas e soluções
- memoria-threads-e-runtime.md — Memória de threads e runtime async — base teórica
- plano-acao-banco-compartilhado.md — Plano de ação: implementar o banco compartilhado (Postgres) e migrar rdo-itemize…
- plano-banco-compartilhado.md — Plano: banco compartilhado entre projetos + migração dos bancos antigos
- plano-cancelamento-de-jobs.md — Cancelamento de `job_run` em andamento
- plano-colisao-nome-container.md — Plano: colisão de nome de container entre projetos
- plano-convergencia-templates-gui-webui.md — Plano: convergência dos templates (GUI iced ↔ web UI)
- plano-copiar-servico-entre-servidores.md — Copiar um serviço de um servidor para outro
- plano-dependencias-e-autostart.md — Dependências entre serviços + auto-restart no boot do daemon
- plano-erro-de-deploy-invisivel.md — Erro de deploy invisível: a causa da falha é gravada, mas nunca chega no log que…
- plano-fila-deploys.md — Plano: fila global de deploys (um por vez), visível e gerenciável
- plano-file-io-luau-e-geometria.md — Plano: I/O de arquivo no Luau + geometria da janela fora do Rust
- plano-indice-de-codigo.md — Plano: índice de código para busca barata
- plano-jobs-na-fila-de-deploy.md — Plano: jobs e deploys na mesma fila (uma coisa por vez)
- plano-limpeza-automatica-docker.md — Limpeza automática do Docker: liberar espaço em disco sozinho, todos os dias
- plano-multi-login-clients.md — Plano: login multi-servidor no client iced + login simplificado na webui
- plano-nome-gravado-rede-e-stack.md — Plano: nome de rede e de stack Compose gravados, não derivados
- plano-nomes-descritivos.md — Plano: nomes descritivos em toda a base (GUI glacier + webui)
- plano-pre-deploy-gate.md — Pré-deploy gate: rodar um check antes do deploy, e só prosseguir se ele passar
- plano-reforma-gui-glacier-0.102.md — Plano: reforma do `rustploy-gui` sobre o glacier-ui moderno (0.87 → 0.102)
- plano-registry-embutido.md — Registry Docker embutido no rustployd
- plano-tray-bandeja-e-ciclo-de-vida.md — Plano — ícone de bandeja e o app que sobrevive à última janela
- plano-unificacao-webhook-api.md — Plano: unificar o webhook na porta da API
- plano-webui-janelas-e-polimento.md — Plano: webui com acabamento da GUI, janelas arrastáveis e paridade total
- plano-widgets-glacier-0.68.md — Widgets novos do glacier-ui (0.63 → 0.68): o que dá pra aproveitar no rustploy
- prompt-depurar-infra-com-agente.md — Prompt: depurar a infra pela API de agente
- relatorio-porta-externa-automatica.md — Relatório: URL de conexão externa sem burocracia — porta automática + firewall g…
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

### rustploy-daemon/
- .gitignore
- Cargo.lock
- Cargo.toml
- README.md — Rustploy
- docker-compose.test.yml
- install.sh
- rustploy
- rustployd

### rustploy-daemon/.claude/
- settings.json

### rustploy-daemon/crates/daemon/ → daemon-core.md
- Cargo.toml — Rustploy PaaS daemon — lightweight self-hosted deployment platform
- build.rs — Gera, em tempo de compilação, os assets estáticos da web UI/PWA (`crates/daemon/…

### rustploy-daemon/crates/daemon/src/ → daemon-core.md
- env_backup.rs — Backup periódico das env vars de projetos e serviços em snapshots JSON, com list…
- env_switch.rs — Troca de uma env var de conexão (`DATABASE_URL`…) num projeto, com o valor antig…
- event_bus.rs — Bus de eventos em memória (broadcast): o que alimenta o SSE `/api/events` dos cl…
- firewall.rs — Cliente do helper privilegiado de firewall (`rustployd-fw`).
- health.rs — Checagens de saúde HTTP e TCP usadas pelo healthcheck do deploy e pelo watchdog.
- logs.rs — Streaming dos logs dos containers gerenciados para o event bus, reconectando e e…
- main.rs — Ponto de entrada do `rustployd`: resolve config e diretórios, sobe banco, Docker…
- metrics.rs — Coleta periódica de métricas de CPU/memória/disco do host e por container, publi…
- migration.rs — Assistente de migração: banco antigo (serviço Compose do projeto) → database ger…
- ports.rs — Alocação automática de portas externas (`ServiceSpec.host_port`).
- secrets.rs — `SecretsManager`: secrets por projeto cifrados no banco com a chave mestra do da…
- shared_db.rs — Provisionamento de database + usuário dentro de um servidor de banco compartilha…
- watchdog.rs — Watchdog dos serviços no ar: checa se o container roda e passa no healthcheck, r…

### rustploy-daemon/crates/daemon/src/api/ → daemon-api.md
- http_api.rs — HTTP/JSON + SSE control API — the daemon's remote administrative channel.
- mod.rs — API do daemon: `AppState` (estado compartilhado por todos os handlers) e os cach…
- public_routes.rs — Rotas HTTP **públicas** (sem Bearer): o webhook de deploy e o callback OAuth do …
- routes.rs — `dispatch`: o `match` que manda cada `Command` para o seu handler em `handlers/`…
- web_ui.rs — Servidor de estáticos da web UI/PWA (`crates/daemon/webui/`) — alternativa ao cl…

### rustploy-daemon/crates/daemon/src/api/handlers/ → daemon-api.md
- 69 arquivos, um arquivo por `Command` (exceto `mod.rs`); o que cada um faz está em `comandos.md` e `daemon-api.md`: daemon_status, deploy_abort, deploy_delete, deploy_engine_status, deploy_history, deploy_queue_pause, deploy_queue_promote, deploy_queue_reorder, deploy_rollback, deploy_start, docker_cleanup, docker_inventory, docker_prune, docker_remove, env_backup, get_build_logs, get_daemon_settings, get_job_logs, get_webhook_url, git_branch_list, git_oauth_start, git_provider_create, git_provider_delete, git_provider_list, git_repo_list, ingress, job_create, job_delete, job_list, job_list_all, job_run_cancel, job_run_history, job_run_now, job_update, logs_get, managed_database, manifest_apply, manifest_export, manifest_export_all, manifest_import, migration, mod, ping, project_create, project_delete, project_env_set, project_list, project_update, recent_deployments, reconcile, regenerate_webhook_token, registry, secret_delete, secret_list, secret_set, service_archive_upload, service_connection_info, service_create, service_delete, service_export, service_get, service_import, service_list, service_reload, service_stop, service_update, set_daemon_settings, shared_access, wizard

### rustploy-daemon/crates/daemon/src/bin/ → daemon-core.md
- rustployd-fw.rs — `rustployd-fw` — helper privilegiado de firewall do rustploy.

### rustploy-daemon/crates/daemon/src/db/ → daemon-db.md
- build_logs.rs — Tabela `build_log`: linhas do log de build de cada deployment.
- daemon_settings.rs — Tabela chave-valor de configurações do daemon editáveis pela UI (ACME, registry,…
- deployments.rs — Tabela `deployment`: criar, transicionar de estado, histórico por serviço e esta…
- git_providers.rs — Persistence for connected Git providers (Gitea OAuth2 / PAT).
- job.rs — Tabela `job`: jobs one-shot (Schedules), incluindo quais estão vencidos para o a…
- job_log.rs — Tabela `job_log`: linhas de log (stdout/stderr) de cada execução de job.
- job_run.rs — Tabela `job_run`: cada execução de um job e seu exit code.
- managed_database.rs — Tabela `managed_database`: databases criados em servidores compartilhados.
- migration.rs — Tabela `migration`: estado do assistente de migração (passos, log, o que foi par…
- mod.rs — Conexão SQLite (`Db`) e as migrações do schema, feitas à mão com `add_column_if_…
- projects.rs — Tabela `project`: CRUD de projetos e suas env vars de nível de projeto.
- registry.rs — Wrappers SQL do registry OCI embutido (metadados; os bytes de blob/manifest vive…
- registry_tokens.rs — Tokens de acesso do registry OCI embutido (Basic auth — ver `crate::registry::au…
- services.rs — Tabela `service`: CRUD do ServiceSpec, status e container live.
- shared_access.rs — Tabela `shared_access`: quais projetos podem alcançar cada servidor de banco com…
- webhook_tokens.rs — Tabela de tokens de webhook de deploy, um por serviço.

### rustploy-daemon/crates/daemon/src/deploy/ → daemon-deploy.md
- env_resolve.rs — Resolução de env vars com secrets decifradas — extraído de `DeployExecutor::reso…
- executor.rs — `DeployExecutor`: roda um deployment pela máquina de estados (clone/pull/build, …
- git.rs — Clone de repositório git para build, com progresso, credenciais de provedor cone…
- mod.rs — Motor de deploy: fila global, executor, recuperação no boot, clone git e resoluç…
- queue.rs — Fila **global** de deploys: no máximo um deploy rodando por vez no daemon.
- recovery.rs — Recuperação no boot: aborta deploys interrompidos, reconcilia status com o Docke…
- shared_net.rs — Servidor de banco compartilhado: conecta o container dele às redes dos projetos …

### rustploy-daemon/crates/daemon/src/docker/ → daemon-docker.md
- compose.rs — Serviços e jobs Docker Compose: `up` de stack com a rede do projeto injetada, ex…
- containers.rs — Containers de serviços Application: nomes (live, staging, réplicas, legado), bus…
- images.rs — Imagens: pull (com credenciais) e build a partir de um contexto empacotado em ta…
- mod.rs — Cliente Docker (`DockerClient`, via bollard) e os submódulos por recurso.
- networks.rs — Rede Docker por projeto: nome e criação sob demanda.

### rustploy-daemon/crates/daemon/src/git_providers/ → daemon-registry.md
- gitea.rs — Minimal Gitea API client: OAuth2 token exchange/refresh plus the few REST endpoi…
- github.rs — Minimal GitHub API client: OAuth2 token exchange/refresh plus the few REST endpo…
- mod.rs — Clients for hosted Git providers.

### rustploy-daemon/crates/daemon/src/ingress/ → daemon-docker.md
- mod.rs — Ingress: proxy reverso HTTP/HTTPS embutido, tabela de rotas e TLS/ACME.
- proxy.rs — Proxy reverso HTTP/1.1 embutido, construído sobre hyper.
- router.rs — Tabela de rotas do ingress (domínio → backends, porta → backends) com round-robi…
- tls.rs — Certificados TLS: resolução por SNI, emissão e renovação ACME (Let's Encrypt) e …

### rustploy-daemon/crates/daemon/src/jobs/ → daemon-deploy.md
- mod.rs — Jobs one-shot (Schedules): execução (`runner`) e agendamento (`scheduler`).
- runner.rs — Execução de um `Job` (tarefa one-shot via docker-compose): resolve rede + env va…
- scheduler.rs — Ticker de agendamento dos jobs one-shot — mesmo formato de `metrics.rs`/`env_bac…

### rustploy-daemon/crates/daemon/src/maintenance/ → daemon-deploy.md
- mod.rs — Limpeza automática (agendada) de recursos Docker não usados — ver `docs/plano-li…
- run.rs — Execução de uma limpeza (agendada ou "Executar agora"): roda os recursos marcado…
- scheduler.rs — Ticker que verifica se a limpeza automática de Docker está devida — mesmo idioma…

### rustploy-daemon/crates/daemon/src/registry/ → daemon-registry.md
- auth.rs — Basic auth do registry OCI embutido — checada em TODA rota (inclusive `GET /v2/`…
- error.rs — Envelope de erro da OCI Distribution Spec: `{"errors":[{"code","message","detail…
- gc.rs — Garbage collection do registry: libera do disco o que nenhuma tag alcança.
- http.rs — Rotas HTTP da OCI Distribution API v2 — dispatch manual (hyper cru não tem route…
- internal_token.rs — Token interno usado pelo próprio deploy executor pra puxar imagens do registry e…
- mod.rs — Registry Docker OCI Distribution API v2 embutido — push/pull, GC e Basic auth po…
- name.rs — Validação de `<name>`, `<reference>` (tag) e `<digest>` da OCI Distribution Spec…
- storage.rs — CAS (content-addressable store) do registry: blobs em disco, sessões de upload e…

### rustploy-daemon/crates/daemon/webui/ → webui.md
- app.css — Rustploy — web UI stylesheet.
- app.js — único <script type="module"> carregado por index.html.
- busy.js — feedback imediato e trava de clique para toda ação assíncrona.
- directives.js — diretivas Alpine próprias da webui.
- fmt.js — timestamps, durações e paleta de estado.
- icons.js — conjunto de ícones SVG inline (traço 1.75, estilo Lucide) e a diretiva `x-icon="…
- index.html — Casca única da webui (Alpine.js): login, shell e todas as telas, uma seção por v…
- manifest.webmanifest
- sw.js — service worker do PWA Rustploy.
- wm.js — gerenciador de janelas da webui (diretiva Alpine `x-win`).

### rustploy-daemon/crates/daemon/webui/net/ → webui.md
- api.js — cliente HTTP/JSON do daemon.
- sse.js — consumidor de endpoints SSE do daemon: o firehose `/api/events` (porta de crates…

### rustploy-daemon/crates/daemon/webui/screens/ → webui.md
- dashboard.js — tela "Deployments" (view padrão do shell).
- deploy_engine.js — tela "Deploy Engine": fila global (um deploy por vez), execução em andamento e h…
- docker.js — tela "Docker": containers/imagens/volumes/networks do host inteiro (não só recur…
- ingress.js — tela "Ingress": rotas ativas no reverse proxy (por domínio) e portas TCP de host…
- login.js — tela de login.
- monitoring.js — tela "Monitoring": uso de CPU/memória do host e por container.
- new_service.js — wizard "Novo serviço", porta de new_service.gv (client iced): passo pick_type → …
- project_detail.js — projeto aberto (view=project_services no client iced): sub-abas Serviços/Variáve…
- projects.js — tela "Projects": grid de cards + criar/editar/remover.
- schedules.js — tela "Schedules": jobs one-shot (docker-compose) agendados ou manuais, de todos …
- service_bundle.js — copiar um serviço entre servidores (docs/plano-copiar-servico-entre-servidores.m…
- service_detail.js — detalhe de um serviço.
- settings.js — tela "Settings": Web Server / Git / Infra as Code.

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
- rustployd-fw.service
- rustployd-fw.socket
- rustployd.service

### rustploy-daemon/packaging/debian/
- postinst
- prerm

### rustploy-daemon/scripts/
- migrate_id_prefixes.sh

### rustploy-gui/ → gui.md
- .gitignore
- .luaurc
- Cargo.lock
- Cargo.toml — Rustploy — desktop client (glacier-ui) for the Rustploy PaaS daemon
- README.md — rustploy-gui por dentro
- build.rs — Build script: logos dos blueprints para o release e recursos do `.exe` no Window…

### rustploy-gui/assets/
- application.manifest
- rustploy.rc
- (+2 imagens/fontes)

### rustploy-gui/packaging/
- postinst
- rustploy-gui.desktop

### rustploy-gui/src/ → gui.md
- assets.rs — Runtime asset location.
- desktop.rs — `rustploy-gui --install-desktop`: integração com o desktop para quem instalou po…
- embedded.rs — Assets embutidos no binário — modo standalone (só em builds de release).
- main.rs — Rustploy (glacier-ui) — desktop client whose UI is described in XML templates an…
- manifest_zip.rs — Ponte Lua ↔ Rust para o `.zip` do Infra as Code.

### rustploy-gui/src/agent/ → gui.md
- actions.rs — Índice das ações dispatcháveis da UI.
- catalog.rs — `GET /agent/schema` — o documento de descoberta.
- client.rs — Cliente HTTP para o daemon rustploy remoto.
- handoff.rs — Arquivo de handoff: como um agente na mesma máquina descobre esta API.
- mod.rs — API de agente — um servidor HTTP local que empresta a sessão desta janela.
- routes.rs — Servidor hyper da API de agente e os handlers de cada rota.
- servers.rs — Os servidores que o usuário já usou nesta máquina.
- session.rs — A sessão da GUI (URL + token do daemon remoto), compartilhada com o servidor da …
- ui.rs — Controle da própria janela — o que antes só um clique alcançava.

### rustploy-gui/src/app/ → gui.md
- mod.rs — Rustploy (glacier-ui) — desktop client whose UI is described in XML templates an…

### rustploy-gui/tests/ → gui.md
- busy.rs — busy.luau no motor de verdade: o botão libera no toast da própria ação, não só q…
- fmt_service_detail.rs — O `fmt/service_detail.luau` (`compose_host` e `internal_url`) rodando no motor d…
- fmt_time.rs — O `fmt/time.luau` rodando no motor de verdade.
- templates_render.rs — Headless validation: every template parses, every screen/tab evaluates and build…

### rustploy-gui/tests/fixtures/ → gui.md
- acao_salva.gvb — Fixture do teste busy.rs: um botão cuja ação toasta e depois ainda faz outro fet…
- acao_salva.luau — Fixture do teste `busy.rs`.
- compose_host.gvb — Fixture do teste fmt_service_detail.rs: tela mínima que roda o fmt/service_detai…
- compose_host.luau — Fixture do teste `fmt_service_detail.rs`: exercita `compose_host` e `internal_ur…
- tempo.gvb — Fixture do teste fmt_time.rs: tela mínima que roda o fmt/time.luau e exibe o res…
- tempo.luau — Fixture do teste `fmt_time.rs`: exercita o `fmt/time.luau` de verdade, através d…

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
- app.gvb — O app Rustploy: a raiz do arquivo é o `app(...)`, e as telas — a janela principa…
- home.gvb — Telas globais da sidebar, cada uma numa seção por valor de view: Monitoring, Ing…
- log_window.gvb — Janela de LOGS AO VIVO (runtime OU build): motor Glacier próprio e ISOLADO do ap…
- login.gvb — Tela de login: URL do daemon e token, com a lista de servidores lembrados.
- new_job_window.gvb — Janela "Novo job": motor Glacier próprio, aberto por open_window a partir do app…
- new_project_form.gvb — Janela "Novo projeto": motor Glacier próprio, aberto por open_window a partir do…
- new_registry_token_window.gvb — Janela "Novo token do registry": motor Glacier próprio, aberto por open_window a…
- new_service.gvb — Wizard "Novo serviço" (view=new_service): tipo → formulário por tipo, espelhando…
- new_service_window.gvb — Janela do wizard "Novo serviço": motor Glacier próprio, aberto por open_window (…
- service.gvb — Detalhe de um serviço: cabeçalho com ações (deploy, stop, reload) e as abas Gene…
- service_export_window.gvb — Janela "Exportar serviço": motor Glacier próprio, aberto por open_window a parti…
- shell.gvb — Casca do app conectado: sidebar, topbar e as views de projeto (Deployments, Proj…

### rustploy-gui/views/components/ → gui-views.md
- badge.gvb — Variante "badge" da célula de estado (mesmo ponto + rótulo, mas com o espaçament…
- loading_row.gvb — Linha "Carregando dados…" com spinner.
- nav_item.gvb — Item de navegação da sidebar: ícone (sempre visível) + rótulo (some abaixo de 90…
- picker_row.gvb — Linha de escolha do wizard "Novo serviço": título + subtítulo à esquerda e um bo…
- project_card.gvb — Template "fragment": dois nós de topo (o slot vazio e o card) — o glacier-ui (0.…
- service_card.gvb — Card de serviço da aba "Serviços" de um projeto — o análogo do ProjectCard.
- stat_card.gvb — Tile de KPI do cabeçalho (STATUS/UPTIME/SERVICES/CPU/…).
- state_cell.gvb — Célula de estado das tabelas: o ponto colorido "●" + o rótulo, ambos na mesma co…
- tab_button.gvb — Botão de aba genérico.
- template_row.gvb — Linha do catálogo de templates de aplicação: logo à esquerda (vetor ou raster co…

### rustploy-gui/views/home/ → gui-views.md
- deploy_engine.gvb — Seção `deploy_engine` (view = deploy_engine) das telas globais; importada por ho…
- docker.gvb — Seção `docker` (view = docker) das telas globais; importada por home.gvb.
- ingress.gvb — Seção `ingress` (view = ingress) das telas globais; importada por home.gvb.
- monitoring.gvb — Seção `monitoring` (view = monitoring) das telas globais; importada por home.gvb…
- schedules.gvb — Seção `schedules` (view = schedules) das telas globais; importada por home.gvb.
- settings.gvb — Seção `settings` (view = settings) das telas globais; importada por home.gvb.

### rustploy-gui/views/home/deploy_engine/ → gui-views.md
- executando.gvb — Aba "Executando" do Deploy Engine; importada por deploy_engine.gvb.
- fila.gvb — Aba "Fila" do Deploy Engine; importada por deploy_engine.gvb.
- historico.gvb — Aba "Histórico" do Deploy Engine; importada por deploy_engine.gvb.

### rustploy-gui/views/home/docker/ → gui-views.md
- containers.gvb — Containers — um por serviço gerido pelo Rustploy (ligação com projeto/serviço é …
- images.gvb — Images — projeto/serviço é melhor esforço (inferido pela tag; imagens manuais/de…
- networks.gvb — Networks — projeto reconhecido pela convenção rp_net_&lt;id curto&gt;.
- registry.gvb — Registry — repositórios/tags do registry OCI embutido (Fase 1: só push/pull via …
- volumes.gvb — Volumes — Rustploy só usa bind mounts, então volumes nomeados aqui são sempre ex…

### rustploy-gui/views/home/settings/ → gui-views.md
- git.gvb — Git: contas conectadas + formulário de conexão
- iac.gvb — Infra as Code: o manifesto é um `.zip` com exatamente rustploy.yml (projetos/ser…
- maintenance.gvb — Manutenção: limpeza automática de recursos Docker sem uso (ver docs/plano-limpez…
- web.gvb — Web Server (default)

### rustploy-gui/views/scripts/ → gui-scripts.md
- app.luau — ponto de entrada do `<script>` de app.gvb.
- busy.luau — trava de "ação em andamento" para os handlers de on_click/on_submit.
- busy_actions.luau — ações (on_click/on_submit) que ganham a trava de busy.luau.
- fmt.luau — fachada: reexporta os builders de views/scripts/fmt_*.luau sob um único módulo, …
- glacier.d.luau — Definições dos globais que o motor glacier-ui injeta no interpretador Luau em ru…
- helpers.luau — utilitários puros compartilhados pelos handlers_*.luau (sem estado, sem I/O).
- log_window.luau — script da JANELA de logs (runtime OU build) de um serviço/deployment, um motor G…
- new_job_window.luau — script da JANELA "Novo job", um motor Glacier próprio e ISOLADO do app principal…
- new_project_window.luau — script da JANELA de "Novo projeto", um motor Glacier próprio e ISOLADO do app pr…
- new_registry_token_window.luau — script da JANELA "Novo token do registry", um motor Glacier próprio e ISOLADO do…
- new_service_window.luau — script da JANELA do wizard "Novo serviço", um motor Glacier próprio e ISOLADO do…
- service_export_window.luau — script da JANELA "Exportar serviço", um motor Glacier próprio e ISOLADO do app p…
- state.luau — estado mutável compartilhado entre todos os handlers/*.luau (mesmo interpretador…

### rustploy-gui/views/scripts/fmt/ → gui-scripts.md
- dashboard.luau — builders de lista do dashboard (deployments/projects/services/docker/ingress/mon…
- docker_cleanup.luau — resumo textual da limpeza automática de Docker (Settings → Manutenção).
- git.luau — builders dos provedores/repositórios/branches Git conectados (Gitea/GitHub) — Se…
- jobs.luau — formata Job/JobSummary/JobRun (tarefas one-shot via docker-compose) pra exibição…
- registry.luau — formata repositórios/tags do registry OCI embutido pra exibição na sub-aba Docke…
- service_detail.luau — builders da tela de detalhe de serviço (source, healthcheck, env vars, logs, dep…
- time.luau — timestamps e durações.
- types.luau — tipos compartilhados entre os submódulos fmt_*.luau (as formas dos modelos que c…
- util.luau — busca, codificação de array e mapas de estado (paleta = view.rs).

### rustploy-gui/views/scripts/handlers/ → gui-handlers.md
- bundle.luau — copiar um serviço para outro servidor (lado do app principal): o botão "Exportar…
- bundle_import.luau — passo "Importar" do wizard "Novo serviço": recria aqui um serviço exportado de o…
- connection.luau — ciclo de vida da sessão: init (semeia o contexto), login/logout, configurações d…
- deploy_queue.luau — gerência da fila global de deploys (um por vez) na tela Deploy Engine: cancelar/…
- docker.luau — limpeza de recursos Docker sem uso (imagens, volumes, redes).
- jobs.luau — ações da tela global "Schedules" (sidebar) e da aba "Jobs" do projeto: rodar ago…
- nav.luau — navegação da sidebar/tabs e busca do topbar.
- projects.luau — grade de projetos: criar/editar/remover projeto, variáveis de ambiente de projet…
- registry.luau — sub-aba Docker > Registry: navega repo→tags (fetch sob demanda, tags não vêm no …
- secrets.luau — aba "Secrets" do projeto: criar/sobrescrever e apagar valores cifrados, e o atal…
- services.luau — detalhe do serviço (service.gvb): fetch completo, mutações de spec/env, ciclo de…
- settings.luau — Settings (Web Server) e Settings → Git (provedores Gitea: conectar via OAuth/PAT…
- stream.luau — consumidor do SSE de /api/events: aplica o snapshot periódico (2s) e os eventos …
- wizard.luau — wizard "Novo serviço".

### rustploy-gui/views/scripts/net/ → gui-scripts.md
- api.luau — cliente HTTP/JSON da API do daemon.

### rustploy-gui/views/service/ → gui-views.md
- advanced.gvb
- connection.gvb — Connection (valores copiáveis)
- databases.gvb — Databases (servidor de banco compartilhado entre projetos)
- deployments.gvb — Deployments
- domains.gvb — Domains (editável): lista de rotas HTTP (domínio → porta de container, TLS por r…
- environment.gvb — Environment
- general.gvb — General (source / build, editável)
- general_compose.gvb — Editor do YAML de um serviço Compose (bancos/brokers); o corpo do `if @service_s…
- general_git.gvb — Sub-aba Git do provider: URL/imagem crua; corpo do `if @provider_tab == "git"` d…
- general_gitea.gvb — Sub-aba conta conectada (Gitea/GitHub): picker conta/repo/branch; corpo do `if @…
- general_zip.gvb — Sub-aba Zip do provider: upload local com Dockerfile na raiz; corpo do `if @prov…
- healthcheck.gvb — Healthcheck (editável)
- logs.gvb — Aba Logs do detalhe do serviço (importada por service.gvb).
- migrar.gvb — Migrar este banco para um servidor compartilhado

### rustploy-gui/views/shell/ → gui-views.md
- deployments.gvb — Lista de deployments (view = deployments); importada por shell.gvb.
- project_env.gvb — Aba Variáveis do projeto.
- project_jobs.gvb — Aba Jobs do projeto.
- project_secrets.gvb — Aba Secrets do projeto; a condição de carregamento vai na chamada.
- project_services.gvb — Projeto aberto (view = project_services): cabeçalho/edição, sub-abas e grade de …
- projects.gvb — Lista de projetos (view = projects); importada por shell.gvb.

### rustploy-gui/views/styles/ → gui-views.md
- app.gss — Rustploy — glacier-ui stylesheet.
- theme.json

### rustploy-shared/ → shared.md
- .gitignore
- Cargo.toml — Shared types and protocol definitions for the Rustploy PaaS platform
- build.rs — Gera, em tempo de compilação, o catálogo estático de templates a partir dos blue…

### rustploy-shared/src/ → shared.md
- config.rs — Configuração do daemon (`config.toml`): structs de cada seção com defaults e o s…
- connection.rs — Connection string de banco/broker, montada num lugar só.
- lib.rs — Tipos compartilhados entre daemon e GUI (modelos, protocolo, config, manifest, t…
- manifest.rs — Infra-as-Code: structs do manifesto declarativo (`rustploy.yml`).
- models.rs — Modelos de domínio: projeto, `ServiceSpec` e suas fontes, deployment e estados, …
- protocol.rs — Protocolo da API: `Command` (o que o cliente pede), `Response` e `Event` (o que …
- service_bundle.rs — Pacote de serviço: UM serviço (mais as variáveis do projeto que o usuário escolh…
- wizard.rs — Lógica do wizard "Novo serviço" (Application / Database / Broker / Compose /Temp…

### rustploy-shared/src/templates/ → shared.md
- mod.rs — Catálogo de templates de aplicações (formato Dokploy), lido dos blueprints em `t…

### rustploy-shared/templates/
- logos.txt
- blueprints/ — catálogo de templates de app (formato Dokploy), compilado pelo build.rs do shared (775 arquivos, não indexados)

### tools/
- busy_actions.py

### tools/indexer/
- Cargo.lock
- Cargo.toml — Gera docs/indice/: índice de arquivos e símbolos para busca barata por agentes

### tools/indexer/src/ → tools.md
- commands.rs — `comandos.md`: uma linha por variante de `Command` ligando as três pontas de uma…
- main.rs — Gera `docs/indice/`: um mapa de arquivos e símbolos pensado para um agente achar…
- script.rs — Índice dos arquivos que não são Rust: scripts Luau e JS, templates `.gv`/`.gvb`,…
