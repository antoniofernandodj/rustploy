# Plano: chave de acesso única + API de agente no daemon (via MCP)

> **Status: implementado** (2026-10-13; formato do token revisto em 2026-10-14). Decisões aprovadas: token devolvido =
> o próprio `[api].token` (opção A), MCP **e** manual HTTP lado a lado, rotas de
> janela removidas. Desvios do plano durante a execução estão na seção 10.

## 1. O problema, em linguagem simples

Hoje, para entrar no rustploy, a pessoa precisa de **duas informações**:

- **onde** o servidor está (`https://rustploy.exemplo.com:9797`);
- **qual** é o token (`a3f9…`, o `[api].token` do `config.toml`).

Na GUI isso vira dois campos. Na webui já é um campo só, porque a página é
servida pelo próprio daemon e o "onde" é a própria origem — mas o que se cola
ali é o token cru.

E para um **agente de IA** a situação é pior: ele não fala com o daemon. Fala
com uma ponte HTTP que roda **dentro do app GUI** (`rustploy-gui/src/agent/`,
~3 mil linhas), em `127.0.0.1:9800`, que empresta a sessão da janela. Ou seja:
sem o app desktop aberto naquela máquina, nenhum agente opera o rustploy. Um
agente rodando num servidor, num CI ou numa outra máquina não tem como entrar.

O que você pediu resolve as duas coisas de uma vez:

1. **Uma chave só**, que carrega dentro dela o "onde" e o "qual".
2. **O daemon passa a ser a porta do agente**, sem intermediário. A ponte da
   GUI some.

## 2. A chave de acesso

### Como ela é (o "token de conexão")

```
cnVzdHBsb3kuZXhlbXBsby5jb206OTc5Nzo6YTNmOS4uLg==
```

É o base64 **padrão, com padding**, de `host:porta::token` — no exemplo,
`rustploy.exemplo.com:9797::a3f9...`. Exatamente o que sai de
`base64.b64encode(b"host:porta::token")` no Python.

- `::` separa o endereço do token. O endereço já tem um `:` (o da porta); `::`
  não aparece em `host:porta`. O único endereço com `::` é IPv6, que vem entre
  colchetes (`[::1]:9797`), então a busca pelo separador começa depois do `]`.
  O token pode ter `::` à vontade: vale o primeiro separador.
- **Sem esquema.** Quem conecta decide: a GUI sonda `GET /api/version` por
  https e depois por http (o esquema de um servidor já salvo com o mesmo
  endereço vai primeiro); a webui é servida pelo próprio daemon e usa a
  origem; o `/api/login` usa o do listener (https quando há `[api].domain`).
- **A porta sempre vai**, mesmo a padrão (`h:443`).
- Os decodificadores aceitam também o alfabeto url-safe e a falta de padding.
- **O daemon deixa o token pronto no `config.toml`**: a cada boot ele grava
  `connection_token = "…"` logo abaixo do `token` de `[api]` (host =
  `[api].domain`, ou o IP de saída). O daemon não lê esse campo — o serde
  ignora chave desconhecida, então o parse tudo-ou-nada continua valendo. Não
  grava quando o token veio de `RUSTPLOY_API_TOKEN`.
- **GUI e webui não passam pelo `/api/login`**: decodificam o token de conexão
  e usam o token da API direto como Bearer. O `/api/login` fica para agentes.

> Versões anteriores, substituídas no mesmo dia (ver seção 10): `rpk1_` +
> base64url de `{"u":"https://host:porta","t":"token"}`; depois base64url sem
> padding de `host:porta::token` com o esquema por extenso quando a porta não
> o deduzia.

### O que ela **não** é

Ela **esconde** o endereço do olhar, como você pediu — mas **não criptografa**.
Qualquer um que tenha a chave consegue decodificá-la. Isso é aceitável porque
quem tem a chave já tem o token, que é o segredo de verdade; o endereço nunca
foi segredo. Deixo isso escrito para ninguém tratar a chave como "mais segura"
que o token: ela é exatamente tão sensível quanto ele.

### De onde a pessoa tira a chave

Hoje o token sai do `config.toml`. A chave precisa de uma origem equivalente:

- **No log de boot do daemon**: só a dica `rode rustployd access-key` (ver seção 10).
- **Um subcomando**: `rustployd access-key` lê o `config.toml` e imprime a
  chave. **Por quê:** é o jeito óbvio para quem está com SSH na VPS, sem cavar
  log.
- **Nas duas UIs, já logado**: um botão "copiar chave de acesso" em
  Configurações (vindo de uma rota autenticada `GET /api/access-key`).
  **Por quê:** é como alguém passa o acesso para um agente ou um colega sem
  abrir terminal.

O endereço dentro da chave sai do mesmo `public_base_url()` que o daemon já usa
para montar a URL de webhook (domínio da API, ou IP de saída + porta).

## 3. O endpoint de login

```
POST https://servidor:porta/api/login
{"key": "<chave>"}
```

(aceita também o corpo como texto puro, só a chave — o mínimo que um agente
precisa acertar.)

### Se o formato estiver errado → `400` explicando o certo

```json
{
  "error": {
    "code": "missing_separator",
    "message": "a chave decodificada não tem o formato host:porta::token — cole a chave de acesso inteira, não só o token",
    "expected": {
      "method": "POST",
      "path": "/api/login",
      "body": {"key": "<base64 de host:porta::token>"},
      "key_format": "base64 (url-safe sem padding, ou o padrão) de `host:porta::token`; …",
      "example": "curl -s https://host:porta/api/login -d '{\"key\":\"<chave>\"}'",
      "where_to_get_one": "rustployd access-key, na VPS; ou Configurações → Copiar chave de acesso"
    }
  }
}
```

Cada erro diz **qual** parte falhou (base64, separador `::`, endereço ou token vazio),
porque é isso que deixa um agente se corrigir sozinho na tentativa seguinte.

### Token certo → `200` com o token e o caminho do agente

```json
{
  "access_token": "a3f9…",
  "token_type": "Bearer",
  "server": "https://rustploy.exemplo.com:9797",
  "next": "Guarde access_token e mande 'Authorization: Bearer <access_token>' em toda chamada.",
  "ai_agent": {
    "if_you_are": "um agente de IA",
    "mcp": "https://rustploy.exemplo.com:9797/mcp",
    "manual": "https://rustploy.exemplo.com:9797/api/agent/manual",
    "instructions": "Leia o manual (GET, com o mesmo Bearer) ou conecte no MCP. Tudo o que um agente pode fazer está lá."
  }
}
```

### Token errado → `401` (sem dizer qual parte do token errou, nunca o eco dele)

### Uma decisão a tomar: qual é o "token de acesso" devolvido?

| Opção | Como funciona | Prós | Contras |
|---|---|---|---|
| **A (recomendada)** | O login devolve o **próprio token** de dentro da chave, depois de validá-lo. | Zero estado novo no daemon; nada de expiração, revogação, tabela de sessão. Funciona com tudo que existe hoje. | O "login" é uma validação + desembrulho, não emite nada novo. |
| B | O login **emite** um token de sessão novo, com validade. | Revogar uma sessão sem trocar o token-mestre. | Precisa de tabela de sessões, expiração, renovação, tela de revogar… é uma feature inteira. |

Recomendo **A agora**: o daemon só conhece um token, sem escopo nenhum (está
na Parte 1 do AGENTS.md), e inventar sessão por cima não dá nenhuma garantia
nova enquanto o token-mestre continuar valendo para tudo. Se um dia vierem
tokens com escopo (somente-leitura, deploy, admin), o `/api/login` já é o lugar
certo para emiti-los — o formato da resposta não muda.

## 4. As duas telas de login

### GUI (glacier)

- **Um campo**: "CHAVE DE ACESSO". Some o campo de URL.
- Os **servidores lembrados** viram um `<select>` acima do campo, com o
  `host:porta` de cada um:

  ```
  rustploy.exemplo.com:9797
  10.0.0.5:9797
  ```

  Escolher um preenche o campo com o token de conexão dele, e sob o campo
  aparece `→ conecta em host:porta` (decodificado do que está no campo, então
  vale também para um token colado).
- Por baixo: a GUI decodifica a chave para saber **onde** mandar o
  `POST /api/login`, e usa o `access_token` devolvido nas chamadas seguintes
  (o resto do app não muda: continua guardando `url` + `token` da sessão).
- **Servidores já lembrados não se perdem**: o armazenamento continua sendo
  `{url, token}`; a chave é **montada** a partir deles na hora de mostrar.
  Nada de migração.
- Daemon antigo, sem `/api/login` (404): a GUI usa o token decodificado direto,
  como hoje. **Por quê:** atualizar a GUI antes do daemon não pode trancar
  ninguém para fora.

### webui

- Continua **um campo**, agora rotulado "chave de acesso".
- O endereço dentro da chave é ignorado para conexão (a página já está no
  servidor certo); se ele não bater com a origem, aparece só um aviso
  ("esta chave aponta para outro servidor"), não um bloqueio — atrás de proxy o
  host visto pelo navegador pode diferir do da chave.
- Sem dropdown, porque a webui só fala com um servidor.

## 5. A API de agente sai da GUI e vai para o daemon — via MCP

### Por que MCP faz sentido aqui

MCP (Model Context Protocol) é o protocolo que clientes de IA (Claude Code,
Claude Desktop, Cursor…) já sabem falar. Com ele:

- o **manual vira parte do protocolo**: o servidor MCP manda `instructions`
  no handshake, e cada ferramenta vem com descrição e esquema de parâmetros.
  O agente não precisa ler um documento e montar `curl` — ele vê as
  ferramentas `deploy`, `list_services`… prontas, com o que cada uma aceita;
- o cliente adiciona uma vez e esquece:
  `claude mcp add --transport http rustploy https://host:porta/mcp --header "Authorization: Bearer <token>"`.

O transporte é o **Streamable HTTP** do MCP: um `POST /mcp` com JSON-RPC, que
pode responder JSON simples (sem SSE). Isso é importante por dois motivos:

1. dá para implementar à mão em **hyper**, no mesmo listener da API, sem crate
   novo nem axum (a regra do projeto);
2. **um agente que só tem `curl` também consegue usar** — é só um POST com
   JSON-RPC. O manual mostra como.

### Mas não só MCP: o manual em HTTP fica junto

Um agente no meio de uma conversa (como eu, agora) normalmente **não consegue
adicionar um servidor MCP sem reiniciar a sessão**. Se a única porta fosse o
MCP, o fluxo "faz login com curl e sai operando" que você descreveu quebraria.

Por isso a proposta é: **uma fonte só de texto do manual**, servida de dois
jeitos:

- `GET /api/agent/manual` (Bearer) → markdown, para o agente de `curl`;
- o mesmo texto como `instructions` + recurso `rustploy://manual` do MCP.

O manual explica as duas portas: "se você suporta MCP, conecte em `/mcp`; se
não, as mesmas ferramentas estão em `POST /mcp` via JSON-RPC, exemplo abaixo".

### As ferramentas (portadas da ponte da GUI)

| Ferramenta MCP | Equivale a hoje | Faz |
|---|---|---|
| `status` | `GET /agent/status` | versão/uptime + fila de deploys |
| `list_services` | `GET /agent/services` | índice projeto→serviço, com id/status/origem |
| `list_deploys` | `GET /agent/deploys` | últimos deploys com `ok`/`error` já resolvidos |
| `deploy` | `POST /agent/deploys` | dispara e **espera** o desfecho (`wait`, `timeout_s`, `log_tail`) |
| `build_logs` | `GET /agent/deploys/<id>/logs` | log de build paginado por cursor |
| `ingress_routes` | `GET /agent/ingress` | tabela de rotas viva (diagnóstico de 502) |
| `ingress_reconcile` | `POST /agent/ingress/reconcile` | recalcula as rotas sem redeploy |
| `rpc` | `POST /agent/rpc` | **qualquer** `Command` do protocolo — a válvula de escape |
| `manual` | `GET /agent/schema` | o manual, para cliente que ignora `instructions` |

**Melhora em relação a hoje:** a ponte chamava o daemon por HTTP. No daemon,
as ferramentas chamam o `dispatch()` direto, em processo — sem salto de rede,
sem `Snapshot` serializado em string e decodificado de novo.

**Upload de zip** não vira ferramenta: MCP não é feito para dezenas de MB em
base64. O manual documenta a rota que **já existe** no daemon,
`POST /api/services/<id>/archive` com o corpo binário.

### O que se perde (fora de escopo, de propósito)

As rotas de **janela** — `/agent/ui`, `/agent/ui/action`, `/agent/ui/context`,
`/agent/connect`, `/agent/servers` — controlavam a GUI desktop em si (clicar
botão, trocar aba). O daemon não tem janela; essas rotas não têm para onde ir.
Na prática elas existiam porque o agente **dependia** da janela; com o acesso
direto ao daemon, tudo o que a janela faz com dados o `rpc` também faz.
Quem queria "ver o agente clicando" perde isso. Se quiser manter, seria uma
decisão à parte — mas não dá para manter a ponte e removê-la ao mesmo tempo.

## 6. Remoção na GUI

- Apagar `rustploy-gui/src/agent/` inteiro e o `mod agent` / ganchos no
  `app/` (spawn, `on_message`, cleanup do handoff).
- Tirar do `Cargo.toml` da GUI o que só a ponte usava (`hyper`, `hyper-util`,
  `hyper-rustls`, `http-body-util`, `bytes`, `rand` — conferindo um a um se
  nada mais usa), e o comentário "0.58.6+ é REQUISITO da API de agente".
- O arquivo `~/.local/share/rustploy/agent-api.json` deixa de ser escrito.

Aqui vou **apagar**, não comentar, porque você pediu a remoção explicitamente
(a regra de "comentar em vez de remover" é para descarte por decisão minha).
O código fica no histórico do git.

## 7. Documentação

- **AGENTS.md, Parte 1** reescrita do zero: "pegue a chave → `POST /api/login`
  → MCP ou manual". Some tudo de handoff, porta 9800, `RUSTPLOY_AGENT_API`.
- O texto do manual servido pelo daemon vira a fonte; a Parte 1 do AGENTS.md
  aponta para ele em vez de duplicá-lo (o mesmo erro que o CLAUDE.md já
  cometeu uma vez).
- `docs/api-agente-no-gui.md` ganha cabeçalho "substituído por este plano".
- `make index` no fim.

## 8. Ordem de execução

1. **shared**: tipo da chave (`AccessKey::encode/decode`, com os erros
   detalhados) — usado pelo daemon. A GUI decodifica em Luau (base64 em Luau
   puro ou exposto pelo Rust da GUI; decido ao ver o que o glacier já tem).
2. **daemon**: `/api/login`, `/api/access-key`, `rustployd access-key`, linha
   no log de boot, `/mcp`, `/api/agent/manual`; testes de cada erro de formato
   e do handshake MCP.
3. **webui**: rótulo, decodificação, aviso de origem, botão de copiar.
4. **GUI**: campo único, dropdown com dica, chamada ao `/api/login`, botão de
   copiar; remoção de `src/agent/`.
5. **docs** + índice; smoke real: daemon local + `curl /api/login` +
   `claude mcp add` + GUI e webui logando com a chave.

## 9. Perguntas em aberto

1. Token devolvido: **A** (o próprio token) ou **B** (sessão emitida)?
2. MCP **e** manual HTTP lado a lado (recomendado), ou só MCP?
3. Tudo bem perder o controle da janela da GUI pelo agente?

## 10. O que mudou na execução

- **A chave não vai para o log de boot.** Ela é o token, e o journal costuma
  ser legível por mais gente que o `config.toml`. O log só diz para rodar
  `rustployd access-key`.
- **O botão "copiar chave" das duas UIs monta a chave da sessão**, não pede ao
  daemon: endereço + token que acabaram de funcionar daquela máquina/navegador.
  O `public_base_url` do daemon pode não ser o endereço pelo qual o cliente
  chega (sem `api.domain` ele cai no IP de saída). `GET /api/access-key`
  continua existindo, para agentes.
- **As URLs de `ai_agent` no `/api/login` usam o header `Host`** da requisição
  (esquema vindo da chave), pelo mesmo motivo.
- **Pilha estourada**: chamar `dispatch(Snapshot)` de dentro de uma ferramenta
  estourava a pilha da thread tokio em debug (o future do `dispatch` é enorme e
  o snapshot despacha outros comandos por dentro). `tools.rs` usa `Box::pin`.
- **Codec da chave fora do `rustploy-shared`**, para não exigir publicar uma
  versão nova no crates.io: Rust no daemon, Luau na GUI, JS na webui — os três
  geram bytes idênticos.
- **Formato da chave trocado** (mesmo dia, depois do primeiro uso real): de
  `rpk1_` + base64url de `{"u","t"}` para base64url de `host:porta::token`
  (seção 2), a pedido. Nenhuma compatibilidade com a `rpk1_`: o formato tinha
  horas de vida, e as listas de servidores lembrados guardam `{url, token}`,
  não a chave — elas são remontadas no formato novo sozinhas. O rótulo do
  dropdown perdeu os espaços de alinhamento: `host:porta · cnVzdH…OnRvaw`.
- **Segunda troca de formato, e o rótulo do dropdown** (2026-10-14). O pedido
  foi literal: `base64(host:port::server_token)`, igual ao `b64encode` do
  Python — base64 padrão com padding, sem esquema nenhum dentro. Junto:
  - o daemon grava o token pronto no `config.toml` (`[api].connection_token`);
  - GUI e webui deixam de chamar o `/api/login` e usam o token da API direto;
  - a GUI descobre o esquema sondando `/api/version` (https, depois http), e a
    lista de servidores lembrados deduplica por `host:porta`, não pela URL.
  - **O rótulo do dropdown virou o próprio token.** O `<ComboEdit>` do glacier
    usa o `combo_box` do iced, que mostra **no campo** o rótulo da opção
    escolhida — então a dica `host:porta · cnVzdH…` aparecia no lugar do token,
    com o endereço à mostra. Voltar com a dica só na lista exige o glacier
    separar o texto do campo do texto da lista (`AccessKey.hint` ficou no
    `access_key.luau`, sem uso, esperando isso).

- **Select + campo, em vez do `<ComboEdit>`** (2026-10-14, logo depois). Com o
  rótulo = token, a lista não dizia mais para onde cada item conectava (e os
  tokens longos vazavam da caixa). Como o combo não separa o texto do campo do
  texto da lista, a tela virou dois widgets: `<select>` com `host:porta` e um
  `input` com o token, mais a linha `→ conecta em host:porta`. Junto:
  - **"Esquecer" casa pelo token inteiro** (e pelo `host:porta` quando ele
    decodifica). Um par salvo sem token — daemon sem `[api].token` — gera
    `host:porta::`, que o decodificador recusava por token vazio; por isso o
    botão não o removia e ele "voltava".
  - **Token vazio passou a valer** nos decodificadores da GUI e da webui (é o
    caso legítimo da API aberta). O do daemon continua exigindo token: o
    `/api/login` sem token não teria o que devolver.
