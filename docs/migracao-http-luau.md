# Plano: migrar RWP → HTTP/SSE + lógica de rede em Luau (rustploy.chiquitos.tech)

## Contexto

Hoje o rustploy-gui fala com o daemon por um protocolo binário próprio (**RWP**:
TCP + postcard, portas 8787/rwps). Toda a lógica de rede vive em Rust
(`crates/rustploy-gui/src/app/rwp.rs` + `net/*`, ~2.700 linhas), inclusive um
`Root` monolítico (`app/root.rs`, 1.721 linhas) que faz o *match* de todas as
ações de UI, e um *poll loop* de 2s (`net/mod.rs::poll_stream`) que busca tudo e
formata (`net/view.rs`, 779 linhas) em strings JSON que os templates iteram.

O glacier-ui **0.14** passou a suportar **Luau** com `fetch` (async/await),
`sse` e `websocket`, além de `require` de módulos `.luau`. O objetivo é:

1. **Aposentar o RWP** e expor a mesma API como **HTTP/JSON + SSE**, servida em
   `https://rustploy.chiquitos.tech` (via ingress + ACME já existentes).
2. Mover **toda a lógica de rede reativa a cliques** para blocos `<script>` Luau
   dentro dos próprios templates (`fetch`/`sse`), com a formatação/filtragem
   reimplementada em Luau (decisão do usuário: "Luau reimplementa tudo").
3. Fazer **webhooks, OAuth e redirect** apontarem para o subdomínio (já derivam
   de `webhook_base_url` no DB — passa a valer o subdomínio).

Decisões já tomadas: **Luau reimplementa a formatação**; API **via ingress+ACME
no subdomínio**; RWP **removido de vez**.

### Restrições descobertas (guiam o desenho)
- **Contexto único global**: `GlacierUI` tem UM `context_data: HashMap<String,String>`
  (`glacier-ui/src/lib.rs:34`) compartilhado por todos os componentes. Logo, o
  `<script>` de qualquer template ativo lê/escreve o mesmo estado — não há estado
  isolado por tela a reconciliar.
- **Luau é dirigido a eventos** (`init`, handlers de clique, `on_message` de
  stream) — **não há timer/interval** na camada Luau. Portanto o *poll de 2s*
  não tem equivalente Luau: o daemon precisa **empurrar** o estado por **SSE**
  (snapshot periódico + eventos vivos), invertendo poll→push.
- **Não há JSON no Luau**: o prelúdio (`glacier-ui/src/luau/prelude.luau`) só
  expõe `fetch`/`sse`/`websocket`/`Client`. Como os templates iteram strings
  JSON e o daemon devolve JSON, o Luau precisa de `encode`/`decode` nativos —
  **inexistentes hoje**. Isto é pré-requisito e vira a Parte 0.

---

## Parte 0 — glacier-ui 0.15.0: `json` nativo para a camada Luau

Seguir a regra do projeto (`CLAUDE.md`): alterar o glacier local, **publicar** e
subir a dependência — nunca `path`/`[patch]`.

- Em `~/Development/rust/glacier-ui`, expor um global **`json`** no interpretador
  Luau com `json.encode(value)` e `json.decode(str)`, usando a ponte serde do
  `mlua` (`LuaSerdeExt`: `lua.to_value` / `lua.from_value`) — nativo e rápido,
  em vez de parser em Lua puro. Instalar junto do prelúdio em
  `luau/mod.rs::from_source` (ao lado de `install_module_system`).
  - `json.decode`: string → tabela Luau (arrays viram tabelas 1-indexadas).
  - `json.encode`: tabela Luau → string JSON (respeitando arrays).
- Testes em `luau/mod.rs` (padrão dos existentes): round-trip decode/encode,
  array de objetos, aninhado, e um caso real (decodificar um payload de lista de
  serviços e reencodar a forma que o template espera).
- Bump `glacier-ui/Cargo.toml` → `0.15.0`; `cargo publish --dry-run` → `publish`.
- Subir `crates/rustploy-gui/Cargo.toml` para `glacier-ui = "0.15.0"`;
  `cargo check -p rustploy-gui`.

> Observação: `mlua` já é dependência do glacier; `LuaSerdeExt` vem da feature
> `serialize` do `mlua` — habilitar se ainda não estiver.

---

## Parte 1 — Daemon: API HTTP/JSON + SSE (substitui o RWP)

Reaproveitar **todo** o `dispatch()` e os handlers existentes — só troca a
camada de transporte.

### 1a. Servidor HTTP de API (novo módulo `crates/daemon/src/api/http_api.rs`)
- Basear no `webhook_server.rs` (já usa `hyper` puro) — mesmo estilo de
  `service_fn`/`serve_connection`. Escuta numa porta interna de config
  (ex.: `api.port`, default 9797, bind loopback) — o **ingress** termina o TLS
  no subdomínio e faz proxy para ela (ver 1d). "Não necessariamente 443" fica
  honrado: público via ingress, interno em porta própria.
- **Auth**: header `Authorization: Bearer <token>` comparado (constant-time,
  reusar `constant_time_eq` de `rwp/server.rs`) ao token de config. Sem token +
  bind loopback = livre (como hoje).
- **Endpoints**:
  - `POST /api/rpc` — corpo = `Command` em JSON; chama `dispatch(state, cmd)`;
    responde `Response` em JSON. Um único endpoint cobre os 53 comandos sem
    roteamento por comando (Luau reencaixa a forma). `Response`/`Command` já
    derivam serde; servir com `serde_json`. (JSON é externamente-tagueado por
    padrão — o Luau lê `res.Services`, `res.Projects`, etc. A regra
    "sem skip/default serde" vale só para o wire postcard; JSON é caminho
    novo e independente.)
  - `GET /api/events` — **SSE**. Assina `state.bus` (como
    `rwp/server.rs::stream_events`) e, em paralelo, um `tokio::interval(2s)` que
    monta um **snapshot** (status + deployments + projects/services + docker +
    engine — a mesma bateria que o `poll_stream` faz hoje) e emite como um
    evento SSE `snapshot`. Cada item vira `event: <tipo>\ndata: <json>\n\n`.
    Substitui poll **e** stream numa conexão só.
  - `OPTIONS *` / cabeçalhos CORS se necessário (cliente é desktop, mas o SSE
    do glacier usa `hyper`+`rustls` direto — provável dispensa de CORS).

### 1b. Config: `RwpConfig` → `ApiConfig`
- `crates/shared/src/config.rs`: renomear/adaptar `RwpConfig` → `ApiConfig`
  (`bind_address`, `port`, `token`, `max_connections`, timeouts). Manter
  `RUSTPLOY_RWP_TOKEN` como alias aceito ou migrar para `RUSTPLOY_API_TOKEN`.
- `main.rs`: remover o `tokio::spawn(rwp::run(...))`; adicionar
  `tokio::spawn(api::http_api::run(state, api_cfg))`. Manter `webhook_server`.

### 1c. Subdomínio no banco + wiring de webhook/oauth/redirect
- O DB **já** tem `daemon_settings.webhook_base_url` (`db/daemon_settings.rs`),
  que já alimenta `get_webhook_url::build_url`, `callback_redirect_uri` e o
  OAuth (`git_oauth_start.rs`). **Definir esse valor como
  `https://rustploy.chiquitos.tech`** passa a apontar webhooks, redirect e OAuth
  para o subdomínio sem código novo.
- Adicionar (se quisermos separar o host da API do host de webhook) uma chave
  `KEY_PUBLIC_DOMAIN`/`api_base_url` em `daemon_settings.rs`; por ora reusar
  `webhook_base_url` mantém tudo num lugar só. Expor/editar via Settings
  (`get_daemon_settings.rs`/`set_daemon_settings.rs`) — já existe o campo
  `ss_domain` no GUI.

### 1d. Ingress: rota do subdomínio → API interna
- O ingress (`ingress/proxy.rs`) já é um reverse-proxy com tabela
  `HashMap<domain, upstream>` (arc-swap) e TLS/ACME. Registrar uma rota fixa
  `rustploy.chiquitos.tech → 127.0.0.1:<api.port>` (e as rotas
  `/webhook/*` e `/oauth/*` do subdomínio → `webhook_port`), de forma que o
  ACME emita o cert do subdomínio e o público chegue por 443. Verificar como
  rotas "de sistema" (não de deploy) são inseridas na tabela — provável semear
  no boot em `main.rs` junto da subida do proxy.
- **DNS**: cadastrar `rustploy.chiquitos.tech` (A/AAAA) apontando para o host —
  passo de infra, fora do código (documentar no plano de execução).

### 1e. Remoção do RWP
- Apagar `crates/daemon/src/rwp/` (`mod.rs`, `server.rs`) e a referência em
  `main.rs`/`api/mod.rs`. Mover utilitários reusados (`constant_time_eq`) para
  onde couber (`http_api.rs`).
- Remover `RwpFrame`/`RwpReply`/`RWP_PROTOCOL_VERSION`/`RwpError` de
  `crates/shared/src/protocol.rs` **após** o GUI parar de usá-los (Parte 2).
  `Command`/`Response`/`Event` **permanecem** (agora serializados em JSON).

---

## Parte 2 — GUI: `<script>` Luau substitui a camada de rede Rust

Estratégia: o glacier auto-liga um `LuauComponent` a todo template que tenha
`<script>` (`glacier-ui/src/luau/mod.rs::has_script`). O `App` Rust vira uma
casca fina (só chrome de janela + persistência local); o `Root` monolítico some.

### 2a. Bibliotecas Luau reutilizáveis (`crates/rustploy-gui/templates/lib/`)
Módulos `require`áveis (resolvidos relativo ao template; ver `module_roots`):
- **`net/api.luau`** — client sobre o `Client`/`fetch` do prelúdio: base URL +
  header `Authorization: Bearer`; método `rpc(cmd_table)` que faz
  `POST /api/rpc` com `json.encode`, e devolve `json.decode(res.body)`. Encapsula
  o transporte (equivalente ao antigo `rwp.rs`/`RwpClient`).
- **`fmt.luau`** — reimplementa `view.rs`: `fmt_bytes`, `fmt_uptime`,
  `fmt_secs`, e os *builders* de lista (services/projects/deployments/docker/
  ingress/monitoring/logs) que produzem a **mesma forma JSON** que os templates
  já iteram (para não mexer na markup de listas). Filtro de busca
  case-insensitive vive aqui (equivalente a `search_pairs`).
- **`state.luau`** (opcional) — helpers de leitura/escrita de chaves de contexto
  agregadas.

### 2b. `app.xml` — dono da conexão e do stream de estado (sempre montado)
- `<script>`: `init` lê `ctx.api_url`/`ctx.api_token` (do login) e abre **um**
  `sse(api_url.."/api/events", { headers=..., on_message="on_state" })`.
- `on_state(data)`: `json.decode`, despacha por tipo de evento:
  - `snapshot` → chama os builders de `fmt.luau` e escreve as chaves de lista
    (`ctx.services`, `ctx.projects`, `ctx.deployments`, `ctx.docker_*`,
    `ctx.ingress`, `ctx.monitoring`, contadores, `daemon_*`, `eng_*`, `sys_*`).
  - `LogLine`/`BuildLog`/`ContainerMetrics`/`SystemMetrics` → mantêm o
    ring-buffer e as chaves `svc_logs*`/`dep_build_*` (a lógica de seleção usa
    `ctx.selected_service`/`ctx.selected_deploy`, hoje em `selected_shared`).
  - Detecção de fim de deploy + timer: como não há timer Luau, o "1s,2s,3s…" vem
    do próprio snapshot de 2s (mostrar decorrido calculado de `started_at`), ou
    aceitar granularidade de 2s. (Simplificação consciente vs. o tick de 1Hz atual.)

### 2c. Handlers por template (a lógica que hoje está no `match` de `root.rs`)
Para cada tela, um `<script>` com as funções nomeadas pelos `onClick`/`onChange`/
`onSubmit` existentes, chamando `net/api.luau`:
- **`login.xml`** — `submit_login`: valida url/token (validação simples em Luau,
  substituindo `login_form`), guarda em `ctx`, navega para `shell`, dispara a
  conexão. Persistência de `Prefs` (url/token lembrados) continua no Rust via um
  efeito pequeno **ou** migra para um endpoint — decisão menor (ver Riscos).
- **`home.xml`** — ações de Deployments/Projects/Monitoring/Ingress/Docker/
  Settings: criar/editar/apagar projeto, `start_deploy`, `stop_all`,
  prune de images/volumes/networks, salvar settings, conectar/desconectar Gitea
  (inicia OAuth abrindo URL no browser — a URL vem de `Command::GitOAuthStart`).
- **`service.xml`** / **`new_service.xml`** — detalhe do serviço e wizard: sub-abas,
  editar spec/env, deploy, logs, webhook regen, etc.
- **`shell.xml`** — busca (`onChange` refiltra via `fmt.luau` sobre o último
  snapshot guardado em contexto), Stop All, Disconnect.

### 2d. Redução do Rust do GUI
- **Apagar**: `app/rwp.rs`, `app/net/*` inteiro, e o corpo de `app/root.rs`
  (o `Root`/`PollKey`/forms/`update` match). `app/wizard.rs` migra para Luau.
- **Manter**: `app/mod.rs` (chrome de janela: `window:*`, geometria, ícone —
  handlers Rust que interceptam `EngineMessage::UiClick` continuam válidos),
  `app/store.rs` (Prefs/WindowState em JSON local), `main.rs`, `assets.rs`.
- `App::boot`: em vez de `motor.register(Box::new(Root::default()))`, registrar
  os templates como componentes por arquivo (mecanismo de registro do glacier
  0.14 — "unifica registro de componentes"; confirmar a API exata:
  `register`/`register_template`/`link rel=import`). Manter
  `register_bare_flags(["else","senao"])`, `load_stylesheet`, `set_initial_screen("app")`.
- `connect_target`/scheme `rwp://` em `app/mod.rs` → passa a normalizar
  `https://` (ou aceitar host puro + porta). Ajustar `DEFAULT_RWP_PORT`.

---

## Sequenciamento (evitar big-bang)

1. **Parte 0** (glacier 0.15.0 com `json`) — isolado, publica e sobe dep.
2. **Parte 1** no daemon **mantendo o RWP vivo em paralelo** temporariamente
   (não apagar `rwp/` ainda): sobe `/api/rpc` + `/api/events` e valida com `curl`.
3. **Parte 2** no GUI, tela a tela, começando por `login` + `app.xml`(SSE) +
   Deployments (read-only via snapshot), depois ações mutantes, depois service
   detail e wizard.
4. **Corte final**: remover `rwp/` do daemon e os tipos `Rwp*` de `shared`, e a
   camada Rust de rede do GUI. Setar `webhook_base_url` para o subdomínio e
   cadastrar a rota no ingress + DNS.

---

## Verificação

- **Parte 0**: `cargo test -p glacier-ui` (novos testes de `json`);
  `cargo run --example imports_luau` ainda roda (regressão do prelúdio).
- **Daemon**: `cargo run -p daemon`; então
  `curl -H "Authorization: Bearer <tok>" -d '{"DaemonStatus":null}' localhost:9797/api/rpc`
  e `curl -N .../api/events` para ver snapshots/eventos SSE chegando.
- **GUI**: `cargo run -p rustploy-gui` (a partir da raiz do workspace — paths de
  template são relativos ao CWD). Fluxo end-to-end: login → lista carrega via
  SSE → criar projeto → start deploy → logs ao vivo → prune docker → settings.
  (Memória do usuário: *rodar o exemplo/app antes de dar por pronto* — não
  confiar só em teste verde para UI.)
- **Webhook/OAuth**: com `webhook_base_url=https://rustploy.chiquitos.tech`,
  conferir `GetWebhookUrl` retornando URL do subdomínio e o fluxo OAuth do Gitea
  completando no `/oauth/gitea/callback` do subdomínio.

## Riscos / pontos a confirmar na execução
- **API de registro de componentes por arquivo no glacier 0.14** (como ligar
  `home.xml`/`service.xml` como `LuauComponent`s e trocar de tela sem um `Root`
  Rust) — confirmar no README/`lib.rs` antes de reescrever o `boot`.
- **Volume de reescrita**: `view.rs` (779) + `root.rs` match (1.721) viram Luau;
  é a maior fatia de esforço e risco — daí o sequenciamento tela a tela.
- **Granularidade do timer de deploy** cai de 1Hz para 2s (sem timer Luau);
  aceitável, ou expor um `sse` dedicado de progresso.
- **Persistência de Prefs/geometria**: fica em Rust (`store.rs`) acionada por
  efeito, ou migra para arquivo via daemon — manter em Rust é o menor atrito.
- **Segurança do subdomínio**: API pública exige `token` forte (o guard atual já
  recusa bind não-loopback sem token; manter equivalente no `http_api`).
