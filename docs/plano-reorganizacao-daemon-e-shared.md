# Plano (preliminar): reorganizar o daemon e o shared

> Status: **implementado em 2026-10-18** (fases 0 a 7, com os desvios anotados em "Como ficou" no
> fim). Os números e o desenho das seções 1 a 4 são o **antes** e a proposta original, medidos no código
> daquela data. Nada aqui mudou o que o rustploy *faz*: só **onde o código mora** e **quem pode depender de quem**.

## 1. Por que mexer (o problema, em português simples)

Imagine uma casa onde a cozinha, o banheiro e o quarto não têm porta: tudo entra e sai de tudo. Funciona,
mas qualquer reforma exige andar pela casa inteira e ninguém sabe o que vai quebrar. O daemon está assim em
três pontos.

### 1.1 A raiz do daemon é uma gaveta de miudezas

`rustploy-daemon/crates/daemon/src/` tem **13 arquivos soltos** (2 898 linhas) ao lado dos módulos grandes, e eles
não têm nada em comum entre si:

| Arquivo | Linhas | O que realmente é |
|---|---:|---|
| `migration.rs` | 633 | assistente de migração de banco (feature de produto) |
| `main.rs` | 502 | ponto de entrada **e** toda a montagem do sistema |
| `watchdog.rs` | 378 | vigia dos serviços no ar |
| `shared_db.rs` | 267 | criar database/usuário num servidor de banco compartilhado |
| `env_backup.rs` | 220 | backup periódico de variáveis de ambiente |
| `logs.rs` | 153 | transmitir logs de container para o bus de eventos |
| `metrics.rs` | 151 | métricas de CPU/memória/disco |
| `ports.rs` | 122 | alocar portas externas |
| `env_switch.rs` | 118 | trocar uma variável de conexão (`DATABASE_URL`…) |
| `secrets.rs` | 115 | segredos cifrados |
| `firewall.rs` | 111 | cliente do helper de firewall |
| `health.rs` | 88 | checagens de saúde HTTP/TCP |
| `event_bus.rs` | 40 | barramento de eventos em memória |

São pelo menos cinco assuntos (bancos de dados, observabilidade, backup, rede/exposição, infraestrutura
interna) sem um lugar para morar.

### 1.2 As camadas se enxergam "de baixo para cima"

Num sistema bem arrumado, quem está **embaixo** (falar com o Docker, gravar no banco) não conhece quem está
**em cima** (a API HTTP). Aqui há quebras concretas dessa regra:

| Quem | Importa de quem | Por quê é ruim |
|---|---|---|
| `deploy/queue.rs`, `jobs/{runner,scheduler,warm}.rs`, `maintenance/{scheduler,run}.rs` | `api::AppState` | o motor de deploy, os jobs e a manutenção dependem da **camada HTTP** só para pegar suas ferramentas |
| `jobs/warm.rs`, `maintenance/run.rs` | `api::handlers::docker_prune` | **lógica de negócio** (limpar o Docker) mora dentro de um *handler* HTTP |
| `docker/compose.rs`, `docker/images.rs` | `db::build_logs`, `db::job_log` | o módulo que fala com o motor de containers sabe gravar no banco |
| `db/projects.rs` | `docker::networks::legacy_project_net_for` | a camada de dados chama o módulo do Docker só para calcular um **nome** |
| `db/migration.rs` | `env_switch::EnvChange` | o banco depende de um arquivo solto da raiz |

E o `AppState` (a "mochila" que todo mundo recebe) tem **~20 campos** (banco, docker, ingress, bus, segredos,
TLS, filas, caches, registry…) e vive em `api/mod.rs`. Quem recebe a mochila inteira consegue mexer em
qualquer coisa, então nada diz o que cada pedaço de código realmente precisa.

### 1.3 Arquivos gigantes e handlers que fazem o trabalho do domínio

| Arquivo | Linhas |
|---|---:|
| `deploy/executor.rs` | 2 143 |
| `docker/containers.rs` | 1 321 |
| `api/web_ui.rs` | 1 183 |
| `docker/compose.rs` | 1 176 |
| `registry/http.rs` | 1 093 |
| `api/http_api.rs` | 1 047 |
| `api/handlers/service_import.rs` | 1 003 |
| `api/handlers/manifest_import.rs` | 824 |

E `api/handlers/` tem **71 arquivos planos** (um por comando) mais `routes.rs` com um `match` de ~107 braços.
Dois deles (`service_import`, `manifest_import`, ~1 800 linhas juntos) são *features inteiras* escondidas em handler.

### 1.4 O `shared` guarda coisas demais, e quem usa é só o daemon

O `rustploy-shared` é publicado no crates.io, então **toda mudança nele custa um bump + publish + subir a versão
nos consumidores**. Mesmo assim:

- `models.rs` tem **1 630 linhas** e ~60 tipos de uns dez assuntos (projeto, serviço, fonte Git, bancos
  gerenciados, migração, jobs, deploy, registry, inventário do Docker…).
- `protocol.rs` tem **107 comandos**, **59 respostas** e os eventos, tudo num arquivo (769 linhas).
- `lib.rs` tem **7 funções soltas** de nomes de container/rede/stack, e `models.rs` mais três utilitários
  (`normalize_name`, `resolve_env_vars`, `looks_like_git_url`).
- `wizard.rs` (637), `templates` (694 + os blueprints), `connection.rs` (390), `config.rs` (437),
  `manifest.rs` (1 308) e `service_bundle.rs` (564) são **só do daemon** (a conferir ao implementar).

Quem usa o `shared`, medido: o **daemon** usa quase tudo; o **importer** usa só `models` (`Project`, `Service`…);
a **GUI** declara a dependência no `Cargo.toml`, mas **não há nenhuma referência `shared::` no código nem nos
testes dela** (ela é um cliente HTTP/Luau). Ou seja: na prática o `shared` é o "crate de domínio + protocolo do
daemon", e a dependência da GUI parece sobra (confirmar removendo-a e compilando).

## 2. Princípios (as regras do jogo)

1. **Dependência só desce.** `api` → `features` → (`engine`, `store`, `platform`) → `shared`. Nunca o contrário.
   O compilador garante: se um `use` sobe, é erro.
2. **Handler é fino.** Ler o pedido, chamar a feature, devolver a resposta. Regra de negócio não mora em handler.
3. **Cada coisa recebe só o que usa.** Em vez da mochila inteira, contextos pequenos (`DeployContext`, `JobsContext`…)
   ou traits curtos (ex.: um `LogSink` que o Docker usa sem saber que existe banco).
4. **Um assunto por arquivo, e arquivo que cabe na cabeça** (meta: ~500 linhas; passou disso, é candidato a virar pasta).
5. **Agrupar por assunto de produto**, não por tipo técnico (tudo de "bancos de dados" junto).
6. **Nomes por extenso, em inglês** (a convenção do repo). `docker/` vira `engine/` porque agora é **Docker ou Podman**.
7. **Mover sem mudar comportamento.** Nada de reescrever lógica no mesmo passo que muda de lugar.

## 3. O daemon, depois

```
crates/daemon/src/
├── main.rs                 ~30 linhas: lê argumentos e chama bootstrap::run()
├── bootstrap/              (hoje: o grosso do main.rs, 502 linhas)
│   ├── config.rs           config.toml, diretórios, variáveis de ambiente
│   ├── state.rs            monta o AppState (sai de api/mod.rs)
│   ├── workers.rs          sobe watchdog, métricas, backups, filas…
│   └── listeners.rs        sobe API HTTP, ingress, registry
├── platform/               infraestrutura transversal, SEM regra de negócio
│   ├── event_bus.rs  secrets.rs  health.rs
├── engine/                 (hoje: docker/) o motor de containers, Docker ou Podman
│   ├── mod.rs  cli.rs      Engine, cli(), cliente bollard
│   ├── containers/         naming · query · lifecycle · inspect   (quebra do containers.rs)
│   ├── compose/            up · down · jobs · logs                (quebra do compose.rs)
│   ├── images.rs  networks.rs
├── store/                  (hoje: db/) SQLite, um arquivo por agregado (projects, services, deployments, jobs, registry…)
├── features/               um diretório por assunto de produto
│   ├── deploy/             pipeline em estágios (ver 3.2) + queue + recovery
│   ├── jobs/               runner · scheduler · warm
│   ├── ingress/            proxy · router · tls · external_ports (ex-ports.rs) · firewall (ex-firewall.rs)
│   ├── registry/           registro OCI embutido
│   ├── git_providers/
│   ├── databases/          migration.rs · shared_db.rs · env_switch.rs (saem da raiz)
│   ├── observability/      watchdog.rs · metrics.rs · container_logs.rs (ex-logs.rs)
│   ├── backup/             env_backup.rs
│   ├── maintenance/        limpeza do Docker (a lógica sai de api/handlers/docker_prune.rs)
│   └── iac/                manifesto e pacote de serviço (a lógica sai de service_import/manifest_import)
└── api/                    SÓ adaptadores: traduzem pedido -> feature -> resposta
    ├── http/               axum, SSE, autenticação, rotas públicas (webhooks, OAuth)
    ├── web_ui/             a web UI embutida e seus testes headless
    ├── agent/              MCP e a API de agente
    ├── dispatch.rs         escolhe o handler pelo Command (2 níveis, ver 3.3)
    └── handlers/<assunto>/ projects · services · deploys · jobs · docker · git · databases · iac · registry · secrets · …
```

### 3.1 Onde cada arquivo da raiz vai parar

`main.rs` → `bootstrap/` · `migration.rs`, `shared_db.rs`, `env_switch.rs` → `features/databases/` ·
`watchdog.rs`, `metrics.rs`, `logs.rs` → `features/observability/` · `env_backup.rs` → `features/backup/` ·
`ports.rs`, `firewall.rs` → `features/ingress/` · `secrets.rs`, `health.rs`, `event_bus.rs` → `platform/`.
A raiz fica com `main.rs` e as pastas: nada solto.

### 3.2 O executor de deploy em estágios

`deploy/executor.rs` (2 143 linhas) é uma máquina de estados (`Pending → PreDeployCheck → ResolvingDeps → PullingImage /
BuildingImage → Staging → HealthcheckPolling → Draining → Live`, com `Failed`/rollback, e o ramo Compose
`ComposingUp`). A proposta é **um arquivo por estágio** (`resolve.rs`, `source.rs`, `staging.rs`, `healthcheck.rs`,
`promote.rs`, `drain.rs`, `rollback.rs`, `compose_flow.rs`) e um `state_machine.rs` que só orquestra. Isto
**precisa de leitura cuidadosa do arquivo** antes de fixar os cortes: é o código mais crítico do projeto.

### 3.3 Como cada quebra de camada se resolve

| Hoje | Depois |
|---|---|
| `deploy`/`jobs`/`maintenance` importam `api::AppState` | `AppState` sobe para `bootstrap/state.rs`; num segundo passo cada feature passa a receber um contexto próprio, menor |
| lógica de limpeza dentro de `api/handlers/docker_prune.rs` | vira `features/maintenance/prune.rs`; o handler só chama |
| `docker/` grava `build_logs` no `db` | `engine` recebe um `&dyn LogSink` (trait em `platform`); quem implementa é o `store`. O motor de containers deixa de conhecer o banco |
| `db/projects.rs` chama `docker::networks::legacy_project_net_for` | a função é só um **nome**: vai para `shared::naming` |
| `db/migration.rs` usa `env_switch::EnvChange` | o tipo (só dados) vai para `shared` |
| `routes.rs` com um `match` de 107 braços | `dispatch.rs` em dois níveis: um `match` curto por **seção** do `Command` (que já tem as seções: Projects, Services, Deployments, Jobs, Docker, Git…) delega para o `dispatch` de cada pasta de `handlers/` |

### 3.4 Os handlers por assunto (71 arquivos)

`projects/` (create, delete, update, list, env_set) · `services/` (create, update, delete, get, list, stop, reload,
connection_info, archive_upload, wizard) · `deploys/` (start, abort, rollback, history, delete, recent, a fila,
engine_status, build_logs, reconcile) · `jobs/` (os ~11 `job_*`) · `docker/` (inventory, prune, cleanup, remove,
daemon_settings) · `git/` (providers, repos, branches, oauth) · `databases/` (managed_database, migration,
shared_access) · `iac/` (manifest_*, service_export/import) · `registry/` · `secrets/` · `observability/`
(logs_get, daemon_status, ping) · `webhooks/`. Cada handler passa a ter ~10 linhas; o que hoje é lógica em
`service_import` e `manifest_import` vira `features/iac/`.

## 4. O shared, depois

### Opção A (recomendada primeiro): organizar por dentro, sem quebrar ninguém

Dividir os arquivos grandes em pastas **e reexportar tudo na raiz**, de modo que `shared::ServiceSpec`,
`shared::Command` etc. continuem funcionando para quem já usa. Zero mudança de API: sai como patch (0.4.x).

```
src/
├── lib.rs                  só `mod` e `pub use` (nada de lógica)
├── naming.rs               as 7 funções de nome de lib.rs + normalize_name + legacy_project_net_for
├── domain/                 (hoje: models.rs, 1 630 linhas)
│   ├── project.rs  service.rs (ServiceSpec, ServiceStatus)  source.rs (Git/Compose/Archive/Registry)
│   ├── git.rs  database.rs (gerenciados, migração, acesso compartilhado)  deployment.rs
│   ├── job.rs  registry.rs  docker_inventory.rs  ingress.rs  …   (a lista exata sai da leitura do arquivo)
├── protocol/               (hoje: protocol.rs)
│   ├── command.rs          dividido pelas ~22 seções que o enum já tem
│   ├── response.rs  event.rs  logs.rs
├── manifest/  bundle/      IaC e pacote de serviço
└── config.rs  connection.rs  wizard.rs  templates/   (por ora, onde estão)
```

### Opção B (depois, se valer a pena): tirar do shared o que é só do daemon

`config`, `wizard`, `templates`, `connection`, `manifest` e `service_bundle` (~3 900 linhas, mais os blueprints) só o
daemon usa. Movê-los para o daemon deixaria o `shared` pequeno (domínio + protocolo) e reduziria os publishes. Mas:
**quebra a API pública** do crate (bump maior, 0.5.0); os blueprints já estouraram o limite de 10 MB do crates.io uma
vez (os logos foram para a GUI), e o daemon também é publicado; e algumas dessas structs podem fazer parte do
formato de fio (precisa conferir antes de mover). Por isso é decisão separada, depois da Opção A.

Também: **remover a dependência morta da GUI** (`rustploy-shared` no `Cargo.toml`), depois de confirmar que compila.

## 5. Como fazer sem quebrar nada (ordem e critérios)

Cada fase é um conjunto de commits que compila e passa nos testes **sozinha**, e dá para parar depois de qualquer uma.

| Fase | O que | Esforço | Risco | Status |
|---|---|---|---|---|
| 0 | Rede de segurança: `make lint`, `cargo test` (os 3 `headless_tests` falham por ambiente, ignorar), um deploy de teste na instância isolada | P | baixo | ✅ feita |
| 1 | Mover os 13 arquivos da raiz para `bootstrap/`, `platform/`, `features/*` com `git mv` (preserva histórico) e ajustar os `mod`/`use`; deixar `pub use` provisório nos caminhos antigos | P | baixo | ✅ feita |
| 2 | `AppState` → `bootstrap/state.rs`; `docker_prune` → `features/maintenance`; `LogSink`; `legacy_project_net_for` e `EnvChange` → `shared`. **Zera as dependências que sobem** | M | médio | ✅ feita (parte: ver "Como ficou") |
| 3 | `handlers/` em pastas por assunto + `dispatch.rs` em dois níveis; handlers finos (tirar a lógica de `service_import`/`manifest_import` para `features/iac`) | M | médio | ✅ feita |
| 4 | `docker/` → `engine/` (renomear) e quebrar `containers.rs`/`compose.rs` | M | médio | ✅ feita |
| 5 | Quebrar `deploy/executor.rs` em estágios | G | **alto** (código mais crítico): só com leitura completa e testes de deploy | ✅ feita |
| 6 | `shared` Opção A (dividir `models`/`protocol` com reexports) + patch + publish | M | baixo | ✅ feita |
| 7 | (opcional) `shared` Opção B e/ou extrair crates do workspace (`engine`, `store`) | G | alto | ✅ feita (Opção B; sem novos crates) |

P = pequeno, M = médio, G = grande. **Pronto de cada fase:** `make lint` e `make test` verdes, `make index` regenerado e
commitado (o índice de código reflete os caminhos novos), um deploy de ponta a ponta na instância de teste, e nenhuma
mudança em `docs/` de comportamento (só de caminhos).

### O que NÃO muda (contratos)

- O **JSON do protocolo** (`Command`/`Response`/`Event`): a GUI, a web UI, o MCP e os agentes dependem dele.
  Renomear módulos Rust não altera o JSON; renomear variantes alteraria, e não vamos.
- O **`config.toml`** e o **esquema do banco**.
- Os nomes de comando (`DockerContainers` etc.), mesmo com `docker/` virando `engine/`.

## 6. Riscos e cuidados

- **Conflito com trabalho em paralelo:** mover arquivos gera conflitos em qualquer branch aberta. Fazer fases curtas e
  integrar rápido.
- **O `shared` é publicado:** as fases 2 e 6 mexem nele; cada uma exige bump/publish e subir a versão no daemon e no importer.
- **Caminhos citados em outros lugares:** `AGENTS.md`, `docs/indice/` (gerado), comentários `ver crate::…`, e `include_str!`/
  `build.rs` do `webui/` e dos templates. Rodar `make index` e conferir os links.
- **Nomes em logs e mensagens:** `docker` → `engine` aparece em textos de log e de erro; revisar o que o usuário vê.
- **Tentação de "já que estou aqui":** não reescrever lógica nas mesmas mudanças que movem arquivo. Se achar bug, anota
  no `TODO.md` e corrige à parte.

## 7. Decisões em aberto (para você)

1. **Só módulos ou crates novos?** Começar com módulos (fases 1 a 6) é bem mais barato; separar em crates
   (`rustploy-engine`, `rustploy-store`) dá a garantia do compilador sobre as camadas, mas custa configuração e mais
   publishes. Sugestão: módulos agora, crates só se as camadas voltarem a vazar.
2. **Opção B do `shared`** (tirar o que é só do daemon): vale uma versão 0.5.0 que quebra a API pública? Como só o daemon
   usa essas partes, o impacto prático é pequeno, mas a decisão é sua.
3. **Renomear `docker/` para `engine/`** agora que há Podman? Sugestão: sim, na fase 4.
4. **Prioridade:** se for fazer só uma coisa, a **fase 2** (quebrar as dependências que sobem) é a que mais reduz o
   medo de mexer; a **fase 1** é a mais barata e já tira a bagunça da raiz.
5. **Quando?** Convém fazer depois de estabilizar o backend Podman (testes de migração com dados reais pendentes no
   `TODO.md`), para não misturar duas fontes de risco.

## 8. Como ficou (2026-10-18)

Decisões tomadas: **módulos, não crates novos**; **`docker/` virou `engine/`**; **Opção B do shared** (0.5.0);
sem esperar a estabilização do Podman. Verificação em cada fase: `cargo check --workspace --all-targets`,
`cargo test -p rustploy --bins` (antes: 189 passam e 17 falham por falta do socket do Docker; depois: 220
passam — os 31 a mais são os testes que vieram do shared — e os **mesmos** 17 falham). Depois, `DockerClient::for_tests()` (um arquivo vazio como "socket", que é tudo o que o bollard confere) fez esses 17 passarem sem Docker: 237 verdes e, no fim, `make index`.

- **Fase 1.** `bootstrap/` (`mod.rs` com a sequência de boot, `config.rs`, `workers.rs`, `listeners.rs`; `main.rs`
  ficou com ~20 linhas), `platform/` (event_bus, secrets, health), e `features/` com tudo que era assunto de
  produto — inclusive os diretórios que já existiam (`deploy`, `jobs`, `ingress`, `registry`, `git_providers`,
  `maintenance`). Raiz do daemon: só `main.rs` e as pastas.
- **Fase 2.** Zeradas as dependências que sobem **de `engine`, `store`, `platform` e de `features` para
  `api`**: `AppState` foi para `bootstrap/state.rs`; a lógica de limpeza do Docker foi para
  `features/maintenance/prune.rs`; `deploy_start`/`service_stop` viraram `features/deploy/{start,stop}.rs`;
  `to_view` de banco gerenciado foi para `features/databases/managed_view.rs`; o `engine` grava log por um trait
  (`platform/log_sink.rs`, implementado para o `Db` em `store/log_sink.rs`) e não conhece mais o banco;
  `legacy_project_net_for` e `EnvChange` foram para o `shared`. **Contextos (feito depois, no mesmo dia):** cada
  feature passou a receber um contexto próprio em vez do `AppState` inteiro — `DeployContext`, `JobsContext`,
  `MaintenanceContext`, `WatchdogContext`, `ServicesContext`, `MigrationContext` e `IacContext`, cada um no
  `context.rs` da sua pasta e com os campos que a feature de fato usa. O `AppState` os monta
  (`state.deploy_ctx()`…); `TtlCache` foi para `platform/ttl_cache.rs`, `DockerCache` para
  `engine/inventory_cache.rs` e `ActiveDeploys`/`ActiveJobs` para as suas features. Agora `features/` não importa
  mais `bootstrap` (só dois testes de `features/iac/`, que montam um `AppState`).
- **Fase 3.** `api/handlers/` em 12 pastas por assunto (projects, services, deploys, jobs, docker, git,
  databases, iac, registry, secrets, observability, settings), cada uma com o seu `dispatch`; `api/dispatch.rs`
  (era `routes.rs`) só escolhe a pasta. A lógica de criar/atualizar/remover serviço foi para
  `features/services/` e a de `service_import`/`manifest_*`/`service_export` para `features/iac/`. `http_api.rs` e
  `public_routes.rs` viraram `api/http/`; `web_ui.rs` virou `api/web_ui/` (`mod.rs` com o servidor de estáticos e `headless_tests.rs` com os testes de Chrome).
- **Fase 4.** `docker/` → `engine/`, `db/` → `store/`; `containers.rs` em `containers/{naming,query,lifecycle,inspect}`
  e `compose.rs` em `compose/{up,down,jobs,logs}`.
- **Fase 5.** `deploy/executor.rs` (2 143 linhas) virou `executor/`: `state_machine.rs` (laço + despacho),
  um método `step_<estado>` por estágio (`resolve`, `source`, `staging`, `healthcheck`, `promote`, `drain`,
  `rollback`, `compose_flow`), `mod.rs` com a struct e os auxiliares, e `tests.rs`. Os corpos dos estágios
  foram movidos **sem mudança de lógica** (conferido linha a linha contra o arquivo antigo).
- **Fase 6.** `shared`: `models.rs` → `domain/` (13 arquivos, reexportados na raiz; `shared::models` ficou como alias),
  `protocol.rs` → `protocol/{command,response,event,logs}` (o enum `Command` não dá para dividir em arquivos: Rust não
  permite), funções de nome → `naming.rs`.
- **Fase 7 (Opção B).** `config`, `connection`, `templates` (+ blueprints e o gerador do catálogo), `wizard`,
  `manifest` e `service_bundle` saíram do shared para o daemon (`platform/config.rs`, `platform/connection.rs`,
  `features/templates/`, `features/wizard.rs`, `features/iac/`). **Ficaram no shared** os tipos que trafegam no
  protocolo: `ApplyReport`/`ResourceAction`/`ActionVerb`, `ServiceExportPlan`/`ServiceImportReq`/`ServiceImportReport`
  e `WizardCreateReq` — o que o plano chamava de "pode fazer parte do formato de fio" era verdade. O shared passou
  a **0.5.0** (quebra de API) e a GUI deixou de depender dele (a dependência era morta). O índice (`tools/indexer`),
  os scripts do site (`gen-manifest`, `gen-protocol`, `gen-templates`) e o `AGENTS.md` foram atualizados.
