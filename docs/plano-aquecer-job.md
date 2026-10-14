# Aquecer job: saber se o cache existe e deixá-lo pronto antes de rodar

> **Status: implementado** (2026-10-14). Pedido do usuário: um botão "aquecer job"
> em cada job, um comando que diga se o job já está quente (fonte única da
> verdade para a tela) e as ações da linha agrupadas num painel sobreposto, porque
> a lista de botões não cabia mais. Vale para a tela global **Schedules** e para a
> aba **Jobs** do projeto, na GUI (glacier) e na webui.
>
> Onde está: `jobs/warm.rs` (decisão, registro e pedido), `docker/compose.rs`
> (`warm`, `resolve_compose_images`), `jobs/source.rs` (clone compartilhado com a
> execução), `api/handlers/job_warm.rs` e `job_warm_status.rs`, `JobWarmStatus` e
> `JobSummary.warm` em `rustploy-shared/src/models.rs`, `components/job_actions.gvb`
> (GUI) e a janela "Ações do job" em `webui/index.html`.

## O problema

Um job roda `docker compose up --build`. Na primeira vez (ou depois de uma
limpeza do Docker) isso baixa as imagens e compila tudo do zero. O custo aparece
no pior momento: no meio de um deploy que espera pelo migrator, ou na execução
agendada de madrugada. Faltavam duas coisas: **ver** se o job está pronto e
**deixá-lo pronto** sem executá-lo.

## "Quente" é uma conclusão do daemon, nunca da tela

`Command::JobWarmStatus { job_id }` responde `JobWarmStatus { warm, warming,
reason, warmed_at }`. O mesmo valor viaja dentro de cada `JobSummary` do
`JobListAll` (por isso entra no snapshot das duas telas sem chamada extra). As
telas só formatam: `quente` / `frio` / `aquecendo…`, com o `reason` no tooltip ou
no painel.

O daemon decide em `jobs::warm::judge`, uma função pura, a partir de um registro
(`WarmRecord`, chave `job_warm:<job_id>` em `daemon_settings`) conferido contra o
estado real do Docker. O job está **frio** quando:

| Motivo | Como é detectado |
|---|---|
| nunca aquecido | não há registro |
| a definição mudou | a impressão digital difere (compose colado, ou repositório + branch + caminho do compose) |
| uma imagem sumiu | `inspect_image` falha para alguma imagem de registry do registro |
| o cache de build foi limpo | há `has_build` e a última limpeza (`build_cache_last_prune`) é posterior ao aquecimento e o corte de idade dela não poupou o cache |

O registro nasce de dois caminhos, porque os dois baixam e compilam tudo: o
aquecimento explícito e **uma execução bem-sucedida** do job (`run_once` devolve
as imagens do compose, resolvidas por `docker compose config --format json`, e o
`runner` grava o registro). Apagar o job apaga o registro.

### O que a verificação NÃO enxerga (limites assumidos)

- **Imagens base de serviços com `build:`** (o `FROM` do Dockerfile). Se a limpeza
  de imagens remover a base, o job continua marcado quente e a próxima execução
  baixa a base de novo. O custo é um `pull`, não a recompilação.
- **Commit novo no mesmo repositório** de um job git-sourced não esfria o job: as
  camadas de dependências continuam valendo e só as do código novo são refeitas.
- **O cache de build é inferido** pela data da última limpeza, não consultado no
  BuildKit. O corte `until` usado na limpeza é, pelo que se sabe do BuildKit, por
  último uso; isso não foi testado de forma destrutiva.

## Aquecer

`Command::JobWarm { job_id }` responde `JobWarmRequested { already_warm }`:

- já quente → nada é disparado e a tela mostra "já está aquecido";
- senão → o job entra em `AppState::warming_jobs`, o pedido espera a **vaga única
  da fila** (`acquire_job_slot`, a mesma de deploys e jobs, com teto de 30 min) e
  o fim chega em `Event::JobWarmChanged { warming, success, error }`.

A execução, em `docker::compose::warm`: escreve o compose do mesmo jeito que o
`run_once` (rede do projeto, `.env`), baixa com `docker pull` só as imagens de
registry ausentes e roda `docker compose build` se algum serviço tem `build:`.
Um segundo pedido enquanto o primeiro corre devolve `AlreadyWarming`.

## As ações da linha viram um painel sobreposto

Cada linha de job tinha seis botões e não cabia. Agora a linha mostra o estado de
aquecimento e um único botão **⋯**, que abre o painel com: Rodar agora (ou
Cancelar execução), Aquecer, Ver logs, Pausar/Ativar, Editar e Remover.

- **webui:** uma janela flutuante do `wm.js` (`x-win="'job_actions'"`), estado em
  `store.jobActionsJobId`. Ela relê a linha do snapshot, então o estado
  quente/frio muda ao vivo com a janela aberta.
- **GUI:** um `<popup>` do glacier dentro do componente `JobActions`, um por
  linha, centrado sobre a janela. A chave de aberto/fechado é por linha
  (`job_actions_open_<job_id>`, interpolada em `value`), e fechar é zerar a chave.
  Descartados: uma janela do SO (o pedido era um painel sobreposto) e o
  `<dialog>`, porque um diálogo com corpo só abre pela ação `dialog:nome`, sem
  como passar o job da linha (o Luau só abre os diálogos de corpo fixo, como
  `confirm{}`). O `<popup>` não é modal: não bloqueia a tela atrás, fecha com
  clique fora ou Esc.

## Testes

- `jobs::warm::tests`: decisão (nunca aquecido, definição alterada, imagem
  ausente, limpeza total, limpeza com corte de idade, limpeza anterior ao
  aquecimento, job sem build) e impressão digital de job git.
- `docker::compose::tests`: separação entre imagens baixadas e serviços com build.
- `rustploy-shared`: JSON de `JobSummary` sem `warm` (daemon antigo) e de
  `DockerCleanupConfig` salvo antes dos campos de retenção.
- GUI `templates_render`: as seis ações no painel, nos dois pontos de entrada, nos
  estados frio e rodando.
