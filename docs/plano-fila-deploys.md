# Plano: fila global de deploys (um por vez), visível e gerenciável

> **Status: implementado** (2026-07-15). Este doc é o desenho aprovado; abaixo,
> as decisões finais e os desvios em relação ao rascunho:
> - UI da fila mora na tela **Deploy Engine** (painel "NA FILA" acima de
>   "EXECUTANDO AGORA"), não num painel de Deployments.
> - Status novo `ServiceStatus::Queued` ("Na fila") — implementado.
> - O **auto-redeploy do watchdog** também passa a enfileirar (antes spawnava o
>   executor direto), então nada fura a fila.
> - `recovery` re-enfileira os `Pending` no boot; as retomadas de meio-de-caminho
>   (swap/promote/rollback) seguem rodando direto (simplificação conhecida).

## O que se quer

1. Deploys param de rodar concorrentes: passam por uma **fila global**, no
   máximo **um deploy por vez** no daemon inteiro.
2. Essa fila é **visível** e **gerenciável** na tela **Deploy Engine**:
   cancelar um item da fila, reordenar arrastando, mover um item pro topo
   ("furar fila") e pausar/retomar a fila inteira.
3. Enquanto um serviço espera na fila, o status dele aparece como **"Na fila"**
   (distinto de "Deployando").

## Como é hoje

`deploy_start` (`crates/daemon/src/api/handlers/deploy_start.rs`) cria o
deployment no banco, marca o serviço como `Deploying` e **spawna na hora** um
`DeployExecutor` num `tokio::spawn` — ou seja, N pedidos = N deploys rodando ao
mesmo tempo, disputando CPU/rede/Docker. O `AbortHandle` da task vai pro mapa
`state.active_deploys` (`deployment_id → AbortHandle`), usado pelo
`deploy_abort` para cancelar. No boot, `recovery.rs` re-spawna executores para
deployments não-terminais.

Detalhe que ajuda: o estado `DeployState::Pending` já existe e é exatamente
"deployment criado, ainda não começou a rodar". Ele é o encaixe natural de
"item na fila" — não precisamos inventar um estado novo do lado do deployment.

## Desenho proposto

### 1. Engine da fila (novo `crate::deploy::queue`)

Uma `DeployQueue` compartilhada no `AppState`:

```
QueueInner {
    queued: VecDeque<String>,   // deployment_ids esperando, em ordem
    running: Option<String>,    // o deployment_id rodando agora (ou None)
    paused: bool,               // fila pausada?
}
```

mais um `tokio::sync::Notify` para acordar o worker.

Um **único worker** (spawnado no startup) roda o laço:

- espera o `Notify`; se `paused` ou `queued` vazia, volta a esperar;
- senão, tira o da frente (`pop_front`), marca `running = Some(id)`;
- **spawna** o `DeployExecutor::run(id)` como task e guarda o `AbortHandle` em
  `active_deploys` (mantém o mecanismo de abort atual), e dá `await` no join;
- ao terminar (sucesso, falha ou abort), limpa `running`, publica
  `DeployQueueChanged` e volta ao laço.

Como o worker é único e só pega o próximo quando o atual termina, garante-se
**um por vez** de forma natural. O `active_deploys` passa a ter no máximo uma
entrada (o deploy em execução).

Operações de gerência na `DeployQueue`:
- `enqueue(id)` — `push_back` + notify + evento.
- `cancel(id)` — se está na `queued`, remove; se é o `running`, aborta a task.
  Nos dois casos transiciona o deployment `Pending`/atual → `Failed` e o serviço
  para um status de repouso (Stopped) + evento.
- `promote(id)` — move o item pro início da `queued`.
- `reorder(order)` — recebe a nova ordem completa dos ids enfileirados (do
  drag-and-drop) e reescreve a `VecDeque` respeitando-a.
- `set_paused(bool)` — liga/desliga; ao retomar, notifica o worker.
- `snapshot()` — devolve `running` + `queued` (em ordem) + `paused` pro handler
  de status.

### 2. `deploy_start` deixa de spawnar

Passa a: criar o deployment (`Pending`), marcar o serviço como **`Queued`**
(evento `ServiceStatusChanged(Queued)`), **enfileirar** o id na `DeployQueue` e
retornar. O guard atual (`ServiceAlreadyDeploying`) ganha também o caso
`Queued`, pra não enfileirar o mesmo serviço duas vezes.

### 3. Status novo `ServiceStatus::Queued`

Adiciona a variante `Queued` no enum (`crates/shared/src/models.rs`), **no fim**
do enum (o wire postcard é posicional — anexar no fim não desloca as variantes
existentes; e a GUI fala JSON, onde é indiferente). Atualiza o `Display` e o
parse usado pra gravar/ler do banco. A GUI ganha o badge "Na fila".

### 4. Protocolo (tudo anexado no fim de cada enum/struct)

- `DeployEngineSummary` ganha `queued: Vec<ActiveDeployInfo>` e `paused: bool`
  (o handler `deploy_engine_status` monta `queued` a partir da ordem da
  `DeployQueue`, não da ordem do banco).
- Novos `Command`: `DeployQueuePromote { deployment_id }`,
  `DeployQueueReorder { order: Vec<String> }`, `DeployQueuePause { paused }`.
- Cancelar reaproveita o `DeployAbort` existente: ele passa a olhar também a
  fila (se o id está enfileirado, remove; senão, aborta o running como hoje).
- Novo `Event::DeployQueueChanged` (sinal leve; a GUI só refaz o
  `DeployEngineStatus` ao recebê-lo — mesmo padrão dos outros refreshes).

### 5. Recovery (boot)

Deployments em `Pending` (estavam na fila, nunca começaram) deixam de virar
`Failed` e são **re-enfileirados** na ordem de criação — um restart preserva a
fila. Os que caíram no meio de um swap/promote/rollback continuam sendo
retomados como hoje (completam em vez de recomeçar). **Simplificação conhecida:**
essas retomadas de meio-de-caminho rodam direto, fora da fila; na prática é raro
haver mais de uma, então não serializo essas no v1 (dá pra revisitar).

### 6. GUI — tela Deploy Engine

No `home.gv` (`view == deploy_engine`), acima do painel "EXECUTANDO AGORA",
entra um painel **"NA FILA (n)"**:
- um botão **Pausar/Retomar** a fila (mostra o estado `paused`);
- cada linha enfileirada: serviço / projeto / posição, com **cancelar**,
  **↑ topo** (promote) e **alça de arrastar** para reordenar
  (`onReorder`/`reorderKey` — glacier-ui já suporta, ver
  `docs`/memória de drag-and-drop).

Camada Luau (`views/scripts/`): estende o fetch `eng_*` para popular
`eng_queued` e `eng_paused`; handlers `queue_cancel`/`queue_promote`/
`queue_pause`/`queue_reorder` chamando os novos commands; reassina
`DeployQueueChanged` para atualizar ao vivo. Nada de rede nova em Rust — tudo
via os commands acima.

## Arquivos que mudam (se aprovado)

- `crates/daemon/src/deploy/queue.rs` (novo) — a engine da fila + worker.
- `crates/daemon/src/deploy/mod.rs` — expõe o módulo.
- `crates/daemon/src/api/mod.rs` — `AppState` ganha `deploy_queue`; startup
  spawna o worker.
- `crates/daemon/src/api/handlers/deploy_start.rs` — enfileira em vez de spawnar.
- `crates/daemon/src/api/handlers/deploy_abort.rs` — cancela na fila também.
- `crates/daemon/src/api/handlers/deploy_engine_status.rs` — inclui `queued` +
  `paused`.
- `crates/daemon/src/api/handlers/deploy_queue_*.rs` (novos) — promote/reorder/
  pause.
- `crates/daemon/src/api/routes.rs` — dispatch dos novos commands.
- `crates/daemon/src/deploy/recovery.rs` — re-enfileira `Pending`.
- `crates/shared/src/models.rs` — `ServiceStatus::Queued`, campos novos em
  `DeployEngineSummary`.
- `crates/shared/src/protocol.rs` — novos `Command`/`Event`.
- `crates/rustploy-gui/views/home.gv` — painel da fila.
- `crates/rustploy-gui/views/scripts/…` — fetch, handlers, formatação, badge.
- `crates/rustploy-gui/views/styles/app.gss` — estilo do painel/alça.
- Testes: unit da `DeployQueue` (ordem, promote, reorder, pause, cancel),
  `templates_render` do painel, e um teste de `ServiceStatus::Queued`
  round-trip DB/serde.

## Riscos / pontos de atenção

- **Abort do running**: o worker precisa spawnar o executor (não `await` inline)
  pra o `AbortHandle` continuar funcionando como hoje.
- **Duplo-enfileiramento**: guard no `deploy_start` (rejeita se já `Queued`/rodando).
- **Ordenamento**: a ordem "verdadeira" da fila vive na `VecDeque` em memória, não
  no banco. Depois de um restart, ela é reconstruída da ordem de criação dos
  `Pending` (aceitável; a ordem manual de drag não sobrevive a restart no v1).
- **Wire**: todas as adições vão no fim dos enums/structs (postcard posicional);
  a GUI fala JSON, então não quebra.

## Faseamento sugerido

1. **Fase 1 (núcleo):** `DeployQueue` + worker + `deploy_start` enfileira +
   `ServiceStatus::Queued` + recovery re-enfileira. Já entrega "um por vez".
2. **Fase 2 (ver):** `DeployEngineSummary.queued/paused` + painel "NA FILA" só
   leitura + `DeployQueueChanged`.
3. **Fase 3 (gerenciar):** cancelar / promote / reorder / pause + os controles
   na UI.
