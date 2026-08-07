# Cancelamento de `job_run` em andamento

> **Status: implementado** (2026-08-05). Motivação: um `Job` (Schedules) que
> roda uma suíte de testes longa (ex.: `pytest` de outro projeto, via
> git-source) não tinha como ser interrompido — só esperar terminar.

## Por que não bastava abortar a task (como `DeployAbort`)

O deploy já tem `Command::DeployAbort` + `AppState::active_deploys`
(`deployment_id → tokio::task::AbortHandle`): aborta a task do executor. Isso
funciona ali porque pull/build de imagem passa pela API HTTP do Docker via
`bollard` — cancelar a future fecha a conexão e o Engine cancela a operação
do lado dele.

Jobs são diferentes: `docker::compose::run_once_up` spawna um **processo de
verdade** (`tokio::process::Command::new("docker").args(["compose", ...,
"up", ...])`). `tokio::process::Child` **não é `kill_on_drop` por padrão** —
abortar a task que o aguarda deixaria esse processo (e os containers que ele
sobe) órfãos, rodando pra sempre em background. Cancelamento de job precisa
matar o processo de verdade, não só a task Rust.

## Desenho

- `AppState::active_jobs: Arc<Mutex<HashMap<String, watch::Sender<bool>>>>`
  — `job_run_id → sender` de um canal `watch<bool>` (valor inicial `false`).
- `jobs::runner::spawn()` (caminho `Command::JobRunNow`/scheduler) cria o
  par `(tx, rx)`, registra `tx` em `active_jobs` ANTES de rodar, remove a
  entrada depois de terminar (sucesso, falha ou cancelamento) — mesmo
  idioma de `active_deploys`.
- `Command::JobRunCancel { job_run_id }` → handler procura o `job_run_id`
  em `active_jobs` e manda `true` no `watch`. `NotFound` se o run já não
  estiver mais registrado (terminou, ou id inválido).
- `docker::compose::run_once`/`run_once_up` recebem
  `cancel_rx: Option<watch::Receiver<bool>>`. Dentro de `run_once_up`, a
  espera pelos readers de stdout/stderr corre contra o sinal
  (`tokio::select!`) em vez de só `tokio::join!` direto — se o cancelamento
  vencer, `Child::start_kill()` mata o processo (os pipes fecham sozinhos,
  os readers terminam) e a função retorna o sentinel `CANCELLED_EXIT_CODE`
  (`-2`) em vez do exit code real.
- `-2` é distinto do `-1` genérico (processo morto por sinal por outro
  motivo/erro de execução) — dá pra UI rotular "Cancelado" separado de
  "Falhou" sem precisar de coluna nova em `job_run` (schema não muda).
- `docker::compose::run_once` sempre chama `down(...)` depois de
  `run_once_up`, sucesso ou falha — isso já cobre desmontar containers/rede
  que o processo morto deixou pra trás, sem lógica extra de limpeza.
- Cancelamento pedido durante um clone Git lento (antes de chegar no
  `run_once`) não interrompe o clone no meio — só é checado DEPOIS dele, e
  aí pula o `run_once` inteiro, retornando `CANCELLED_EXIT_CODE` direto.

## Fora de escopo (de propósito)

- **Jobs rodados como pré-deploy check** (`DeployExecutor` no estado
  `PreDeployCheck`, ver `docs/plano-pre-deploy-gate.md`): chamam
  `run_inner_mirrored` diretamente, sempre com `cancel_rx = None`. Cancelar
  esses hoje significa cancelar o deploy inteiro via `DeployAbort` — mesma
  limitação de "só aborta a task" que este documento descreve, herdada, não
  introduzida aqui. Estender pra esse caminho é trabalho futuro separado.

## Arquivos que mudaram

- `crates/shared/src/protocol.rs` — `Command::JobRunCancel`.
- `crates/daemon/src/api/mod.rs` — `ActiveJobs` + campo em `AppState`.
- `crates/daemon/src/api/handlers/job_run_cancel.rs` — novo handler.
- `crates/daemon/src/api/routes.rs` — nome + dispatch.
- `crates/daemon/src/jobs/runner.rs` — `spawn()` registra/remove;
  `run()`/`run_inner_mirrored` recebem `cancel_rx`; `run_inner` (wrapper sem
  cancelamento, só usado por `run()`) removido por ficar sem chamador.
- `crates/daemon/src/deploy/executor.rs` — call site de
  `run_inner_mirrored` passa `None`.
- `crates/daemon/src/docker/compose.rs` — `CANCELLED_EXIT_CODE`,
  `wait_for_cancel`, `run_once`/`run_once_up` com `cancel_rx`.
