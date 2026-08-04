# Pré-deploy gate: rodar um check antes do deploy, e só prosseguir se ele passar

> **Status: implementado** (2026-08-04). Este doc é o desenho aprovado; abaixo,
> os desvios em relação ao rascunho original:
> - **Falha do check não pula `RollingBack`** (diferente do que a seção 2
>   dizia originalmente): o `step()` do estado `PreDeployCheck` simplesmente
>   devolve `Err(...)` no caso de falha, e deixa o loop `execute()` cuidar da
>   transição — exatamente como qualquer outro estado falho. `RollingBack` já
>   é seguro de rodar mesmo sem nada staged (os `find_by_name` de containers
>   de staging simplesmente não encontram nada e viram no-op), então não
>   havia necessidade de um caminho especial "pula direto pro Failed" — isso
>   teria sido a única exceção ao padrão de erro do resto do executor.
> - **Logs do check são espelhados como `Event::BuildLog`** de verdade: o
>   parâmetro `mirror_deployment: Option<(deployment_id, service_id)>` foi
>   adicionado em `docker::compose::run_once`/`run_once_up` (e propagado por
>   um novo `JobRunner::run_inner_mirrored`), então a saída do `docker
>   compose up` do check aparece tanto no histórico do Job quanto na tela de
>   deploy — como a seção 2 já previa, só que implementado via um parâmetro
>   explícito em vez de deixar implícito.
> - Testes cobrem os dois ramos sem Docker (sem job = no-op; job apagado =
>   falha) e o round-trip/back-compat de serde — não o exit-code real (exigiria
>   subir um container de verdade; nenhum outro teste deste crate faz isso).

> **Atualização (2026-08-04):** existem **dois** clientes hoje, não um só —
> o `CLAUDE.md` do projeto ainda descreve `rustploy-gui` (iced/Luau) como
> "o único cliente", mas desde 2026-08-03 o daemon também serve uma
> **web UI própria** (`crates/daemon/webui/`, HTML/JS puro + Alpine.js,
> embutida no binário via `build.rs`) como alternativa ao client iced. Os
> dois falam o mesmo protocolo (`/api/rpc` + `/api/events`), então a lógica
> deste plano (campo no `ServiceSpec`, estado novo no executor) vale para os
> dois sem alteração — só a seção de UI precisa cobrir as duas telas em
> paralelo. Ver detalhes na seção 5 e no faseamento.

## O que se quer

Antes de qualquer deploy — manual (botão na GUI) ou automático (webhook de
push) — rodar um job de verificação. Se ele **passar**, o deploy segue
normalmente. Se ele **falhar**, o deploy **não acontece** (nada é baixado,
buildado ou trocado em produção).

Exemplos de uso: rodar a suíte de testes do repo antes de aceitar o deploy,
checar se uma migration de banco aplica sem erro, validar que um endpoint
de smoke-test do build responde antes de promover.

## Como é hoje (resumo)

- **Deploy manual** e **deploy via webhook** convergem no mesmo lugar:
  `deploy_start::handle` (`crates/daemon/src/api/handlers/deploy_start.rs`).
  O webhook (`crates/daemon/src/api/public_routes.rs`) só valida um token na
  URL e chama esse mesmo handler — não existe um caminho separado para
  "deploy via webhook".
- `deploy_start` cria o registro `Deployment` (estado `Pending`) e o
  **enfileira** (`DeployQueue`, ver `docs/plano-fila-deploys.md`). Um worker
  único tira da fila e instancia o `DeployExecutor`
  (`crates/daemon/src/deploy/executor.rs`), que roda um state machine:
  `Pending → ResolvingDeps → (PullingImage|CloningRepo|BuildingImage|
  ComposingUp) → Staging → HealthcheckPolling → SwappingIn → Draining →
  Promoting → Live`, com `RollingBack → Failed` em qualquer erro no meio.
- **Não existe hoje nenhum "portão" antes do deploy.** O único freio é
  estrutural (não deixa dois deploys do mesmo serviço rodarem ao mesmo
  tempo) — nada checa pré-condições externas.
- A feature **Jobs** (agendamentos one-shot, `crates/daemon/src/jobs/`) já
  resolve exatamente o problema de "rodar algo descartável e decidir
  sucesso/falha pelo resultado": cada `Job` tem um `compose` (um
  `docker-compose.yml` normal) e um `main_service` — o nome do serviço,
  dentro desse compose, cujo **exit code decide o resultado**. A execução
  (`docker::compose::run_once`) sobe o stack com
  `--exit-code-from <main_service>`, espera terminar, sempre desmonta tudo
  no fim (sucesso ou falha), e grava um `JobRun { exit_code, success }`.

Ou seja: a peça que faz "rodar um container efêmero e decidir por exit
code" já existe e já é usada em produção (para os agendamentos). A pergunta
vira: **onde plugar essa peça para que ela rode antes do deploy, e barre o
deploy se falhar?**

## Desenho proposto

### 1. O check é um `Job` já existente, referenciado pelo serviço

Em vez de inventar um segundo motor de execução, o serviço aponta para um
`Job` do mesmo projeto:

```rust
// crates/shared/src/models.rs, em ServiceSpec
pub pre_deploy_job_id: Option<String>,  // #[serde(default)]
```

Isso significa: o operador escreve o check exatamente como já escreve um
Job hoje (compose + `main_service`), na mesma tela de Schedules, com o
mesmo histórico de execuções e logs. O único campo novo é "qual Job (se
algum) é o pré-deploy check deste serviço" — uma escolha numa tela de
config do serviço, não uma feature de execução paralela.

Um `Job` usado como gate normalmente não precisa de `recurrence` (não
precisa rodar sozinho de tempos em tempos), mas nada impede que o mesmo Job
tenha as duas funções — são ortogonais.

### 2. Novo estado no state machine do deploy: `PreDeployCheck`

Inserido logo depois de `Pending`, antes de `ResolvingDeps`:

```
Pending → PreDeployCheck → ResolvingDeps → ...
```

O `step()` desse estado, dentro do `DeployExecutor`:

- **Sem `pre_deploy_job_id` configurado** → transiciona direto para
  `ResolvingDeps`, sem pausa perceptível. Todo serviço que não optar pelo
  recurso continua se comportando exatamente como hoje.
- **Com `pre_deploy_job_id` configurado**:
  1. Busca o `Job`. Se foi apagado nesse meio-tempo, falha imediatamente
     (`Failed`, mensagem "job de pré-deploy não encontrado").
  2. Roda o mesmo caminho que `jobs::runner::run_inner` já usa hoje
     (resolve env vars, `docker::compose::run_once` com
     `--exit-code-from`), reaproveitado como função compartilhada — não
     duplicado.
  3. Cria um `JobRun` normal (aparece no histórico do Job, com os mesmos
     logs) e, além disso, emite os logs como `Event::BuildLog` marcados com
     o `deployment_id` do deploy em curso — assim o operador vê a saída do
     check na própria tela de deploy, não só na aba Schedules.
  4. **Exit code 0** → transiciona para `ResolvingDeps`, deploy continua
     normalmente.
  5. **Exit code != 0** → transiciona direto para `Failed` (não passa por
     `RollingBack`, porque nada foi staged ainda — não há o que desfazer),
     com mensagem "pré-deploy check falhou (exit code N)".

### 3. Por que esse ponto de encaixe e não outro

A alternativa óbvia seria checar *antes* de criar o `Deployment`, dentro do
próprio `deploy_start::handle`. Foi descartada por dois motivos:

- **Deploy manual via GUI é uma chamada RPC que hoje retorna na hora**
  (o deploy real acontece em background, a GUI só recebe "enfileirado").
  Se o check rodasse ali, a chamada ficaria bloqueada pelo tempo do check
  (pode ser minutos), quebrando esse padrão assíncrono e arriscando timeout
  no cliente.
- **Webhook e deploy manual não precisariam de nenhuma mudança própria.**
  Como os dois já convergem em `deploy_start::handle → fila → executor`,
  colocar o gate dentro do executor cobre os dois automaticamente. Colocar
  o gate em `deploy_start` exigiria replicar a lógica assíncrona (ou
  bloquear o handler) nos dois pontos de entrada.

Colocar o gate como um estado do executor também dá de graça: aparece na
timeline do deploy como qualquer outro passo, usa o mesmo mecanismo de
eventos (`DeployStateChanged`) que a GUI já escuta, e falhas seguem o
mesmo padrão visual que qualquer outra falha de deploy (aparece em
vermelho no histórico, com a mensagem de erro).

### 4. Fora de escopo (de propósito)

- **Validação de assinatura do webhook** (tipo `X-Hub-Signature-256` do
  GitHub) — não existe hoje (`docs/webhooks.md` já documenta essa lacuna) e
  é uma questão de autenticação do webhook em si, ortogonal ao gate.
- **Filtro de branch no webhook** — mesma lacuna documentada, também
  ortogonal: o gate roda para qualquer deploy, venha de onde vier.
- **Aprovação manual** (humano clica "aprovar" antes do deploy seguir) —
  isso é um recurso diferente (gate *humano*, não automático). O desenho
  acima é 100% automático: passa ou não passa pelo exit code, sem espera
  por interação. Se quiser aprovação manual no futuro, é outro estado
  (`AwaitingApproval`) com semântica de espera indefinida, não este.
- **Timeout dedicado para o check** — hoje `docker::compose::run_once` não
  tem timeout próprio (roda até o `main_service` sair). Fica como limitação
  conhecida herdada da feature Jobs, não introduzida por este plano.

### 5. Os dois clientes precisam de UI equivalente, não só um

A web UI (`crates/daemon/webui/`) não é um protótipo secundário — é o
mesmo produto, servido de um jeito diferente (o motivo, pelo docstring de
`web_ui.rs`, é contornar o client iced sendo bloqueado pelo Windows
Defender em algumas máquinas). Ela fala exatamente o mesmo `Command`/
`Response`/`Event` do protocolo, então nenhuma mudança de backend deste
plano é específica de um cliente — mas as duas telas de configuração do
serviço e as duas timelines de deploy são implementações **separadas** (uma
em `.gv`+Luau, outra em HTML+JS), então o trabalho de UI se dobra.

Duas coisas achadas na investigação valem a pena registrar aqui:

- **A aba Advanced já existe nos dois clientes** e é o encaixe natural para
  o seletor de `pre_deploy_job_id`: na GUI iced em `service.gv`/
  `services.luau`; na web UI em `crates/daemon/webui/index.html`
  (`initAdvForm()`/`saveAdvanced()`, `screens/service_detail.js:258-282`) —
  hoje essa aba só tem REPLICAS e RUN COMMAND (informativo). O padrão de
  "filtrar Jobs pelo projeto do serviço" já existe na web UI em
  `schedules.js::servicesFiltered`, é só reaproveitar o mesmo filtro.
- **Nenhum dos dois clientes hoje rotula estados de deploy individualmente**
  — os dois colapsam qualquer estado que não seja `Live`/`Stopped`/`Failed`
  num rótulo genérico único ("BUILDING"): `fmt.js::stateLabelKind` na web UI
  e `fmt/util.luau::state_label_color` na GUI iced têm o mesmíssimo
  fallback. Isso quer dizer que o novo `PreDeployCheck` **funcionaria sem
  nenhuma mudança** nos dois (cairia automaticamente no bucket genérico) —
  mas se a intenção é mostrar "Verificando pré-condições" em vez de
  "BUILDING" (como a Fase 2 original já previa para a GUI iced), os dois
  arquivos de rótulo precisam do mesmo tratamento, senão a web UI mostra um
  texto genérico enquanto a GUI iced mostra um específico. Tratar as duas
  juntas evita esse descompasso.
- A web UI **não tem hot-reload** — qualquer mudança em `webui/*` só
  aparece depois de `cargo build -p daemon` (o `build.rs` reprocessa/
  reminifica/gzipa e embute os assets em tempo de compilação).

## Arquivos que mudam (se aprovado)

- `crates/shared/src/models.rs` — `ServiceSpec.pre_deploy_job_id`;
  `DeployState::PreDeployCheck` (nova variante, **no fim do enum** por
  causa do wire postcard posicional).
- `crates/daemon/src/jobs/runner.rs` — extrair a parte de "rodar um Job e
  devolver exit code + logs" de `run_inner` para uma função reaproveitável
  pelo executor (hoje só usada pelo scheduler e por `JobRunNow`).
- `crates/daemon/src/deploy/executor.rs` — novo branch no `step()` para
  `PreDeployCheck`; ajuste no `transition()` para short-circuit quando não
  há job configurado.
- `crates/daemon/src/db/deployments.rs` — nada estrutural muda (estado é só
  mais uma variante do enum já persistido como string/JSON).
- `crates/daemon/src/api/handlers/service_update.rs` (ou onde `ServiceSpec`
  é validado/salvo) — aceitar o novo campo.
- `crates/rustploy-gui/views/service.gv` (aba Advanced, provavelmente) —
  seletor "Pré-deploy check (opcional)" listando os Jobs do projeto.
- `crates/rustploy-gui/views/scripts/handlers/services.luau` — ler/gravar o
  campo novo no formulário.
- `crates/rustploy-gui/views/scripts/fmt/util.luau`
  (`state_label_color`) — rótulo amigável para `PreDeployCheck` (ex.:
  "Verificando pré-condições"), igual aos outros estados já mapeados para
  texto exibível.
- `crates/daemon/webui/index.html` + `crates/daemon/webui/screens/
  service_detail.js` (aba Advanced, `initAdvForm()`/`saveAdvanced()`) —
  mesmo seletor de Job, na web UI.
- `crates/daemon/webui/fmt.js` (`stateLabelKind`) — mesmo rótulo amigável
  para `PreDeployCheck`, para não ficar descompassado da GUI iced.
- Testes: unit do novo branch em `executor.rs` (job ausente, exit 0, exit
  != 0, sem job configurado = no-op), round-trip serde de
  `pre_deploy_job_id` no `ServiceSpec`.

## Riscos / pontos de atenção

- **Job apagado depois de configurado como gate**: tratado como falha clara
  (não devia silenciosamente pular o check).
- **Job usado como gate em vários serviços ao mesmo tempo**: cada deploy
  cria seu próprio `JobRun` (não há singleton por Job), então dois serviços
  gateados pelo mesmo Job rodando em paralelo não colidem — mas competem
  por recursos Docker como qualquer job concorrente hoje.
- **Fila global**: como o check agora ocupa o "slot" de deploy em
  andamento (ver `docs/plano-fila-deploys.md`), um check lento atrasa a
  fila de outros serviços do mesmo jeito que um build lento atrasaria hoje
  — comportamento esperado, não é regressão.
- **Wire**: `DeployState::PreDeployCheck` vai no fim do enum (postcard é
  posicional); `ServiceSpec.pre_deploy_job_id` com `#[serde(default)]` para
  não quebrar specs salvos antes.

## Faseamento sugerido

1. **Fase 1 (núcleo):** campo `pre_deploy_job_id` + estado
   `PreDeployCheck` + lógica no executor + reaproveitamento de
   `jobs::runner`. Sem UI ainda — configurável só via API/manifest.
2. **Fase 2a (GUI iced):** seletor no formulário do serviço (`service.gv`/
   `services.luau`) + rótulo na timeline de deploy (`fmt/util.luau`) + logs
   do check visíveis na tela de deploy.
3. **Fase 2b (web UI):** o mesmo, na aba Advanced de
   `crates/daemon/webui/index.html`/`service_detail.js` + rótulo em
   `fmt.js`. Pode rodar em paralelo à 2a (protocolo já é o mesmo desde a
   Fase 1) ou logo em seguida — mas não deveria ficar pendurada por muito
   tempo depois da 2a, para os dois clientes não divergirem visivelmente.
