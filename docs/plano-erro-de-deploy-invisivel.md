# Erro de deploy invisível: a causa da falha é gravada, mas nunca chega no log que o usuário lê

> **Status: Parte 1 implementada em 2026-08-27** (as três correções + os
> testes de regressão). Da Parte 2, o que foi feito virou a **API de agente
> dentro do app GUI** — ver `api-agente-no-gui.md`, e a seção "O que saiu deste
> plano" no fim deste arquivo. Levantado a partir de um deploy real que falhou
> em 1 segundo sem dizer por quê.

## O caso que expôs o problema

Deploy de um app TanStack Start via `ServiceSource::Git`, num rustploy remoto.
O log completo que o usuário viu, do começo ao fim:

```
13:01:41 ==> Iniciando deploy
13:01:41 --> Clonando repositório: https://github.com/… (main)
13:01:46 --> Clone concluído
13:01:46 --> Build Docker: rp_stand_imob:01M11Z99 (Dockerfile)
13:01:47 ==> Deploy falhou — iniciando rollback
```

A causa real: o `Dockerfile` não estava commitado no `main`. O clone trouxe um
repo sem Dockerfile, o `docker build` respondeu na hora, e o deploy morreu. O
usuário passou um tempo revisando env vars — a única hipótese que o log
permitia — enquanto o Docker já tinha dito exatamente o que estava errado.

O `1 segundo` entre "Build Docker" e "Deploy falhou" é o sintoma: um build de
verdade nesse projeto leva ~2 min. Falha instantânea significa que o build nem
começou. Mas isso é inferência de quem conhece o projeto, não informação que o
log deu.

## Diagnóstico

A mensagem de erro **não é perdida**. Ela é produzida corretamente, capturada
corretamente, e persistida corretamente — em três canais que a tela de log não
lê.

### 1. A mensagem é montada com o texto do Docker

`crates/daemon/src/docker/images.rs:167`

```rust
if let Some(err) = output.error {
    return Err(anyhow!("docker build error: {err}"));
}
```

No caso acima, `err` é `Cannot locate specified Dockerfile: Dockerfile`. Há
mais dois pontos equivalentes logo abaixo (`:171` stream error do bollard,
`:173` erro genérico do stream). Todos produzem string útil.

### 2. O executor captura e grava — em dois canais que não são o log

`crates/daemon/src/deploy/executor.rs:134`

```rust
Err(e) => {
    warn!(
        deployment_id = %deployment_id,
        state = deployment.state.label(),
        error = %e,
        "executor: step falhou, iniciando rollback"
    );
    self.transition(
        deployment_id,
        &deployment.state,
        DeployState::RollingBack,
        Some(e.to_string()),   // <- a causa vai por aqui
    )
    .await?;
}
```

O `e.to_string()` chega em:

- **o log de tracing do daemon** (`warn!`) — journalctl/stderr, não a UI;
- **`StateTransition.message`** (`crates/shared/src/models.rs:570`), persistido
  no `states_log` do deployment em SQLite;
- **`Event::DeployStateChanged { message, .. }`**, publicado no bus e servido
  no SSE.

### 3. O log que a UI mostra é outro canal, e ninguém escreve o erro nele

O painel de log do serviço renderiza a tabela `build_logs`, alimentada
exclusivamente por `log_step()` e `Event::BuildLog`. O `Err(e)` acima nunca
chama `log_step`. A única linha que sobra é a do estado seguinte:

`crates/daemon/src/deploy/executor.rs:827`

```rust
self.log_step(&dep.id, &svc.id, "==> Deploy falhou — iniciando rollback").await;
```

### 4. A GUI ignora o `message` nos dois caminhos

- `states_log` **não aparece em nenhum arquivo** de `crates/rustploy-gui/`.
- `crates/rustploy-gui/views/scripts/handlers/stream.luau:345` trata
  `DeployStateChanged` lendo só `d.service_id` e `d.state`; `d.message` é
  descartado.
- `crates/rustploy-gui/views/scripts/state.luau:23` chega a documentar que "o
  evento `DeployStateChanged` carrega só o `service_id`" — o que é falso: ele
  carrega `message`, sempre carregou.

Ou seja: três produtores corretos, zero consumidores.

## A assimetria que torna isso pior

Dentro do próprio `step[BuildingImage]`, o braço `Archive` já faz a validação
que teria resolvido este caso — com mensagem clara e acionável:

`crates/daemon/src/deploy/executor.rs:399`

```rust
let dockerfile = dst.join(&archive.dockerfile_path);
if !dockerfile.is_file() {
    return Err(anyhow!(
        "Dockerfile não encontrado no zip: {}",
        archive.dockerfile_path
    ));
}
```

O braço `Git`, logo acima, monta `(clone_dir.join(&git.build_context),
git.dockerfile_path.clone())` e entrega direto ao `images::build`, sem checar
nada.

O resultado é que **o rustploy já tem a mensagem certa para exatamente este
erro — e ela só não aparece porque a fonte era git em vez de zip.** E note que,
mesmo se o braço Git tivesse a checagem hoje, ela cairia no mesmo `Err(e)` do
item 2 e continuaria invisível. As duas correções são independentes e ambas
necessárias.

## O que fazer

### Correção 1 — escrever a causa no build log (a que realmente importa)

`crates/daemon/src/deploy/executor.rs:134`, no braço `Err(e)`: chamar
`log_step` com o erro **antes** do `transition`, para a linha aparecer no log
imediatamente acima do `==> Deploy falhou`.

Cuidados:

- O `log_step` precisa do `service_id`. O braço atual só tem `deployment_id` e
  `deployment.state` em mão — conferir o que está no escopo do laço de
  `execute()` (`crates/daemon/src/deploy/executor.rs:121`) e carregar o
  serviço se necessário.
- Prefixar de forma consistente com o resto do log (`==>` para marco, `-->`
  para passo). Sugestão: `==> Erro em [BuildingImage]: {e}`, incluindo o
  `label()` do estado — assim o log diz *em que etapa* quebrou, que hoje
  também se perde.
- Erro de build do Docker pode ser multi-linha; decidir se quebra em várias
  entradas de `build_logs` ou grava uma só com `\n`. O renderizador de log
  (`fmt/service_detail.luau:159`) trata linhas como registros independentes —
  quebrar é provavelmente mais seguro.

Isso sozinho conserta **toda** falha de deploy, não só a de Dockerfile
ausente: healthcheck que não passa, pull negado por credencial, porta ocupada,
compose inválido. Todas hoje terminam no mesmo `==> Deploy falhou` mudo.

### Correção 2 — validar o Dockerfile no braço Git

`crates/daemon/src/deploy/executor.rs:385`, braço `ServiceSource::Git`:
replicar a checagem do braço `Archive` depois do clone, com mensagem análoga —
algo como `Dockerfile não encontrado no repositório: {path} (branch {branch})`.

Vale mencionar o branch na mensagem: o erro mais comum não é "esqueci de criar
o Dockerfile", é "criei mas não commitei/não fiz push", e citar o branch
clonado aponta para isso sem precisar dizer.

Cobre também o caso mais frequente no geral: `dockerfile_path` errado no spec
(ex.: `docker/Dockerfile` vs `Dockerfile`).

### Correção 3 — expor o `message` na GUI

Duas frentes, ambas pequenas:

- `handlers/stream.luau:345`: no desfecho terminal, usar `d.message` no toast e
  na notificação do SO quando `d.state` for `Failed`. Hoje a notificação diz só
  que falhou.
- Corrigir o comentário mentiroso em `state.luau:23`.
- Opcional, mas é o que fecha o buraco de verdade: na tela de detalhe do
  deployment, renderizar as transições de `states_log` que têm `message != nil`.
  Isso dá acesso ao histórico de falhas antigas, que hoje só existem no SQLite
  e no journalctl.

## Como validar

Reproduzir é barato — um serviço `Git` apontando para qualquer repo sem
Dockerfile. Antes da correção, o log termina em `==> Deploy falhou`; depois,
deve trazer a linha com o motivo.

Para conferir que a informação já existe hoje (útil para quem precisa depurar
um deploy que falhou *antes* de o fix entrar):

```bash
curl -s localhost:9797/api/rpc -H 'Authorization: Bearer $TOKEN' \
  -d '{"DeployHistory":{"service_id":"svc_…","limit":1}}' \
  | jq '.Deployments[0].states_log[] | select(.message != null)'
```

A transição `BuildingImage -> RollingBack` traz o texto do Docker.

Vale um teste de regressão em `crates/daemon/src/deploy/executor.rs` (já há
testes de `step()` por lá, ver `:1408` em diante) que force um erro de step e
afirme que uma entrada de `build_logs` com a mensagem foi gravada.

## Fora de escopo

- Rever a UX do estado `Failed` como um todo (hoje o serviço vai para
  `ServiceStatus::Error("deploy failed".into())` — string fixa, mesmo problema
  em outro lugar: `executor.rs`, no braço `RollingBack`). Merece plano próprio,
  mas se a correção 1 for feita, considerar passar a causa real para lá também.
- Retenção/rotação de `build_logs`.

---

# Parte 2 — Logs duráveis no cliente e uma API que um agente consiga dirigir sozinho

> Levantado no mesmo dia, a partir da experiência de conduzir um deploy inteiro
> (Supabase self-hosted em Compose + app com build de Dockerfile) por fora da
> GUI, só via `POST /api/rpc`.

## Ponto de partida honesto: a API já faz quase tudo

Vale registrar antes de listar lacunas, porque muda o tamanho do trabalho.
Todo o deploy citado na Parte 1 foi feito por agente, sem GUI, usando só
`POST /api/rpc`: criar projeto, criar serviço Compose, criar serviço Archive,
subir zip, disparar deploy, ler build logs, inspecionar `states_log`, ler
status. Funcionou.

As rotas HTTP hoje (`crates/daemon/src/api/http_api.rs:250`):

| Rota | O que faz |
|---|---|
| `POST /api/rpc` | um `Command` por requisição — a superfície inteira do protocolo |
| `GET /api/events` | SSE: snapshot completo a cada 2s + eventos do bus |
| `GET /api/health` | liveness |
| `GET /api/services/<id>/logs` | logs do serviço |
| `POST /api/services/<id>/archive` | upload de zip |

Ou seja: **não falta poder, falta ergonomia e durabilidade.** O que segue são
os atritos concretos que apareceram no uso real, em ordem de quanto custaram.

## 2.1 — Descoberta do protocolo (o atrito mais caro)

Não há schema, OpenAPI, nem endpoint de introspecção. Para montar a primeira
chamada foi preciso ler o código-fonte: `crates/shared/src/protocol.rs` para o
enum `Command`, `crates/shared/src/models.rs` para `ServiceSpec`,
`ServiceSource`, `EnvVar`, `Healthcheck`, `ResourceLimits` — e deduzir a
codificação serde (enum externamente tagueado: `{"ProjectCreate":{…}}`,
variante unitária como string nua `"ProjectList"`).

Isso é viável para um agente com o repo em mãos e inviável para um agente
operando um daemon remoto.

**Proposta:** `GET /api/schema` devolvendo o JSON Schema de `Command` e
`Response`. `schemars` sobre os tipos de `shared` gera isso praticamente de
graça e não duplica definição — o schema passa a derivar do mesmo lugar que a
serialização, então não desacopla com o tempo. Bônus: vira a fonte de verdade
para validar payload na borda e devolver erro de forma decente em vez de um
`missing field` de serde.

## 2.2 — `ServiceUpdate` é substituição total, não patch

Para mudar **uma** env var é preciso mandar o `ServiceSpec` inteiro de volta —
incluindo `source`, que no caso Compose carrega os 45 KB do `supabase.yml`.
`env_vars` é substituído, não mesclado.

Para um agente isso é uma armadilha: qualquer campo esquecido no round-trip é
silenciosamente apagado, e um `ServiceGet` → mutação → `ServiceUpdate` mal
feito destrói configuração sem aviso.

**Proposta:** comandos de granularidade fina — `ServiceEnvSet { service_id,
env_vars, merge: bool }` (espelhando o `ProjectEnvSet` que já existe),
`ServiceEnvUnset`, `ServiceSourceSet`. Alternativa mais geral: aceitar patch
parcial em `ServiceUpdate` com campos `Option<…>`, mas isso mexe no wire
format e o caminho dos comandos específicos é mais barato.

## 2.3 — Leitura de log é pull-total, sem cursor

`GetBuildLogs { deployment_id }` devolve o log inteiro, sempre. Acompanhar um
build em andamento exigiu re-buscar tudo em laço e fatiar por índice do que já
tinha sido visto — o log do build do Supabase passou de 1400 linhas, rebaixadas
integralmente a cada poll.

`LogsGet { service_id, tail }` tem `tail`, mas é "últimas N", não "desde X".

**Proposta:** `GetBuildLogs { deployment_id, after_seq: Option<u64>, limit }`,
com a resposta trazendo o `seq` da última linha. Mesmo tratamento para
`LogsGet`. O SSE já resolve o caso de acompanhamento em tempo real, mas exige
manter conexão aberta — para um agente que trabalha em turnos, o cursor é o
padrão certo.

## 2.4 — Logs de container não são persistidos em lugar nenhum

Este é o ponto do "cliente deveria armazenar logs", e é o mais estrutural.

Tabelas persistidas hoje (`crates/daemon/src/db/`): `build_log` e `job_log`.
**Não existe tabela para log de runtime de container.** `LogsGet` e o SSE de
logs leem do Docker ao vivo.

Consequências, todas reais:

- **O swap zero-downtime apaga o histórico.** `Draining` → `Promoting` destrói
  o container antigo; o log dele morre junto. Depurar "o que aconteceu antes
  do último deploy" é impossível.
- **Rotação do Docker é do Docker.** O `json-file` driver com default do
  daemon pode truncar sem que o rustploy saiba.
- **Rollback perde a evidência.** Justamente o caso em que se quer o log é
  aquele em que o container que falhou já foi removido.

**Proposta em duas camadas:**

1. **Daemon:** tabela `container_log` alimentada por um tail contínuo do
   Docker por container ativo, com retenção configurável
   (`[logs] retention_days`, `max_bytes_per_service`) — mesmo idioma do
   `[deploy] image_cache` que já existe. Precisa de cuidado com volume:
   serviço tagarela enche o SQLite. Considerar arquivo por serviço com índice
   no banco, em vez de linha por linha em tabela.
2. **Cliente:** cache local do que já foi baixado, para o histórico sobreviver
   a reinício do daemon e para a GUI abrir log antigo sem round-trip. Aqui o
   `rustploy-gui` já tem I/O de arquivo (`docs/plano-file-io-luau-e-geometria.md`).

Vale decidir explicitamente se a retenção fica no daemon, no cliente, ou nos
dois — a Parte 1 mostra que informação que existe só num canal que ninguém lê
é equivalente a informação perdida.

## 2.5 — Sem CLI

O `rustploy apply -f` / `rustploy export` morreu junto com o TUI e não tem
substituto (README, seção *Infra-as-Code*). Hoje o binário `rustploy` na raiz
é um shell script de uma linha apontando para `target/release/rustploy`, que
não é mais construído.

Para um agente, um CLI costuma ser mais barato que montar JSON à mão: menos
tokens, erro legível, composição com shell. E resolve de graça o problema 2.1
— `rustploy --help` é descoberta.

**Proposta:** CLI fino sobre o mesmo `/api/rpc` (`rustploy service env set`,
`rustploy deploy start`, `rustploy logs -f`, `rustploy apply -f`), sem lógica
própria. Não precisa cobrir tudo de saída; cobrir o que aparece em runbook.

## 2.6 — Superfícies que só a GUI alcança

Levantar antes de assumir paridade: `ManifestApply` / `ManifestImport` /
`ManifestExportAll` existem no protocolo, então IaC é acessível por API. Mas
vale uma varredura de `Command` vs. o que a GUI de fato usa, para achar o
inverso — fluxo que a GUI faz em vários passos e que não tem comando
equivalente (o upload de archive, por exemplo, é rota HTTP separada, não
`Command`; um agente não descobre isso lendo `protocol.rs`).

## Ordem sugerida

1. **2.1 (schema)** — destrava tudo o mais e é o de menor risco.
2. **2.3 (cursor de log)** — barato, e melhora também a GUI.
3. **2.4 (persistir logs)** — o de maior valor e maior esforço; decidir
   retenção antes de escrever código.
4. **2.2 (patch de env)** — evita perda silenciosa de config.
5. **2.5 (CLI)** — depende de 2.1 para não virar manutenção duplicada.

## Nota de segurança

Ampliar a superfície para "um agente controla absolutamente tudo" tem
consequência: `api.token` hoje é um bearer único, sem escopo. Quem tem o token
cria projeto, lê secret decifrada (`resolve_env` devolve texto puro), sobe
container e derruba stack.

Se um agente vai operar isso de forma autônoma, vale um token com escopo
(read-only vs. deploy vs. admin) antes de expandir o alcance, e não depois.
`crates/daemon/src/api/http_api.rs:219` é onde o gate de bearer vive hoje —
é um `if` só, o que também quer dizer que é o ponto único para introduzir
escopo.


---

# O que saiu deste plano

> Registrado em 2026-08-27, depois da implementação.

## Parte 1 — feita por inteiro

**Correção 1 (a que importava).** O braço `Err(e)` de `execute()`
(`crates/daemon/src/deploy/executor.rs`) agora escreve a causa no `build_log`
antes de transicionar para `RollingBack`, então a linha do motivo aparece
imediatamente acima do `==> Deploy falhou`. O formato é
`==> Erro em [BuildingImage]: <causa>`, com as linhas seguintes indentadas —
erro de `docker build` é multi-linha com frequência, e o renderizador do cliente
trata cada registro como uma linha. A quebra vive em `failure_log_lines()`,
função pura e testada.

Como previsto, isso conserta **toda** falha de deploy, não só a de Dockerfile
ausente.

**Correção 2.** A checagem de Dockerfile saiu do braço `Archive` e virou uma só,
depois do `match`, valendo para `Git` e `Archive`. Duas coisas mudaram além do
que o plano pedia:

- O caminho conferido é `context.join(dockerfile_path)`, não a raiz do
  clone/zip. É o que `create_tar_gz` usa como nome dentro do tar — a checagem
  antiga do `Archive` errava quando o `build_context` não era `.`.
- A mensagem é especializada por fonte (`missing_dockerfile_msg`): a de `Git`
  cita o branch, como o plano sugeriu, e ambas citam o `build_context` quando
  ele não é a raiz.

**Correção 3.** `handlers/stream.luau` passou a usar `d.message` no desfecho:
notificação do SO, toast e a linha de status da tela do serviço. O texto passa
por `fmt.short_reason()` (primeira linha não-vazia, aparada, truncada) — o log
completo continua na tela de log. O comentário mentiroso de `state.luau` foi
corrigido, com o registro de que o evento sempre carregou `message`.

**A mesma lacuna na webui.** O plano só olhou a GUI iced, mas o daemon serve um
segundo cliente (`crates/daemon/webui/`), e lá era pior: o `applyBusEvent` não
tratava `DeployStateChanged` de forma alguma — a tela do serviço só via a linha
do deployment virar `Failed` no snapshot de 2s, sem nenhuma pista. Agora trata,
com `shortReason()` em `fmt.js` espelhando o helper Luau.

**Fora de escopo que entrou junto.** O plano sugeriu "considerar" passar a causa
real para o `ServiceStatus::Error("deploy failed")`, e é o que `rollback_cause()`
faz agora: lê a mensagem da transição que entrou em `RollingBack` (que já está
persistida quando o step de rollback roda) e usa a primeira linha, truncada. Os
dois pontos do executor que fixavam a string foram trocados.

**Testes.** `mod failure_log_tests` em `executor.rs`, 10 testes, todos sem
Docker — incluindo um que roda `step[BuildingImage]` de verdade numa fonte `Git`
sem Dockerfile e afirma a mensagem, e outro que confere a causa persistida no
`build_log`.

## Parte 2 — mudou de lugar

O plano supunha que a API para agente cresceria no daemon. Ela nasceu no **app
GUI**, e o motivo é o que o próprio plano registra sem tirar a conclusão: *"não
falta poder, falta ergonomia"*. O poder está no `POST /api/rpc` do daemon; o que
falta é o **caminho até ele** — URL pública e bearer token na máquina do agente,
duplicando um segredo que já vive no app logado.

A ponte (`crates/rustploy-gui/src/agent/`, hyper dos dois lados) empresta a
sessão da janela: o agente fala com `localhost`, nunca vê o token do daemon, e
segue a GUI quando ela troca de servidor. Ver `api-agente-no-gui.md` para as
rotas e o desenho.

Dos itens levantados aqui:

| Item | Situação |
|---|---|
| 2.1 descoberta do protocolo | **parcial** — `GET /agent/schema`, catálogo curado à mão. Sem `schemars` sobre a `shared`. |
| 2.2 patch de env var | **não feito** — o catálogo avisa que `ServiceUpdate` é substituição total. |
| 2.3 cursor de log | **feito na ponte** — `GET /agent/deploys/<id>/logs?after=`. O `GetBuildLogs` do daemon segue sem cursor. |
| 2.4 persistir log de container | **não feito** — continua o item de maior valor e maior esforço. |
| 2.5 CLI | **não feito** — a ponte cobre o que aparecia em runbook. |
| 2.6 superfícies só-GUI | **documentado** — o catálogo registra que o upload de zip é rota HTTP, não `Command`. |

A nota de segurança do plano ficou **mais** relevante, não menos: a ponte tem o
alcance da janela, que é o bearer sem escopo do daemon. Ela não tem como
inventar um escopo que o daemon não tem.
