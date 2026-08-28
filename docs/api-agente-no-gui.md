# A API de agente vive no app, não no daemon

> Implementado em 2026-08-27, junto com as correções da Parte 1 de
> `plano-erro-de-deploy-invisivel.md`.

## O que se queria

A ideia original era simples de dizer e não tão simples de posicionar: **usar um
agente, na máquina local, para operar um rustploy que roda longe.** Criar
projeto, subir serviço, disparar deploy, descobrir por que falhou — sem abrir a
GUI para cada passo, e sem virar administrador de JSON.

O primeiro instinto é pôr essa API no daemon. Mas o daemon já tem: `POST
/api/rpc` cobre o protocolo inteiro, e um deploy completo (Supabase em Compose
mais um app com build de Dockerfile) já foi conduzido por agente exatamente
assim. O que faltava não era poder — era **como chegar até lá**.

Para falar com o daemon remoto, um agente precisa da URL pública dele e do
bearer token, na máquina onde o agente roda, fora do lugar onde essas duas
coisas já vivem. Esse lugar é o app. O usuário digitou URL e token na tela de
login, o app validou a conexão, guardou o par e mantém uma sessão viva com SSE e
tudo. Pedir que ele repita isso num arquivo de configuração do agente é
duplicar um segredo à toa, e é o passo em que a coisa toda emperra.

Por isso a API mora aqui, no `rustploy-gui`.

## Como funciona

```
  agente local ──HTTP──> 127.0.0.1:9800 ──HTTPS──> rustploy remoto
                          (o app GUI)               POST /api/rpc
                              │
                              └─ sessão (url + token) lida do contexto da janela
```

O app sobe um servidor HTTP pequeno em loopback. Quando o agente chama uma
rota, o app encaminha para o daemon remoto usando a sessão que a janela já tem.

Três consequências que valem enunciar, porque são o motivo do desenho:

1. **O agente nunca vê o token do daemon.** Ele usa um token local, próprio,
   que morre junto com o processo.
2. **O agente não precisa saber onde o daemon está.** Ele fala com `localhost`.
3. **Trocar de servidor na GUI troca o alvo do agente junto.** Fez logout? A API
   passa a responder 503 dizendo exatamente isso. Não há estado paralelo para
   sair de sincronia.

A ponte não inventa permissão nenhuma: o que ela alcança é o que a janela
alcança.

## Como um agente descobre a API

Um arquivo, gravado no data dir do usuário quando o app sobe:

```
~/.local/share/rustploy/agent-api.json
```

```json
{
  "version": 1,
  "url": "http://127.0.0.1:9800",
  "token": "…64 hex…",
  "pid": 48213,
  "remote_url": "https://rustploy.exemplo.com",
  "connected": true,
  "docs": "GET /agent/schema (Authorization: Bearer <token>)"
}
```

Ler esse arquivo é o passo de descoberta inteiro. Depois dele, `GET
/agent/schema` conta o resto: cada rota, cada campo, a codificação do protocolo
e os comandos mais usados com exemplo pronto.

O arquivo nasce com permissão `0600` — quem lê o token opera o rustploy remoto
inteiro. O token é novo a cada execução, então um handoff velho esquecido no
disco não vale nada.

## As rotas

| Rota | Para quê |
|---|---|
| `GET /agent/health` | a ponte está no ar? a janela está conectada? (sem token) |
| `GET /agent/schema` | o catálogo — comece por aqui |
| `GET /agent/status` | versão/uptime do daemon + fila de deploys |
| `GET /agent/services` | índice achatado projeto→serviço: id, nome, status, origem |
| `GET /agent/deploys` | últimos deploys **com o desfecho já resolvido** |
| `POST /agent/deploys` | dispara um deploy e espera o desfecho |
| `GET /agent/deploys/<id>/logs` | build log paginado por cursor |
| `POST /agent/rpc` | passthrough cru: qualquer `Command` do protocolo |

### A rota que motivou tudo

`POST /agent/deploys` com `wait` responde, numa chamada só, as duas perguntas
que importam:

```bash
curl -s localhost:9800/agent/deploys \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"service":"stand-imob","wait":true}'
```

```json
{
  "deployment_id": "dep_01M11Z99…",
  "service_id": "svc_01H…",
  "state": "Failed",
  "ok": false,
  "error": "docker build error: Cannot locate specified Dockerfile: Dockerfile",
  "started_at": "2026-08-27T13:01:41Z",
  "finished_at": "2026-08-27T13:01:47Z",
  "waited": true,
  "timed_out": false,
  "log_tail": ["==> Iniciando deploy", "…", "==> Erro em [BuildingImage]: …"],
  "log_cursor": 6
}
```

Pelo caminho cru, o mesmo resultado custaria: um `DeployStart`, um laço de
`DeployHistory` filtrando pelo id, um `GetBuildLogs` inteiro, e o conhecimento
prévio de que a causa da falha mora no `states_log` e não no campo `state` — que
é exatamente o conhecimento que ninguém tem na primeira vez.

`ok` é `true` para `Live`, `false` para `Failed`, e `null` quando não houve
desfecho: ainda rodando, parado de propósito (`Stopped`) ou substituído por um
deploy mais novo (`Pruning`). Chamar qualquer um dos dois últimos de falha seria
mentira.

Aceita `"service"` (nome) ou `"service_id"`. Nome de serviço é único por
projeto, não globalmente — com ambiguidade a rota recusa e lista os candidatos,
em vez de escolher sozinha.

### Log com cursor

`GetBuildLogs` do daemon devolve o log inteiro, sempre; acompanhar um build de
1400 linhas em laço significa rebaixar tudo a cada volta. A fatia acontece na
ponte:

```bash
curl -s "localhost:9800/agent/deploys/dep_…/logs?after=120" -H "Authorization: Bearer $TOKEN"
```

```json
{ "total": 1402, "after": 120, "next_after": 620, "has_more": true, "lines": ["…"] }
```

Passe o `next_after` da resposta anterior e você recebe só o que surgiu desde
então. O tráfego caro é o desta ponte para o agente — o salto até o daemon
acontece na mesma sessão que a GUI já mantém.

### Passthrough

`POST /agent/rpc` aceita qualquer `Command`, esteja ele no catálogo ou não. É a
válvula que mantém as rotas de conveniência honestas: elas existem para os
caminhos frequentes, não para virarem a única porta.

```bash
curl -s localhost:9800/agent/rpc -H "Authorization: Bearer $TOKEN" \
  -d '{"DeployHistory":{"service_id":"svc_…","limit":3}}'
```

## Configuração

| Variável | Efeito |
|---|---|
| *(nada)* | liga em `127.0.0.1:9800` |
| `RUSTPLOY_AGENT_API=off` | desliga |
| `RUSTPLOY_AGENT_API=127.0.0.1:9910` | outra porta |

Endereço não-loopback é recusado e cai no default. Se a porta preferida estiver
ocupada, o servidor usa uma efêmera — e é por isso que o agente lê a porta do
handoff em vez de assumi-la.

## Decisões e limites

**hyper dos dois lados, não axum nem reqwest.** O daemon já serve a própria API
com hyper cru; carregar um segundo framework HTTP dentro de um app de desktop
para servir oito rotas em loopback não se paga. O mesmo vale para o cliente.

**Só loopback, e com token mesmo assim.** Loopback não é fronteira de segurança
num desktop multiusuário, e o que está do outro lado da ponte derruba produção.

**Sem escopo.** Quem tem o token do handoff tem o alcance da janela, que é o
bearer do daemon — hoje sem escopo nenhum. Enquanto o daemon não tiver tokens
com escopo (read-only / deploy / admin), esta ponte não tem como inventar um. É
a mesma ressalva registrada na nota de segurança de
`plano-erro-de-deploy-invisivel.md`, e ela ficou mais relevante, não menos.

**Thread e runtime próprios.** O loop do iced é dono da thread principal, e o
executor dele não é lugar de `tokio::spawn` antes de `run()`. Um runtime
`current_thread` numa thread separada custa quase nada e mantém a API viva
independentemente da UI — inclusive com a janela fechada, quando o app fica
recolhido na bandeja e o motor headless segue vivo (glacier-ui 0.48+).

**A sessão é observada, não notificada.** Não há evento de login no glacier para
assinar. O gancho `on_message` do `GlacierDaemon` roda depois de cada dispatch
da janela principal com o motor no estado resultante, então basta reler o
contexto ali. Cobre login, logout e troca de servidor sem conhecer nenhum dos
três fluxos, e a escrita só acontece quando algo de fato mudou (o SSE dispara um
dispatch a cada 2s).

## O que não foi feito

Do levantamento da Parte 2 do plano, ficaram de fora:

- **`GET /api/schema` com JSON Schema de verdade** (item 2.1). O catálogo aqui é
  curado à mão: descreve as rotas desta API por inteiro e os comandos de
  runbook, não as ~90 variantes do enum `Command`. Gerar o schema a partir dos
  tipos exigiria `schemars` sobre a crate `shared` inteira — vale a pena, mas é
  trabalho de outro dia, e o passthrough aceita qualquer comando de qualquer
  jeito.
- **Persistir log de container** (item 2.4). Continua sem tabela; `LogsGet` lê do
  Docker ao vivo, e o swap de deploy destrói o container antigo com o log junto.
  É o item de maior valor e maior esforço do plano.
- **Patch de env var** (item 2.2). `ServiceUpdate` segue sendo substituição
  total; o catálogo avisa disso em `careful`, que é mitigação, não conserto.
- **CLI** (item 2.5). Um agente com esta ponte não precisa dele para o que
  aparece em runbook.
