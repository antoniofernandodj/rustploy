# Rustploy — guia do projeto

> PaaS de baixo consumo escrito em Rust, sem orquestrador externo. Alternativa
> ao Dokploy/Coolify para VPS modestas: um binário só, sem Swarm nem
> Kubernetes, proxy reverso embutido e footprint de serviço abaixo de 50 MB.

**Este arquivo é a referência única do projeto.** O `CLAUDE.md` apenas aponta
para cá — até 2026-08-28 os dois descreviam a arquitetura em paralelo, o que
garantia que um dos dois estaria errado.

Como este guia está organizado:

| Parte | Para quem |
|---|---|
| [1 — Manual de Controle por Agente](#parte-1--manual-de-controle-por-agente) | quem vai **operar** um rustploy por HTTP |
| [2 — Trabalhando no código](#parte-2--trabalhando-no-código) | quem vai **mexer** no repositório |
| [3 — Arquitetura](#parte-3--arquitetura) | como o sistema funciona por dentro |
| [4 — História](#parte-4--história) | o que já mudou e por quê |

---

# Parte 1 — Manual de Controle por Agente

> A API de agente foi implementada em 2026-08-28. As decisões de desenho por
> trás dela estão em `docs/api-agente-no-gui.md`; a implementação, em
> `crates/rustploy-gui/src/agent/`.

## O modelo mental

Um agente **não fala com o daemon**. Fala com o **app GUI**, que já está logado
no daemon e empresta a própria sessão:

```
  agente local ──HTTP──> 127.0.0.1:9800 ──HTTPS──> rustploy remoto
                           (rustploy-gui)            POST /api/rpc
                                 │
                                 └─ sessão (url + token) da janela
```

Três consequências que são o motivo do desenho:

1. **O agente nunca vê o token do daemon.** Usa um token local, próprio, que
   morre com o processo.
2. **O agente não precisa saber onde o daemon está.** Fala com `localhost`.
3. **Trocar de servidor na GUI troca o alvo do agente junto.** Não há estado
   paralelo para sair de sincronia.

O alcance é exatamente o da janela — nem mais, nem menos.

## Descoberta (comece aqui)

Um arquivo, escrito quando o app sobe:

```bash
cat ~/.local/share/rustploy/agent-api.json
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

É o passo de descoberta inteiro. `GET /agent/schema` conta o resto em JSON.

```bash
export RP=$(python3 -c "import json;print(json.load(open('$HOME/.local/share/rustploy/agent-api.json'))['url'])")
export RPT=$(python3 -c "import json;print(json.load(open('$HOME/.local/share/rustploy/agent-api.json'))['token'])")
alias rp='curl -s -H "Authorization: Bearer $RPT"'
```

O arquivo nasce `0600` — quem lê o token opera o rustploy remoto inteiro. O
token é novo a cada execução, então um handoff velho no disco não vale nada
(confira o `pid`, ou simplesmente chame `/agent/health`).

## Todas as rotas

Autenticação: `Authorization: Bearer <token do handoff>` em tudo, exceto
`GET /agent/health`. Toda falha é JSON:
`{"error":{"code":"…","message":"…"}}`.

### Dados (encaminhados ao daemon)

| Rota | Para quê |
|---|---|
| `GET /agent/health` | a ponte está no ar? a janela está conectada? (sem token) |
| `GET /agent/schema` | o catálogo completo, legível por máquina |
| `GET /agent/status` | versão/uptime do daemon + fila de deploys |
| `GET /agent/services` | índice achatado projeto→serviço |
| `GET /agent/deploys?limit=` | últimos deploys **com o desfecho resolvido** |
| `POST /agent/deploys` | dispara um deploy e **espera** o resultado |
| `GET /agent/deploys/<id>/logs?after=&limit=` | build log paginado por cursor |
| `POST /agent/services/<id>/archive` | sobe um `.zip` local (por caminho) |
| `POST /agent/rpc` | passthrough: qualquer `Command` do protocolo |

### Janela (controlam a GUI em si)

| Rota | Para quê |
|---|---|
| `GET /agent/servers` | servidores já usados nesta máquina |
| `POST /agent/connect` | **entra na sessão** pela tela de login |
| `POST /agent/disconnect` | sai da sessão |
| `GET /agent/ui` | o que a janela está mostrando agora |
| `GET /agent/ui/actions` | **todas** as ações que a GUI aceita |
| `POST /agent/ui/action` | dispara qualquer ação, como um clique |
| `POST /agent/ui/context` | escreve chaves no contexto da janela |

Códigos: `400` pedido malformado ou comando recusado pelo daemon · `401` token
desta API errado · `404` rota/serviço inexistente · `502` daemon inalcançável ou
recusou · `503` a janela não está conectada a daemon nenhum.

## Receitas

### 1. Conectar sem ninguém na frente do app

Era o único ponto que exigia um humano. Hoje:

```bash
rp $RP/agent/servers          # o que já foi usado aqui
rp $RP/agent/connect -d '{"url":"https://rustploy.exemplo.com"}'
# → {"connected":true,"remote_url":"https://rustploy.exemplo.com"}
```

Sem `token` no corpo, a ponte usa o **salvo** para aquela URL — o segredo não
atravessa a rede em nenhum sentido. Com um servidor novo, mande
`{"url":"…","token":"…"}`.

A rota **espera o desfecho** (até 30 s) e a janela acompanha de verdade: valida
com um `DaemonStatus`, abre o SSE, carrega as configurações e vai para a tela
`shell`. Token errado → `502 connect_refused`.

### 2. Deployar e saber se funcionou

A rota que existe para responder as duas perguntas de uma vez:

```bash
rp $RP/agent/deploys -d '{"service":"stand-imob","wait":true}'
```

```json
{
  "deployment_id": "dep_01M11Z99…",
  "state": "Failed",
  "ok": false,
  "error": "docker build error: Cannot locate specified Dockerfile: Dockerfile",
  "log_tail": ["…", "==> Erro em [BuildingImage]: …", "==> Deploy falhou — iniciando rollback"],
  "log_cursor": 6,
  "waited": true,
  "timed_out": false
}
```

- `ok`: `true` = Live, `false` = Failed, `null` = ainda rodando, ou terminal sem
  desfecho (`Stopped` = parado de propósito, `Pruning` = substituído por um
  deploy mais novo). Chamar esses dois de falha seria mentira.
- Aceita `"service"` (nome) ou `"service_id"`. Nome é único **por projeto**, não
  globalmente: com ambiguidade a rota recusa e lista os candidatos.
- `wait:false` devolve `202` na hora, e você acompanha por `GET /agent/deploys`.
- `timeout_s` (padrão 900, teto 3600), `log_tail` (padrão 40, teto 500).

### 3. Acompanhar um build longo sem rebaixar tudo

```bash
rp "$RP/agent/deploys/dep_…/logs?after=120"
# → {"total":1402,"after":120,"next_after":620,"has_more":true,"lines":[…]}
```

Passe o `next_after` da resposta anterior. O `GetBuildLogs` do daemon só sabe
devolver o log inteiro; a fatia acontece na ponte.

### 4. Subir um zip

```bash
rp $RP/agent/services/svc_…/archive -d '{"path":"/home/user/app.zip"}'
```

Recebe o **caminho local**, não os bytes — a ponte é loopback, mandar dezenas de
MB em base64 seria custo puro. No daemon isto é rota HTTP com corpo binário,
**não** um `Command`: não dá para descobrir lendo `protocol.rs`.

### 5. Dirigir a janela

```bash
rp $RP/agent/ui
# → {"screen":"shell","view":"deployments","selected_service":"", …}

rp $RP/agent/ui/actions          # o chaveiro: 144 ações, com o arquivo de origem
rp $RP/agent/ui/action  -d '{"action":"nav_docker"}'                  # clique
rp $RP/agent/ui/action  -d '{"action":"docker_tab","value":"images"}' # onChange
rp $RP/agent/ui/context -d '{"search":"nginx"}'                       # campo
```

**A chave-mestra é `ui/action`.** Todo botão, aba e formulário da GUI é uma
função global do Luau (`views/scripts/handlers/*.luau`), e essa rota chama
qualquer uma delas pelo mesmo caminho de um clique. Cobre a superfície inteira
por construção — inclusive telas que ainda não existem. `GET /agent/ui/actions`
lista os nomes lidos da árvore em execução, então não há lista para envelhecer.

Ações com valor (`value`) são o `onChange` de um campo; sem valor, um clique.
Abrir janela-filha (novo projeto, wizard, logs) também é ação — ex.:
`open_new_project_window`.

Para ler chaves fora do resumo: `GET /agent/ui?keys=screen,view,docker_tab` ou
`?all=1` para o contexto inteiro (resposta grande).

### 6. Qualquer outra coisa

```bash
rp $RP/agent/rpc -d '{"DeployHistory":{"service_id":"svc_…","limit":3}}'
```

Aceita qualquer `Command`, esteja no catálogo ou não. É a válvula que mantém as
rotas de conveniência honestas: elas existem para os caminhos frequentes, não
para virarem a única porta.

Codificação serde **externally-tagged**: variante com campos é objeto de uma
chave (`{"ProjectCreate":{…}}`), variante sem campos é a string nua
(`"ProjectList"`). Vale para `Command` **e** para `Response`. Fonte da verdade:
`crates/shared/src/protocol.rs` e `models.rs`.

## Armadilhas

- **`202` não é `200`.** Ações de UI (`ui/action`, `ui/context`) são
  *entregues*, não confirmadas — o efeito é assíncrono. Confirme em
  `GET /agent/ui` ou na rota de dado correspondente. `POST /agent/connect` é a
  exceção: essa espera o desfecho.
- **Método errado dá `404`, não `405`.** `curl` sem `-d` manda GET; use
  `-X POST` nas rotas sem corpo (`/agent/disconnect`).
- **`ServiceUpdate` é substituição total, não patch.** Faça `ServiceGet`, mude o
  campo, devolva o spec inteiro. Campo omitido é campo apagado.
- **`api_token` e `token` saem sempre como `"<redigido>"`** em `/agent/ui` — o
  desenho inteiro é o agente operar sem vê-los.
- **A janela pode estar recolhida na bandeja.** O motor segue vivo e aceitando
  ações; não é preciso abri-la para operar.
- **Log de container não é persistido.** `LogsGet` lê do Docker ao vivo, e o
  swap de deploy destrói o container antigo com o log junto. Build log
  (`build_log`) esse sim é persistido.
- **`503 not_connected`** quer dizer que a janela não está logada — use
  `POST /agent/connect`, não é erro de rede.

## Configuração e limites

| Variável | Efeito |
|---|---|
| *(nada)* | liga em `127.0.0.1:9800` |
| `RUSTPLOY_AGENT_API=off` | desliga a ponte |
| `RUSTPLOY_AGENT_API=127.0.0.1:9910` | outra porta |

Endereço não-loopback é **recusado** e cai no default: isto é uma ponte para
processos da mesma máquina, não um segundo daemon. Porta ocupada → porta
efêmera, e o handoff diz qual (por isso leia o arquivo, não assuma a porta).

**Sem escopo.** Quem tem o token do handoff tem o alcance da janela, que é o
bearer do daemon — hoje sem escopo nenhum. Não há modo somente-leitura.
Enquanto o daemon não tiver tokens com escopo (read-only / deploy / admin),
esta ponte não tem como inventar um. Ver a nota de segurança em
`docs/plano-erro-de-deploy-invisivel.md`.

## Como isto funciona por dentro

Relevante quando algo não responde como esperado:

- O servidor é **hyper** (não axum), numa thread com runtime tokio próprio — o
  loop do iced é dono da thread principal. Vive em
  `crates/rustploy-gui/src/agent/`.
- A sessão é **observada**, não notificada: o gancho `on_message` do
  `GlacierDaemon` roda depois de cada dispatch da janela principal, e a ponte
  relê o contexto ali. Cobre login, logout e troca de servidor sem conhecer
  nenhum dos três fluxos.
- O caminho de volta (ponte → janela) é o canal `external` do **glacier-ui
  0.58.6+**: `ExternalSender` injeta no motor o mesmo `EngineMessage` que um
  clique produz. É o que tornou `connect`/`ui/action`/`ui/context` possíveis —
  antes disso a GUI só se movia por evento do loop do iced.

------

# Parte 2 — Trabalhando no código

## Convenções

### glacier-ui: nunca `path`, nunca `[patch]`

A crate `rustploy-gui` consome `glacier-ui` **do crates.io** (versão fixada no
`Cargo.toml`), não o código-fonte local em `~/Development/rust/glacier-ui`.

Quando uma mudança no `glacier-ui` for necessária — renomear um item público,
corrigir bug, adicionar recurso — o fluxo é **sempre publicar uma nova versão e
subir a dependência**. Nunca contorne com `[patch.crates-io]` ou dependência por
`path`:

0. **Antes de qualquer coisa, conferir se o tree local bate com o que está
   publicado.** A `version` do `Cargo.toml` local não é prova: já aconteceu de o
   repositório parar na `0.57.0` enquanto o crates.io seguia até a `0.57.4`
   (versões publicadas de outra máquina, nunca commitadas nem enviadas ao
   `origin`). Bumpar dali teria revertido quatro versões em silêncio. Comparar
   com o pacote real:

   ```bash
   # a versão publicada fica em ~/.cargo/registry depois de qualquer build que a use
   diff -rq ~/Development/rust/glacier-ui/src \
            ~/.cargo/registry/src/*/glacier-ui-<versão-publicada>/src
   ```

   Conferir os **dois sentidos** — `git fetch` e comparar com o `origin`
   também, não só com o crates.io. Se divergir, recuperar primeiro (o
   `Cargo.toml.orig` do pacote preserva os comentários; o `Cargo.toml` é gerado
   pelo cargo e os descarta) e commitar essa recuperação **separada** da
   mudança nova.
1. Aplicar a mudança em `~/Development/rust/glacier-ui`.
2. Bump da versão em `glacier-ui/Cargo.toml` (ex.: `0.58.5` → `0.58.6`) e
   entrada no `CHANGELOG.md`.
3. **Rodar um exemplo de verdade** (`cargo run --example …`) — `cargo test`
   verde não basta para uma feature de UI.
4. Commit completo **antes** do publish, e `git push`.
5. `cargo publish` (validar antes com `cargo publish --dry-run`).
6. Subir a versão em `crates/rustploy-gui/Cargo.toml` para a recém-publicada.
7. `cargo check -p rustploy-gui` para confirmar.

O passo 4 é o que evita a divergência do passo 0 — foi justamente pulá-lo que a
criou, duas vezes.

**Se algo parecer faltar no glacier**, o lugar de consertar é o builder do
`GlacierDaemon`, não um runtime paralelo dentro do `rustploy-gui`. Já houve um
runtime `iced::daemon` inteiro reimplementado aqui (~250 linhas) porque o
builder não expunha multi-janela; o buraco foi fechado na 0.38 e o runtime
local, removido.

### Título e tamanho de janela moram no `.gv`, não no Rust

Desde o glacier-ui **0.59**, cada janela declara o que ela é no cabeçalho do
próprio template — não procure isso no `main.rs`/`app/mod.rs`:

```xml
<screen title="Novo job — Rustploy" size="560 700">
    <resources>
        <link rel="theme" href="…" />
        <style> … </style>
        <script src="scripts/new_job_window.luau"></script>
    </resources>

    <column class="…"> … </column>
</screen>
```

Onde cada coisa vive hoje:

- **`views/app.gv`** declara `title`, `size` e `min-size` da janela principal.
  O `main_window_settings()` em `app/mod.rs` ficou só com o chrome que o
  template não descreve (borderless, ícone, `application_id`,
  `exit_on_close_request`), e o builder não chama mais `.title()`/`.main_size()`.
- **As janelas-filhas** (`new_*_window.gv`, `new_project_form.gv`,
  `log_window.gv`) declaram as suas, e as chamadas `open_window{…}` dos
  handlers Luau passam só o `file` (mais `data`). Duas exceções propositais, em
  que só quem abre sabe o título: `log_window.gv` (título dinâmico — "Logs —
  nginx", "Build — abc123") e a edição de job, que sobrepõe o "Novo job" do
  arquivo.
- A **geometria lembrada** (`remember_window_geometry`) continua ganhando do
  `size` declarado: ele é o tamanho de *primeira* abertura.

O `<resources>` agrupa o que não desenha (`<style>`, `<script>`, `<link>`,
`<import>`); é opcional, mas nos templates de janela daqui ele já está em uso.
Um engano no cabeçalho é erro de parse (atributo desconhecido, tamanho que não
seja par de números, widget dentro do `<resources>`) — não passa em silêncio.
O teste `janelas_declaram_titulo_e_tamanho_no_proprio_template`
(`tests/templates_render.rs`) trava esses valores.

### Toda feature de UI vive em dois lugares

O daemon serve uma **webui própria** (`crates/daemon/webui/`, HTML + Alpine.js)
além do cliente `rustploy-gui`. Os dois consomem a mesma API. Uma mudança de
interface que só entre num dos dois vira divergência silenciosa — foi o que
aconteceu com o motivo da falha de deploy, que a GUI passou a mostrar e a webui
continuou ignorando por um tempo.

Gotcha recorrente da webui: filhos de `.scroll_fill` (acima **ou** dentro, ex.
`.grid`) sem `flex-shrink: 0` são espremidos até sumir. Checar em toda tela ou
card novo.

A webui é servida com `Cache-Control: immutable` por um ano — testar na mesma
porta reaproveita o JS velho do cache do navegador. Use ctrl+shift+r.

### Luau

**Ferramental.** Type-check toda mudança antes de considerá-la pronta:

```bash
luau-lsp analyze --base-luaurc=.luaurc \
  --definitions=crates/rustploy-gui/views/scripts/glacier.d.luau <arquivo(s)>
```

Não substitui `cargo test -p rustploy-gui --test templates_render` (o runtime
`mlua` de verdade), mas pega erro de caminho de módulo e de tipo em segundos.
Para o VS Code, a extensão `johnnymorganz.luau-lsp` (config já versionada em
`.luaurc` + `.vscode/settings.json`). O `glacier.d.luau` **precisa** do
tratamento `--definitions=` / `luau-lsp.types.definitionFiles`: sem isso o
editor o interpreta como script comum e levanta ~40 erros falsos. Ver
`docs/luau-modularizacao-pacotes.md`.

**Convenção de `require`** (glacier-ui 0.22+, resolução relativa ao arquivo que
chama, como Node.js):

- irmão no mesmo diretório → nome nu: `require("stream")`;
- pacote pai a partir de dentro de `handlers/` ou `fmt/` → `require("../state")`;
- do script de entrada (`app.luau`, na raiz) → caminho completo:
  `require("handlers/connection")`.

**Convenção de sintaxe.** Quando o **único** argumento de uma chamada é um table
literal, use a forma sem parênteses — `f{ ... }`, não `f({ ... })` (idem
`toast{...}`, `api:rpc_checked{...}`, `open_window{...}`, `json.array{}`,
`os.time{...}`, `ipairs{...}`). Vale **só** para o único-argumento-table:
chamadas com mais de um argumento (`prune({...}, "msg")`,
`setmetatable({...}, mt)`) mantêm os parênteses. String literal única também
poderia dispensar parênteses, mas **não** adotamos essa forma.

### Armadilhas de template (`.gv`)

- **Nunca escreva uma tag literal dentro de um comentário** (nem dentro de
  `<style>`): o parser quebra e o erro aponta para a linha errada.
- **O GSS não suporta seletor por vírgula** (`.a, .b { }`, nem dentro de
  `@media`): vira uma chave só, errada, e falha em silêncio. Uma declaração por
  seletor.
- **`if=` como atributo** condiciona só o elemento; a **tag** `<if>` condiciona
  todos os filhos e usa `cond=`, não `if=`.

### Descartar ≠ apagar

Código reutilizável que sai de uso vira comentário com `TODO` no lugar, não
deleção — o histórico do git existe, mas o contexto de "por que isso estava
aqui" se perde.

## Build & Run

```bash
# Tudo
cargo build
cargo build --release

# Daemon (precisa do socket do Docker). `default-run = "rustployd"`, então o
# `--bin` só é necessário para o outro binário (`rustployd-fw`).
cargo run -p rustploy

# GUI
cargo run -p rustploy-gui
```

**Testes.** Os diretórios são `crates/daemon` e `crates/shared`, mas os
**pacotes** se chamam `rustploy` e `rustploy-shared` (renomeados para o
`cargo install rustploy`) — `-p daemon` / `-p shared` não resolvem. O daemon não
tem lib target, então seus testes ficam sob `--bins`.

```bash
cargo test -p rustploy --bins <nome_do_teste>
cargo test -p rustploy-shared
cargo test -p rustploy-gui --bins            # inclui a API de agente
cargo test -p rustploy-gui --test templates_render   # runtime mlua real
cargo check --workspace
```

Três testes de `web_ui::headless_tests` exigem `google-chrome` instalado e
falham por ambiente onde ele não sobe — não são regressão.

**Rodar a GUI sem monitor** (para screenshot): ver
`docs/plano-convergencia-templates-gui-webui.md` e a receita de Xvfb. Dois
detalhes que custam tempo: a tela do Xvfb precisa ser **maior** que a janela
(1400x900; o default do glacier é 1024x768, e menor que isso a janela nunca
mapeia, em silêncio), e `WAYLAND_DISPLAY` precisa ser removido do ambiente
(`env -u WAYLAND_DISPLAY DISPLAY=:99 …`) ou o winit tenta Wayland e trava sem
logar nada. A trava de instância única (`single_instance`) ocupa uma porta fixa
derivada do app id — com o app instalado rodando, o build de dev **sai em
silêncio**. Desde a API de agente, quase nada disso é necessário: para conferir
*comportamento*, use as rotas da Parte 1; Xvfb só para conferir pixel.

## Configuração

Carregada de `$RUSTPLOY_CONFIG`, depois `/etc/rustploy/config.toml`, depois
`~/.config/rustploy/config.toml`. Sem nenhum deles, valem os defaults.

O parse é **tudo ou nada**: um `RUSTPLOY_CONFIG` só é aceito se der parse
completo, sem merge parcial com os defaults. Um arquivo de teste precisa
declarar **todas** as seções (`[daemon]`, `[ingress]` + `[ingress.acme]`,
`[docker]`, `[deploy]`, `[metrics]`, `[secrets]`, `[api]`, `[env_backup]`,
`[external_ports]`, `[registry]`).

Overrides por env: `RUSTPLOY_DB_PATH`, `RUSTPLOY_LOG_LEVEL`,
`RUSTPLOY_API_TOKEN`. O daemon loga JSON estruturado; a verbosidade sai de
`RUST_LOG=<nível>` ou `RUSTPLOY_LOG_LEVEL`.

Defaults que importam: banco em `/var/lib/rustploy/db`, chave mestra em
`/etc/rustploy/master.key`, API em `127.0.0.1:9797`, ingress em `0.0.0.0:8080`
e `:443`, registry desligado na porta `5100`, portas externas na faixa
`20000–20999`.

**Guarda de segurança**: a API se recusa a subir com bind não-loopback e sem
`api.token` configurado. Com `api.domain` definido, a própria porta da API
termina TLS com certificado ACME.


# Parte 3 — Arquitetura

## Visão geral

```
┌──────────────────────────────────────────────────────────────┐
│ Host Linux                                                   │
│                                                              │
│  rustployd (binário único)                                   │
│   ├── ingress hyper  :80/:443   proxy reverso + ACME         │
│   ├── API HTTP/JSON + SSE  :9797                             │
│   ├── deploy executor          máquina de estados            │
│   ├── SQLite (sqlx)            projetos/serviços/deployments │
│   ├── registry OCI embutido    :5100 (opcional)              │
│   └── webui estática           servida na mesma porta da API │
│                                                              │
│  rustployd-fw  (root, socket activation)  allow/deny no ufw  │
│  dockerd       /var/run/docker.sock                          │
└──────────────────────────────────────────────────────────────┘
             ▲                                ▲
             │ HTTP/JSON + SSE                │ HTTP/JSON + SSE
      rustploy-gui (desktop)            navegador (webui)
             │
             └── API de agente em 127.0.0.1:9800 (Parte 1)
```

## Crates

| Crate | Binário | Papel |
|---|---|---|
| `shared` | — | Modelos, tipos do protocolo e structs de config, usados pelo daemon e pela GUI |
| `daemon` | `rustployd` | Servidor: API, banco, Docker, ingress, motor de deploy, registry |
| `rustploy-gui` | `rustploy-gui` | Cliente desktop glacier-ui (XML→iced). Toda a rede e lógica de negócio vive em **Luau** (`views/scripts/`), falando com o daemon pela API HTTP/JSON + SSE |
| `fw-helper` | `rustployd-fw` | Helper privilegiado de firewall (root, socket activation em `/run/rustploy/fw.sock`). O daemon pede allow/deny de portas externas (`daemon/src/firewall.rs`); o helper só aceita portas dentro da faixa `[external_ports]` e só fala com o ufw. **Sem dependência da crate `shared`, de propósito.** Ver `docs/relatorio-porta-externa-automatica.md` |

Os identificadores são ULIDs, com prefixo por tipo (`prj_`, `svc_`, `dep_`,
`arc_`).

## Protocolo da API

O daemon tem **um** protocolo voltado a cliente: HTTP/JSON + SSE
(`crates/daemon/src/api/http_api.rs`).

| Rota | O que faz |
|---|---|
| `POST /api/rpc` | um `Command` por requisição — a superfície inteira do protocolo |
| `GET /api/events` | SSE: snapshot completo a cada 2s + eventos do bus |
| `GET /api/health` | liveness |
| `GET /api/services/<id>/logs` | SSE dedicado do log de runtime |
| `POST /api/services/<id>/archive` | upload de zip (corpo binário — **não** é um `Command`) |
| `/webhook/…`, `/oauth/…` | rotas públicas, autenticação própria, fora do gate de bearer |

`Command`, `Response` e `Event` vivem em `crates/shared/src/protocol.rs` e são
despachados por `dispatch()` (`api/routes.rs`, um handler por variante em
`api/handlers/`).

**Codificação serde externally-tagged**: variante com campos é objeto de uma
chave (`{"ProjectCreate":{…}}`); variante sem campos é a string nua
(`"ProjectList"`). Vale para `Command` **e** para `Response` — esquecer o
segundo caso é o erro clássico de quem escreve cliente. JSON é
auto-descritivo, então um campo renomeado vira `nil` do lado Luau em vez de
decodificar como lixo silenciosamente.

As respostas do `POST /api/rpc` são **comprimidas com gzip** quando o cliente
manda `Accept-Encoding: gzip` (o `fetch` do glacier-ui 0.51+ manda por padrão e
descomprime transparente) e o corpo passa de 1 KB — ganho em conexão remota,
no-op prático em localhost. O SSE **não** é comprimido, de propósito: stream de
vida longa. Ver `docs/compressao-gzip-api.md`.

Cada registro do SSE é **auto-descritivo**: o `data` JSON carrega um campo
`kind` (`"snapshot"` / `"bus"`), porque o cliente SSE do glacier-ui descarta a
linha `event:` e só enxerga o `data:`.

## Daemon (`crates/daemon/src/`)

- **`api/routes.rs`** — `dispatch()` casa cada variante de `Command` com seu
  módulo handler.
- **`api/handlers/`** — um arquivo por comando (`deploy_start.rs`,
  `project_create.rs`, …).
- **`db/`** — wrappers SQLite (via `sqlx`) para projetos, serviços,
  deployments, jobs, secrets, certificados. Tabelas de log persistidas:
  `build_log` e `job_log`. **Não existe tabela de log de runtime de container** —
  `LogsGet` e o SSE de logs leem do Docker ao vivo, então o swap de deploy leva
  o log do container antigo junto.
- **`deploy/executor.rs`** — `DeployExecutor` roda a máquina de estados num
  `tokio::spawn`. Ver a seção abaixo.
- **`docker/`** — wrappers bollard: `images` (pull/build), `containers`
  (create/start/stop/rename/remove), `networks` (rede bridge por projeto,
  `rp_net_<prefixo_do_projeto>`). Não há `volumes.rs`: o rustploy nunca cria
  volume nomeado, só bind mount (`ServiceSpec.volumes`).
- **`api/handlers/docker_inventory.rs`** — listagem do host inteiro para a aba
  Docker (`DockerImages`/`Volumes`/`Networks`/`Containers`), não só o que é do
  rustploy. Imagens e volumes vêm de **uma** chamada `docker system df` — o
  único endpoint do Docker Engine que já calcula contagem de uso de graça;
  networks são cruzadas à mão com `list_containers(all: true)`, porque o
  endpoint de listagem de networks nunca preenche o próprio campo `Containers`.
  Atribuição de projeto/serviço é melhor esforço: imagens por tag
  (`rp_<safe_name>:…` para builds Git, string exata para imagens de registry),
  networks pela convenção `rp_net_<id_curto>`; volumes não têm atribuição
  nenhuma (não há label para correlacionar). Também tem o `stop_all_managed`
  (`Command::StopAllManaged`), que para todo serviço do rustploy replicando o
  `service_stop::handle`, **independente do que a coluna de status diz** — assim
  drift de estado não deixa container rodando.
- **`api/handlers/docker_prune.rs`** — remove imagens/volumes/networks/
  containers/cache de build sem uso (`Prune*`, todos por `Response::PruneResult`).
- **`ingress/proxy.rs`** — proxy reverso hyper, HTTP/1.1. A tabela de rotas é um
  `HashMap<domínio, upstream>` protegido por `arc-swap`: leitura lock-free no
  hot path de cada requisição, escrita por swap atômico do ponteiro quando o
  executor promove um deploy.
- **`ingress/tls.rs`** — ACME via `instant-acme` + `rustls`, com renovação
  automática em background.
- **`event_bus.rs`** — canal de broadcast em processo. Os módulos publicam
  `Event`; o handler do SSE cria um subscriber por conexão. Se o canal encher, o
  evento é descartado em silêncio — **jamais bloqueia o produtor**.
- **`secrets.rs`** — criptografia `age`. Secrets guardados por nome,
  referenciados em `ServiceSpec.env_vars` como `EnvVarValue::Secret(nome)`,
  decifrados em memória só na hora de criar o container.
- **`metrics.rs`** — laço de fundo que consulta as estatísticas do Docker e
  publica `ContainerMetrics`.
- **`registry/`** — registry OCI embutido (auth Basic por token, ingress, GC).
- **`firewall.rs`** — cliente do `rustployd-fw`.

## Máquina de estados do deploy

```
Pending → PreDeployCheck → ResolvingDeps
                              ├─ PullingImage ──┐   (fonte Registry)
                              ├─ CloningRepo → BuildingImage ─┤ (Git / Archive)
                              └─ ComposingUp ───┤   (fonte Compose)
                                                ▼
                    Staging → HealthcheckPolling → SwappingIn
                                    │ falha              ▼
                                    ▼                 Draining
                              RollingBack               ▼
                                    ▼                Promoting
                                 Failed                  ▼
                                                       Live → (Pruning)
```

Terminais: `Live`, `Stopped`, `Failed`, `Pruning`. Qualquer erro em qualquer
step dispara `RollingBack`. Cada transição é persistida no `states_log` do
deployment (`{from, to, at, message}`) e publicada como
`Event::DeployStateChanged`.

**`PreDeployCheck`** roda a fila `ServiceSpec.pre_deploy_checks()` em ordem e só
avança se todos passarem (exit code 0) — a primeira falha interrompe a fila e o
deploy. Fila vazia passa direto. A fila inteira roda dentro deste **único**
step, sem sub-estado por índice: o laço de `execute()` não impõe timeout por
step, e manter tudo num `PreDeployCheck` evita ter de persistir o índice. Ver
`docs/plano-pre-deploy-gate.md`.

**Recovery ao reiniciar**: o daemon busca todo deployment em estado não-terminal
e decide pelo estado — pré-swap (o container antigo ainda vive) é rollback
seguro; swap em curso exige inspecionar o que existe no Docker; `Promoting`
conclui; `RollingBack` conclui e marca `Failed`.

### Falha de deploy: onde a causa aparece

Ponto que já foi um bug e vale conhecer. Quando um step falha, a causa vai para
**quatro** canais:

1. o `build_log` (`==> Erro em [<Estado>]: …`, multi-linha quebrada em
   registros), **antes** da transição para `RollingBack` — é o único canal que a
   tela de log lê;
2. o `states_log` do deployment, na transição que entra em `RollingBack`;
3. o `Event::DeployStateChanged`, com `message` — consumido pela GUI e pela
   webui no toast e na notificação do SO;
4. o `ServiceStatus::Error(<causa>)` do serviço (primeira linha, truncada em 160
   caracteres).

Até 2026-08-27 só existiam (2), (3) e o log de tracing — nenhum deles chega ao
painel de log, e o usuário via o deploy morrer num `==> Deploy falhou` mudo. Ver
`docs/plano-erro-de-deploy-invisivel.md`.

## Pipeline de deploy, em detalhe

Deploy com fonte **Git**: clona o repo (`git2` dentro de `spawn_blocking`,
porque é `!Send`) → constrói a imagem Docker (contexto em tar, saída em
streaming como eventos `BuildLog`) → cria o container de staging → poll de
healthcheck (TCP/HTTP/DockerNative) → troca a rota no ingress → drena o
container antigo → renomeia o staging → `Live`.

Fonte **Registry** pula clone e build, indo direto ao pull. **Archive** (zip
enviado pelo cliente) entra no mesmo caminho do build. **Compose** sobe uma
stack inteira via `docker compose`.

O `Dockerfile` é conferido em `context.join(dockerfile_path)` — relativo ao
**contexto**, que é o nome dentro do tar mandado ao Docker, e não à raiz do
clone/zip — para as duas fontes que constroem imagem, antes de chamar o Docker.

Containers: `rp_<nome_do_serviço>_live` em produção,
`rp_<nome>_<deploy_id[:8]>_staging` em voo. Artefatos de build ficam em
`<db_path>/builds/<deployment_id>/` e somem na promoção ou no rollback.

**Healthcheck próprio, não o do Docker**: o do Docker tem resolução de intervalo
grosseira demais para minimizar a janela de swap. O poll inspeciona o container
(se parou, aborta na hora) e então faz HTTP (resolvendo o IP do container **na
rede do projeto**, não na default — container em várias redes tem vários IPs),
TCP, ou lê `health.status` no modo DockerNative.

## `rustploy-gui` (`crates/rustploy-gui/src/`)

UI declarada em templates de sintaxe XML (`views/*.gv` — tags XML, extensão
`.gv`, **não** `.xml`), renderizados pela crate publicada `glacier-ui`. Toda
responsabilidade de rede e de negócio (login, consumidor SSE, navegação, cada
mutação) vive em **Luau** (`views/scripts/`), **não** neste Rust — o `src/` daqui
é o runtime `iced::daemon`, a moldura da janela, a persistência local e a **API
de agente** (`src/agent/`, a única parte do Rust daqui que fala rede).

Rode da raiz do workspace (`cargo run -p rustploy-gui`) ou de um layout
empacotado: caminhos de template/script são relativos ao CWD que o glacier
resolve, não necessariamente ao diretório de lançamento.

- **`main.rs`** — entrada fina: `assets::locate_and_chdir()`, depois
  `app::run()`. Desde o glacier 0.36 o app roda sobre **`iced::daemon`**
  (multi-janela), não `iced::application`.
- **`assets.rs`** — localiza a base dos assets no boot e faz `chdir` para lá,
  para toda referência relativa resolver igual independente de como o app foi
  lançado. Ordem: `$RUSTPLOY_UI_ASSETS` → diretório do próprio executável
  (layout portátil/Windows) → `/usr/share/rustploy` (pacote Debian) → diretório
  atual (dev). Confirma a base sondando `crates/rustploy-gui/views/app.gv`. Só
  existe em debug: em release os assets são embutidos no binário
  (`embedded.rs`, `include_dir`) e o executável é standalone.
- **`app/mod.rs`** — desde o glacier **0.38**, apenas **configuração do
  `GlacierDaemon`**. O runner da lib cuida do loop `iced::daemon`, do
  motor-por-janela, das janelas-filhas, dos broadcasts entre elas, dos listeners
  globais e das ações `window:*` da titlebar borderless (tratadas contra o `Id`
  da janela em roteamento, **não** via `window::latest()` — no Wayland o
  round-trip perde o serial do pointer-grab e `window:drag` vira no-op
  silencioso). O que é específico do rustploy entra por ganchos:
  `.font()`/`.default_font()` (JetBrains Mono embutida), `.main_window()`
  (borderless, ícone, `min_size`, tamanho de primeiro lançamento,
  `exit_on_close_request: false`), `.child_window()` (filhas também borderless),
  `.main()` (registra `app.gv`, sobe a API de agente, define a tela),
  `.on_message()` (espelha sessão e contexto para a API de agente),
  `.remember_window_geometry(true)` e `.tray()`/`.on_tray()`.
- **Bandeja e ciclo de vida** (glacier **0.47+**, feature `tray`): fechar a
  última janela **recolhe para a bandeja** em vez de encerrar. Desde a **0.48**
  o motor da principal é **recolhido headless**, não descartado — SSE e login
  ficam vivos, então as notificações de deploy chegam com a janela fechada, e
  "Open Rustploy" religa a mesma sessão. Linux via libappindicator+GTK, Windows
  via message-loop Win32, macOS não suportado. Ver
  `docs/plano-tray-bandeja-e-ciclo-de-vida.md`.
- **Geometria da janela** (glacier **0.49+**, nativa): gravada **consultando o
  tamanho na hora** de fechar, não rastreada de eventos `Resized`/`Moved` — no
  handshake do xdg-shell no Wayland chega um `Resized` espúrio com o `min_size`,
  e um valor rastreado nasce envenenado com o mínimo. `window::position` é
  sempre `None` no Wayland (o protocolo não a expõe ao cliente; não é
  contornável), então só o tamanho volta lá. Ver
  `docs/plano-file-io-luau-e-geometria.md`.
- **`src/agent/`** — a API de agente da Parte 1. Servidor hyper em loopback numa
  thread com runtime tokio próprio; `on_message` é o caminho de ida (espelha
  sessão e contexto), o canal `external` do glacier **0.58.6+** é o de volta
  (`ExternalSender` injeta no motor o mesmo `EngineMessage` de um clique).
  hyper dos dois lados de propósito — nada de axum, nada de reqwest. Desenho em
  `docs/api-agente-no-gui.md`.
- **`views/`** (todos `.gv`) — `app.gv` (titlebar + handles de resize, chaveia
  em `screen`), `login.gv`, `shell.gv` (sidebar + topbar, chaveia em `view`),
  `home.gv` (Deployments/Projects/Monitoring/Ingress/Docker/Settings), 
  `service.gv` (detalhe do serviço, com suas sub-abas), `new_service.gv`
  (wizard), janelas separadas (`new_project_form.gv`, `log_window.gv`,
  `new_job_window.gv`, `new_registry_token_window.gv`) e `components/*.gv`.
  Estilizados por `views/styles/app.gss`, linkado globalmente do `app.gv` —
  janelas separadas precisam **relinká-lo**, porque cada janela é um motor
  isolado.
- **Multi-janela** (glacier 0.37+): `open_window{ file = …, data = {…} }` abre um
  motor Glacier próprio, que recebe a conexão via `data`; ele responde com
  `broadcast(evento, payload)` + `close_window()`, e o runner entrega o
  broadcast à principal, cujo `on_broadcast` atualiza a tela. É como o "Novo
  projeto" funciona.

## webui (`crates/daemon/webui/`)

Segundo cliente, servido pelo próprio daemon desde 2026-08-03: HTML + Alpine.js,
sem build step. `app.js` orquestra o boot do Alpine à mão (a ordem dos
`import` garante que os `Alpine.data`/`Alpine.store` estejam registrados antes
do `alpine:init`), `net/api.js` fala `POST /api/rpc`, `net/sse.js` consome os
streams — sem `EventSource`, que não permite mandar o header `Authorization`.
`screens/*.js` são as telas.

## Segurança

- Cada projeto tem uma rede bridge dedicada; containers de projetos diferentes
  não se enxergam. O proxy é o único ponto de entrada externo. Nada de
  `--privileged` nem capabilities extras por padrão.
- Secrets são cifrados em repouso com `age`; o plaintext nunca vai ao disco.
- `api.token` é um **bearer único, sem escopo**. Quem o tem cria projeto, lê
  secret decifrado (`resolve_env` devolve texto puro), sobe container e derruba
  stack. A API de agente herda exatamente esse alcance e não tem como inventar
  um escopo que o daemon não tem — se um agente vai operar isso de forma
  autônoma, o token com escopo (read-only / deploy / admin) precisa vir antes de
  ampliar o alcance, não depois. O gate de bearer é um `if` só, em
  `api/http_api.rs`: é o ponto único onde escopo entraria.

# Parte 4 — História

Decisões já tomadas e revertidas. Estão aqui porque cada uma delas parece uma
boa ideia para quem chega agora, e todas já foram tentadas.

| Foi | É | Por quê |
|---|---|---|
| SurrealDB embarcado (RocksDB) | **SQLite via `sqlx`** | write amplification e peso desproporcional para um daemon que mira < 50 MB |
| API Axum sobre Unix Domain Socket, corpo Bincode/Postcard | **HTTP/JSON + SSE**, um `POST /api/rpc` | o cliente deixou de ser local; JSON é auto-descritivo, então campo renomeado vira `nil` em vez de lixo decodificado |
| Rotas REST por recurso (`POST /projects`, `GET /services/:id`…) | um endpoint com o enum `Command` | um lugar só para autenticar, logar e versionar |
| TUI Ratatui como interface primária | **removido**; `rustploy-gui` (glacier-ui) | o TUI levou junto o CLI `apply`/`export`, que não tem substituto — `ManifestApply`/`ManifestExport` ficaram acessíveis só por API |
| "Não tem Web UI" | o daemon **serve uma webui** | toda feature de UI agora precisa entrar nos dois clientes |
| UI declarada em KDL | **XML + Luau** | — |
| `crates/rustploy-gui/src/app/` reimplementando o runtime `iced::daemon` (~250 linhas) | ganchos do builder do `GlacierDaemon` | o buraco era de API do glacier; foi fechado na 0.38 |
| Login lembrado e geometria da janela persistidos à mão em `app/store.rs` | `storage` do Luau + `remember_window_geometry` nativos | — |
| Auto-deploy por webhook marcado como "v2" | **implementado** | — |

Planos escritos e ainda **não** implementados vivem em `docs/plano-*.md` — o
cabeçalho de cada um diz o status. Notáveis: dependências entre serviços +
autostart no boot, e a integração do registry embutido com o executor de deploy.

Buracos conhecidos, todos com o porquê registrado em
`docs/plano-erro-de-deploy-invisivel.md`:

- **Log de runtime de container não é persistido.** O swap de deploy destrói o
  container antigo e o log dele morre junto — justamente o caso em que se quer
  o log é aquele em que ele já não existe. É o item de maior valor e maior
  esforço em aberto.
- **`ServiceUpdate` é substituição total, não patch.** Um round-trip mal feito
  apaga configuração sem aviso.
- **Não há schema do protocolo.** O catálogo em `GET /agent/schema` é curado à
  mão; gerar de verdade exigiria `schemars` sobre a crate `shared`.
- **Não há CLI.** O binário `rustploy` na raiz é um shell script de uma linha
  apontando para um alvo que não é mais construído.
