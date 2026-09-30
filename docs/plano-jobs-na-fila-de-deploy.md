# Plano: jobs e deploys na mesma fila (uma coisa por vez)

> **Status: implementado** (2026-09-29), **opção A**. Decisões do usuário: opção A
> (vaga única); a pausa da fila segura também os jobs; um job que esperava quando
> o daemon reiniciou é marcado como **interrompido** (não reexecutado).
> Origem: relato de que um job pode rodar em paralelo com um deploy.
>
> Onde está: `deploy/queue.rs` (a vaga: `acquire_next`, `acquire_job_slot`,
> `try_job_slot_now`), `jobs/runner.rs` (`esperar_vaga`, "aguardando a vez",
> cancelamento na espera), `jobs/scheduler.rs` (não empilha), `jobs/mod.rs`
> (`recover_interrupted`), `db/job_run.rs` (`has_unfinished`, `list_unfinished`).

## O problema, em termos simples

O rustploy tem uma **fila global de deploys**: no máximo um deploy roda por vez.
Foi feita assim para que dois builds/`compose up` não briguem por CPU, memória,
disco e Docker ao mesmo tempo (`crates/daemon/src/deploy/queue.rs`).

Só que essa regra vale **só para deploys**. Um **job** (a tela Schedules: tarefa
de uma vez só, agendada ou por "Executar agora") não passa pela fila: sobe na
hora, numa tarefa própria. Então hoje pode acontecer:

- um deploy rodando **e** um job rodando ao mesmo tempo;
- dois jobs ao mesmo tempo;
- o agendador disparar um job no meio de um deploy que estava em andamento.

É o mesmo tipo de disputa que a fila de deploys existe para evitar.

## O que o código faz hoje (fatos, para o desenho não errar)

1. **Jobs nascem na hora.** `jobs::runner::spawn` cria o `job_run` e faz
   `tokio::spawn` da execução. É chamado pelo agendador (`jobs/scheduler.rs`,
   a cada 30 s) e pelo `JobRunNow`.
2. **O gate de pré-deploy roda o job por dentro do deploy.** O `DeployExecutor`,
   no estado `PreDeployCheck`, chama `run_inner_mirrored` direto. Ele já está
   ocupando a vez do deploy. **Esse caminho não pode entrar na fila de novo**:
   o deploy ficaria esperando o job, e o job esperando o deploy (travamento).
3. **Um job longo se dispara de novo sozinho.** `reschedule` (que empurra o
   próximo horário) só roda **depois** que o job termina. Enquanto ele roda, o
   `next_run_at` continua no passado, e o agendador o vê "vencido" no tick
   seguinte, 30 s depois. Um job que leva mais de 30 s pode, hoje, rodar **em
   paralelo com ele mesmo**. É um defeito que já existe e que uma fila
   agravaria (um item novo por tick).
4. **O boot não trata job interrompido.** Se o daemon reinicia com um job
   rodando, o `job_run` fica sem `finished_at` para sempre (na tela, "rodando").
5. **Cancelar job** usa `active_jobs` (um sinal por execução). Um job que ainda
   espera a vez precisa poder ser cancelado sem ter começado.
6. **A tela Deploy Engine só conhece deployments** (`DeployEngineStatus` lê a
   tabela `deployment`). Jobs não aparecem nela.

## Como resolver: duas opções

### Opção A — uma vaga só, compartilhada (recomendada como primeiro passo)

Uma "vaga única" (um semáforo de 1) guardada no estado do daemon. Quem quer
executar pega a vaga; quem chega depois espera em ordem de chegada.

- O **worker de deploys** pega a vaga antes de marcar o deploy como "rodando" e a
  solta ao terminar.
- **Jobs avulsos** (agendador e `JobRunNow`) pegam a **mesma** vaga antes de
  subir o `compose`. Enquanto esperam, o log do job mostra "aguardando a vez".
- O **gate de pré-deploy não pega a vaga** (já está dentro de um deploy que a
  tem). Isso evita o travamento do item 2.
- **Agendador:** ao disparar um job, empurra o `next_run_at` **na hora** (não
  depois de terminar) e não dispara de novo um job que já está esperando ou
  rodando. Corrige o item 3.
- **Boot:** execuções que ficaram sem fim são marcadas como interrompidas
  (item 4). Bônus pequeno, mesmo esforço.

**Vantagem:** poucas linhas, sem mudar protocolo nem telas, e resolve o problema
real (uma coisa por vez). **Limite:** um job esperando **não aparece** na fila do
Deploy Engine e não dá para reordená-lo; ele só aparece como "rodando" na tela
Schedules, com "aguardando a vez" no log.

### Opção B — jobs como itens da fila de verdade

O job vira um item da mesma fila dos deploys: aparece no Deploy Engine, dá para
promover/reordenar/remover, e a pausa da fila o segura.

Exige: um tipo de item novo no protocolo (`shared`), o worker despachando por
tipo, um estado "na fila" em `job_run` (coluna nova), o `DeployEngineStatus`
devolvendo os dois tipos, reconstrução da fila no boot e as **duas telas** (GUI e
webui) mostrando o item de job. É bem mais trabalho e mais risco.

**Recomendação:** fazer a **A** agora (resolve o problema e o defeito do item 3)
e só fazer a **B** se vocês quiserem ver e ordenar jobs na fila do Deploy Engine.

## O que precisa de uma decisão sua

1. **A ou B?** (recomendo A.)
2. **A pausa da fila deve segurar os jobs também?** Recomendo que sim: "pausar"
   passa a significar "nada novo começa", deploy ou job.
3. **Um job que estava esperando quando o daemon reiniciou:** marcar como
   interrompido (recomendo: o agendador o dispara de novo no horário seguinte) ou
   reexecutar no boot?

## Como validar

- Unitário: a vaga é FIFO; o worker só marca o deploy como rodando depois de
  pegá-la; o gate não pega a vaga (teste com um deploy que tem pré-deploy);
  agendador não redispara job que já espera ou roda; `next_run_at` empurrado ao
  disparar.
- Ponta a ponta com Docker (mesmo esquema dos testes de hoje, daemon isolado e
  `RUSTPLOY_FW_SOCKET` apontado para um socket inexistente): dispara um deploy
  longo e um `JobRunNow` juntos e confere no log/`started_at` que o segundo só
  começou depois do fim do primeiro; um job de mais de 30 s **não** dispara duas
  vezes; um deploy com pré-deploy conclui (sem travar).

## Ordem sugerida

1. Vaga única + worker de deploy usando-a (sem mudar comportamento visível).
2. Jobs avulsos usando a vaga; gate de pré-deploy explicitamente fora.
3. Agendador: empurrar `next_run_at` ao disparar e não redisparar.
4. Boot: marcar execuções sem fim como interrompidas.
5. Testes de ponta a ponta acima.

## Como foi validado (2026-09-29)

12 testes de unidade novos (vaga: exclusão mútua, ordem de chegada, pausa,
pausa durante a espera, item removido enquanto espera; espera do job: vaga
livre, ocupada, cancelamento, pausa; banco e boot) e um teste de ponta a ponta
com Docker de verdade num daemon isolado (`RUSTPLOY_FW_SOCKET` apontado para um
socket inexistente), com jobs e deploys reais:

- **A)** job rodando + `DeployStart`: o serviço fica `Queued` e o deploy só
  começa quando o job termina (a diferença medida foi de ~3 ms).
- **B)** deploy rodando + `JobRunNow`: o log do job mostra "Aguardando a vez" e
  o job só começa depois de o deploy terminar.
- **C)** fila pausada: nem o job nem o deploy começam; ao retomar, rodam um de
  cada vez (intervalos sem sobreposição).
- **D)** `JobRunCancel` enquanto espera: a execução fecha com exit code `-2`,
  `success = false` e "Cancelado antes de começar", sem nunca subir o container.
- **E)** deploy com gate de pré-deploy conclui (`Running`) sem travar: o gate
  roda por dentro do deploy e não pega a vaga de novo.
- **F)** `kill -9` com um job rodando, e reinício: a execução é fechada com exit
  code `-1`, `success = false` e a linha "Execução interrompida…" no log.

**Não coberto por ponta a ponta:** o agendador não empilhar um job que já está
esperando ou rodando. A recorrência mínima é de 1 hora (`IntervalHours`), então
só a peça de banco (`has_unfinished`) tem teste.

## Observações fora do escopo

- **Container de job órfão depois de `kill -9`.** O `docker compose up` de um job
  é um processo filho: se o daemon morre à força, o container do job continua
  rodando sozinho (no teste F ele seguiu vivo e segurando a rede do projeto). O
  boot agora fecha a execução no banco, mas não derruba esse container. Um
  `docker compose down` do projeto `jrun_…` no `recover_interrupted` resolveria;
  fica como pendência.
- **Outras tarefas pesadas continuam fora da vaga:** a limpeza automática do
  Docker (`maintenance/`) e o GC do registry rodam quando o horário vence,
  independentemente de deploy ou job. Se for para valer "só uma coisa por vez"
  também para elas, é o mesmo mecanismo (pegar `acquire_job_slot`).
