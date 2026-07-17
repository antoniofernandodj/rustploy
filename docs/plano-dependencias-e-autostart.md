# Dependências entre serviços + auto-restart no boot do daemon

> Plano de implementação (ainda não executado). Investigação feita em 2026-07-11 sobre o estado atual do boot do daemon e do modelo de dados; retomar a partir daqui quando for implementar.
>
> **Atualização (2026-07-13):** o TUI (`crates/client`) foi removido do projeto depois desta
> investigação. Os passos abaixo que tocam `crates/client/src/models.rs`/`events.rs` (a aba
> Advanced do TUI) estão obsoletos — `rustploy-gui` é o único cliente hoje; a exposição de
> `DependsOn` na UI, quando for implementada, vai para os equivalentes em Luau
> (`views/scripts/handlers/services.luau` + `new_service.xml`/`service.xml`), não nos
> arquivos Rust do TUI listados abaixo.

## Contexto

Hoje, se o host reinicia, nenhum serviço volta sozinho. Investigação confirmou (`crates/daemon/src/main.rs`, `deploy/recovery.rs`, `watchdog.rs`, `docker/containers.rs`):

- Containers são criados com `RestartPolicy::NO` (`docker/containers.rs::create_staging`) — o próprio Docker nunca os religa após reboot do host.
- `deploy::recovery::recover()` (chamado uma vez no boot, `main.rs:100`) só retoma **deployments em andamento** que ficaram presos; para serviços já `Live`/`Stopped` ele só restaura rotas de ingress de containers *já vivos* — nunca inicia nada.
- O loop `reconcile()` (a cada 30s) só corrige o banco pra refletir o Docker observado (`Running`→`Stopped` se o container sumiu); nunca faz o caminho inverso.
- `watchdog_loop` só reage a serviços já `status IN (Running, Degraded)` — um serviço que já foi rebaixado a `Stopped` pelo `reconcile` fica invisível pra ele, e a corrida entre os dois loops normalmente resulta nisso mesmo no boot.
- Não existe nenhum campo de "intenção" (`desired_state`) — `status` é puramente observacional e é sobrescrito por qualquer evento.
- Não existe nenhuma noção de dependência/ordem entre serviços do mesmo projeto em lugar nenhum (`ServiceSpec`, schema, executor). O único uso de `depends_on:` no repo é dentro do YAML cru de um `docker-compose.yml` de usuário (feature Schedules/Jobs) — delegado inteiramente ao próprio Docker Compose, não é reutilizável como grafo Rust.

Objetivo: no boot do daemon, todo serviço que estava rodando antes do reboot deve voltar a subir sozinho, respeitando uma ordem declarada de dependências dentro do mesmo projeto (ex.: banco antes da API).

Decisões confirmadas com o usuário:
- Um serviço parado manualmente (via "Parar") **não** deve voltar sozinho — só o que estava de fato rodando volta. Isso exige um novo estado persistido de intenção (`desired_state`), distinto do `status` observado.
- Se uma dependência falha ao subir no boot, os serviços que dependem dela ficam **bloqueados** (não tentam subir), marcados com erro explicando qual dependência falhou.

## Visão geral do design

1. **Novo campo `ServiceSpec.depends_on: Vec<String>`** (IDs de serviços irmãos, mesmo projeto) — declarado pelo usuário via TUI/GUI/manifest IaC.
2. **Novo campo `desired_state`** (Running/Stopped), persistido em coluna própria do banco (não no blob JSON da spec) — grava a intenção do usuário, atualizado por `deploy_start` (→Running) e `service_stop`/`stop_all_managed` (→Stopped). É a fonte de verdade de "isso deveria estar rodando", em vez do `status` observacional atual.
3. **Validação de grafo** (sem ciclos, sem referência cruzada de projeto, sem serviço inexistente) em `ServiceCreate`/`ServiceUpdate`; guarda de exclusão em `ServiceDelete` para não apagar uma dependência em uso.
4. **Novo módulo `deploy::autostart`**, chamado de forma síncrona no boot (`main.rs`, logo após `deploy::recovery::recover(...)` e **antes** de qualquer `tokio::spawn` dos loops de background) — isso elimina de vez a corrida entre `watchdog`/`reconcile` encontrada na investigação, porque o autostart processa e conclui o bring-up de cada serviço antes do primeiro tick do `reconcile` (que dispara quase imediatamente após ser spawnado).
5. Mantém `RestartPolicy::NO` nos containers — decisão deliberada: o rustployd passa a ser a única autoridade responsável por decidir *quando* e *em que ordem* religar containers; deixar o Docker religar sozinho (`unless-stopped`) criaria uma segunda fonte de bring-up correndo em paralelo com a lógica de dependências do rustployd, sem coordenação.

## Modelo de dados

### `crates/shared/src/models.rs`
- `ServiceSpec`: adicionar `#[serde(default)] pub depends_on: Vec<String>` como último campo (mesmo padrão já usado por `domains`/`db_kind`/`host_port`; o `serde(default)` é o que permite ler specs antigas do blob JSON `service.spec` sem migração).
- Novo enum `DesiredState { Running, Stopped }` com `Display`/parse espelhando o padrão existente de `ServiceStatus` (`db/services.rs::parse_status`).
- `Service`: adicionar `pub desired_state: DesiredState` como último campo.
- Novo módulo `crates/shared/src/deps.rs`:
  - `pub fn topo_sort(nodes: &[(String, Vec<String>)]) -> Result<Vec<String>, Vec<String>>` — Kahn's algorithm; `Ok(ordem)` ou `Err(ids_no_ciclo)`. Reutilizado tanto na validação de escrita (create/update) quanto na ordenação de boot (autostart).

### `crates/daemon/src/db/mod.rs`
- Migração incremental (padrão `add_column_if_missing`, já usado para `project.env_comments`): `ALTER TABLE service ADD COLUMN desired_state TEXT NOT NULL DEFAULT 'Running'`.

### `crates/daemon/src/db/services.rs`
- `SELECT_COLS` e `ServiceRow`/`row_to_service` ganham `desired_state`.
- `create()`: `Service{}` literal ganha `desired_state: DesiredState::Running` (default da coluna já cobre o INSERT).
- Novo `pub async fn set_desired_state(db: &Db, id: &str, state: &DesiredState) -> Result<()>`.
- Novo `pub async fn get_desired_running(db: &Db) -> Result<Vec<Service>>` — `WHERE desired_state = 'Running'`, usado pelo autostart no boot (todos os projetos; agrupamento por `project_id` fica em memória, no módulo autostart).
- Novo `pub async fn find_dependents(db: &Db, service_id: &str) -> Result<Vec<Service>>` (ou reaproveitar `list_all` + filtro em memória, mais simples dado o volume esperado) — usado pela guarda de exclusão.

## Validação (create/update/delete)

Em `crates/daemon/src/api/handlers/service_create.rs` e `service_update.rs`, antes de persistir:
1. Buscar todos os serviços do mesmo `project_id` (`db::services::list`).
2. Verificar que todo ID em `depends_on` existe nessa lista (senão `RpResponse::err("InvalidDependency", ...)` — rejeita implicitamente referência cruzada de projeto, já que só se busca dentro do próprio projeto).
3. Rodar `shared::deps::topo_sort` sobre o grafo completo do projeto (serviços existentes + a spec sendo criada/atualizada) — se retornar `Err`, `RpResponse::err("DependencyCycle", ...)`.

Em `crates/daemon/src/api/handlers/service_delete.rs`: antes de `db::services::delete`, checar `find_dependents` — se não vazio, `RpResponse::err("DependencyInUse", "outros serviços dependem deste: ...")`.

## Onde `desired_state` é gravado

- `deploy_start.rs` (`crates/daemon/src/api/handlers/deploy_start.rs:98-101`, junto com o `update_status(..Deploying..)` já existente): também `db::services::set_desired_state(.., &DesiredState::Running)`.
- `service_create.rs`: já nasce `Running` por default da coluna — nenhuma mudança necessária além do que já foi listado.
- `service_stop.rs` (`finish_stop`, `crates/daemon/src/api/handlers/service_stop.rs:143-153`): também `set_desired_state(.., &DesiredState::Stopped)`.
- `docker_inventory.rs::stop_all_managed`: mesmo tratamento (chama `service_stop::handle` por serviço, então herda automaticamente se o `set_desired_state` for colocado dentro de `finish_stop`).

## Módulo novo: `crates/daemon/src/deploy/autostart.rs`

Chamado de `main.rs` assim:
```rust
deploy::recovery::recover(...).await;
deploy::autostart::run(db.clone(), docker.clone(), ingress.clone(), bus.clone(), secrets.clone(), tls.clone(), db_path.clone(), config.deploy.drain_secs).await;
// só depois disso: spawns de watchdog_loop, reconcile loop, etc.
```

Lógica (`run`):
1. `db::services::get_desired_running(&db)` → agrupar por `project_id`.
2. Para cada projeto (pode rodar em paralelo entre projetos distintos, ex. `futures::future::join_all`): `docker::networks::ensure_project_network(&docker.inner, project_id)` primeiro (fecha a lacuna encontrada — hoje a rede só nasce dentro do step `Pending` do `DeployExecutor`).
3. Montar o grafo `(id, depends_on)` só com os serviços candidatos desse projeto e chamar `shared::deps::topo_sort`. Se retornar `Err` (ciclo — não deveria acontecer, já é bloqueado na escrita, mas é defensivo), logar erro e pular o projeto inteiro (não arriscar ordem arbitrária).
4. Percorrer a lista topológica **sequencialmente, aguardando cada um** (dentro de um projeto — a ordem importa; correr serviços independentes em paralelo é uma otimização futura, fora de escopo aqui):
   - Se algum `depends_on` já falhou/bloqueado nesta rodada → marcar este serviço `Error("dependência <nome> falhou ao subir")`, publicar `Event::ServiceStatusChanged`, marcar como bloqueado, continuar pro próximo.
   - Se um `depends_on` aponta pra um serviço que **não está** no conjunto candidato (ou seja, `desired_state = Stopped`) → logar aviso e tratar como satisfeito (não bloquear por uma dependência que o próprio usuário deixou desligada).
   - Buscar container vivo (`docker::containers::find_all_by_service_id` / `find_by_name`, mesmo padrão de `service_stop.rs`/`recovery.rs`):
     - **Achado e rodando**: considerar satisfeito, seguir pro próximo sem esperar healthcheck (mesmo critério já usado por `reconcile`).
     - **Achado mas parado**: `start_container` (bollard, igual `watchdog.rs::try_restart:171-176`), depois `poll` de healthcheck com timeout limitado (reaproveitar `crate::health::check_http`/`check_tcp`, já usados por `watchdog.rs`) — sucesso marca `Running`; falha marca `Error` e bloqueia dependentes.
     - **Não encontrado** (removido/nunca existiu) — inclui todo `ServiceSource::Compose`, que é sempre resolvido via `docker compose up -d --build` e não tem um único container previsível: disparar um deploy completo, mas **aguardando o executor terminar** (diferente do `watchdog::trigger_redeploy`, que dá `tokio::spawn` fire-and-forget) — criar `Deployment` (`db::deployments::create`), `update_status(..Deploying..)`, construir `DeployExecutor` e `executor.run(dep_id).await` diretamente (é aceitável bloquear aqui: o boot já está rodando isso de forma síncrona, antes do daemon aceitar conexões normais). Após retornar, reler o `Service` do banco pra saber se terminou `Live` (sucesso) ou `Error`/`Failed` (bloqueia dependentes).
5. Publicar um evento de progresso simples (`Event::LogLine` no barramento existente, ou reaproveitar o mecanismo de log já usado pelo `DeployExecutor::log_step`) pra que TUI/GUI mostrem o que está acontecendo — sem inventar um novo tipo de evento dedicado, a menos que a UI precise diferenciar "autostart" de "deploy manual" visualmente (deixar como extra opcional, não obrigatório pro MVP).

## Protocolo (`crates/shared/src/protocol.rs`)

Nenhuma mudança obrigatória de `Command`/`Response` — `depends_on` e `desired_state` andam de carona nas variantes já existentes (`Command::ServiceCreate(ServiceSpec)`, `Command::ServiceUpdate{id, spec}`, `Response::Service`/`Services`).

## UI

- **TUI** (`crates/client`): aba Advanced (`AdvancedField`/`AdvancedTabState` em `crates/client/src/models.rs:643-706`, hoje com `Replicas | RunCommand | RunArgs | Save`) — adicionar `DependsOn` como uma lista editável (mesmo padrão de edição de `run_args`, mas os itens vêm de um picker limitado aos outros serviços do mesmo projeto, não texto livre). Salvar em `save_advanced` (`crates/client/src/events.rs:1467-1493`), que já usa `..svc.spec.clone()` — só precisa setar o novo campo.
- **rustploy-gui**: form "Advanced" em `crates/rustploy-gui/views/service.xml:910-951` + handler `adv_save` em `views/scripts/handlers/services.luau:381-389` (via `with_spec`) — mesmo tipo de picker (lista de checkboxes/seleção múltipla dos outros serviços do projeto).
- Mostrar visualmente se um serviço está "Parado (manual)" vs "Parado (aguardando reboot/erro)" — usar o novo `desired_state` retornado em `Response::Service` pra diferenciar na UI (pequeno badge/texto), evitando confundir o usuário. Não é estritamente necessário pro backend funcionar, mas é o principal ganho de UX do campo — incluir no escopo.

## IaC / Manifest (`crates/shared/src/manifest.rs`)

`ServiceManifest` (linha 84) ganha `#[serde(default, skip_serializing_if = "Vec::is_empty")] pub depends_on: Vec<String>` — aqui referenciando **nomes** de serviços irmãos (não IDs, já que manifests são editados por humanos antes da criação); o `skip_serializing_if` mantém o YAML exportado enxuto. `ServiceManifest::to_spec`/`from_spec` (linhas ~300-350) resolvem nome↔ID dentro do escopo do mesmo `apply` (todos os serviços do manifesto sendo aplicados juntos podem se referenciar entre si por nome).

## Arquivos principais a tocar

- `crates/shared/src/models.rs` — `ServiceSpec.depends_on`, `DesiredState`, `Service.desired_state`
- `crates/shared/src/deps.rs` — novo, `topo_sort`
- `crates/shared/src/manifest.rs` — `ServiceManifest.depends_on`
- `crates/daemon/src/db/mod.rs` — migração da coluna
- `crates/daemon/src/db/services.rs` — leitura/escrita de `desired_state`, `get_desired_running`, `find_dependents`
- `crates/daemon/src/api/handlers/service_create.rs`, `service_update.rs` — validação de grafo
- `crates/daemon/src/api/handlers/service_delete.rs` — guarda de exclusão
- `crates/daemon/src/api/handlers/deploy_start.rs`, `service_stop.rs` — gravação de `desired_state`
- `crates/daemon/src/deploy/autostart.rs` — novo, orquestração de boot
- `crates/daemon/src/main.rs` — chamar `autostart::run(...)` de forma síncrona, antes dos `tokio::spawn` de `watchdog_loop`/reconcile
- `crates/client/src/models.rs`, `crates/client/src/events.rs` — UI TUI
- `crates/rustploy-gui/views/service.xml`, `views/scripts/handlers/services.luau` — UI GUI

## Verificação

- `cargo test -p shared` — testes novos de `deps::topo_sort`: ciclo direto (A→B→A), ciclo indireto (A→B→C→A), sem ciclo com ordem parcial, projeto vazio.
- `cargo test -p daemon` — testes de `db::services` (mesmo padrão dos existentes em `db/services.rs`) cobrindo `set_desired_state`/`get_desired_running`; teste de handler rejeitando `depends_on` inválido/cíclico/cross-project.
- `cargo check --workspace` depois de cada etapa de tipo (campo novo em `ServiceSpec`/`Service` obriga tocar todo call-site listado no relatório de exploração: `crates/shared/src/wizard.rs:306`, `crates/shared/src/manifest.rs:301`, `crates/daemon/src/db/services.rs` (teste), `crates/client/src/models.rs:1162,1183,1204,1243`, `crates/importer/src/transform/dokploy.rs:83,133`).
- Ponta a ponta (manual, seguindo a skill `verify`/`run` do projeto): criar projeto com serviço A (sem deps) e serviço B (`depends_on: [A]`), deploy dos dois; `docker stop` nos containers dos dois (simulando containers que não sobreviveram ao reboot) + `sudo systemctl restart rustployd`; observar logs/TUI confirmando que A sobe (e fica `Running`) antes de B começar, e que B só inicia depois do healthcheck de A passar.
- Testar o caso "stop manual": parar B explicitamente (`service_stop`), reiniciar o daemon, confirmar que B **não** volta sozinho (A sim, se estava rodando).
- Testar cascata de falha: forçar A a falhar (ex. healthcheck impossível), reiniciar o daemon, confirmar que B fica bloqueado com `Error("dependência A falhou ao subir")` em vez de tentar subir.
- Testar rejeição de ciclo via `Command::ServiceUpdate` direto (TUI ou `rtk`/chamada HTTP) tentando criar A→B→A.
