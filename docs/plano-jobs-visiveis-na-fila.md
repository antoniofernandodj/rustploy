# Plano: a execução de um job entra na fila de deploy de verdade

> **Status: implementado** (2026-11-10, v0.1.58: shared 0.25.0). Decisões: aquecimento na fila, cancelar tira só a execução, tudo numa versão. Pedido do dono: "essas execuções de job deveriam
> cair na fila de deploy, como se fossem qualquer outro deploy". Continuação de
> [`plano-jobs-na-fila-de-deploy.md`](plano-jobs-na-fila-de-deploy.md) (a vaga única, de 2026-09-29).

## O que acontece hoje, em termos simples

A fila de deploys tem **uma vaga só**: uma coisa roda por vez. Um job (tela Agendamentos: "Executar agora" ou
agendado) **já respeita essa vaga**, então não roda junto com um deploy. Mas ele **não entra na fila**: ele
espera a vaga por fora, num semáforo, e a única pista disso é uma linha de log, "Aguardando a vez", na própria
tela Agendamentos.

Consequências que o dono vê:

- Na tela **Deploy Engine** (fila, "Executando", histórico) o job **não aparece**: nem esperando, nem rodando.
  Dá a impressão de que o daemon está parado, quando está executando um job.
- Não dá para **furar a fila**, **reordenar** nem **tirar da fila** uma execução de job, como se faz com um deploy.
- A ordem entre um job e um deploy é "quem pediu a vaga primeiro", sem o dono poder mexer.

## O que muda

Um job passa a ser **um item da mesma lista** dos deploys. Na fila aparecem os dois tipos, na mesma ordem de
execução, e tudo o que vale para um deploy enfileirado vale para o job:

| Ação | Deploy hoje | Job depois |
|---|---|---|
| Aparecer na lista da fila e em "Executando" | sim | **sim** (com um selo "job" e o nome) |
| Promover ("furar fila") e reordenar | sim | **sim** |
| Tirar da fila | sim (aborta o deploy) | **sim** (cancela a execução antes de começar) |
| Pausa da fila segura | sim | sim (já valia) |
| Abortar quando já está rodando | sim | já existe (`JobRunCancel`) |

### Desenho (para quem for implementar)

1. **`features/deploy/queue.rs`:** a `VecDeque<String>` vira `VecDeque<QueueItem>` com
   `enum QueueItem { Deploy(String), Job(String /* run_id */) }`. O worker único passa a rodar os dois: para
   `Job`, chama `JobRunner::run` por dentro da vaga que ele já tem. O semáforo (`acquire_job_slot`,
   `try_job_slot_now`) deixa de ser usado por job avulso: a vaga é só do worker.
2. **`features/jobs/runner.rs`:** `spawn` cria o `job_run` (estado "esperando"), **enfileira** e volta. O
   `esperar_vaga` e a linha "Aguardando a vez" saem; a linha vira "Sua vez: iniciando" quando o worker pega.
3. **O gate de pré-deploy não muda:** continua rodando o job **por dentro** do deploy, que já tem a vaga. Entrar
   na fila de novo travaria o deploy esperando a si mesmo (já documentado no plano anterior).
4. **`features/jobs/warm.rs`:** o aquecimento hoje pega `acquire_job_slot`. Decidir se também vira item da fila
   (recomendo que sim, com `QueueItem::Warm`) ou se mantém a vaga por fora (então ele continua invisível).
5. **Reinício do daemon:** os `job_run` que estavam esperando continuam sendo marcados **interrompidos**, não
   reexecutados (decisão do plano anterior). Os deploys `Pending` seguem sendo recolocados na ordem de criação.
6. **Protocolo (`rustploy-shared`, bump minor):** `ActiveDeployInfo` ganha `kind` (`Deploy` | `Job`, com
   `serde(default)` = `Deploy`, compatível com daemon e cliente antigos) e, para job, o `job_id`/nome. `DeployQueuePromote` e
   `DeployQueueReorder` passam a aceitar o id do item (o `run_id` do job não colide com `dep_...`). Atualizar
   `agent/manual.md`.
7. **GUI e web UI** (as duas, sempre): Deploy Engine mostra o selo "job" nos itens; o botão de tirar da fila de um
   job chama `JobRunCancel`. Texto novo entra no `pt-BR.json` e no `en.json`.
8. **Docs** (pt e en): página do Deploy Engine e a de Agendamentos; `docs/release.md`.

## O que pode dar errado

- **Travar a vaga:** se `QueueItem::Job` rodar por dentro do worker e o job disparar um deploy (ou o gate de
  pré-deploy), ele esperaria a si mesmo. Hoje nenhum job faz isso, mas o teste tem que cobrir.
- **Fila pausada com um job rodando:** continua valendo (a pausa só impede o próximo).
- **Job longo bloqueando deploys:** já é assim hoje; a diferença é que passa a ser **visível** e a fila pode ser
  reordenada para o deploy urgente passar na frente.
- **Cancelar o item errado:** `DeployQueueReorder` com ids de tipos misturados precisa de teste.

## Decisões que preciso do dono

1. O **aquecimento** (`warm`) também entra na fila, ou fica por fora?
2. **Tirar da fila** um job cancela só aquela execução (o job continua agendado)? (recomendo que sim)
3. Pode ser tudo numa versão só (shared 0.25.0, daemon, app e web UI), ou prefere primeiro só **mostrar** o job na
   lista (parte visível, sem reordenar) e depois reordenar?

## Verificação (quando for feita)

Testes de `queue.rs` (ordem mista, promover, reordenar, remover, pausa, job que não trava o worker), de
`runner.rs` (job entra na fila e a linha de log muda), do protocolo (`kind` default), `glacier test` e o teste
headless da web UI. Depois, **ver a tela**: um job esperando atrás de um deploy longo, na GUI e na web UI.
