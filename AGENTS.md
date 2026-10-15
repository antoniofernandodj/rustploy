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

> Desde 2026-10-13 o acesso de agente é servido **pelo próprio daemon**. A
> ponte local que morava no app GUI (`127.0.0.1:9800`, `agent-api.json`) foi
> removida. Desenho e decisões: `docs/plano-chave-de-acesso-e-mcp.md`.
> **Conectar o Claude Code de outra máquina** (comando pronto, prompt para o
> agente, problemas comuns): `docs/conectar-agente-mcp.md`.

**A fonte da verdade é o manual que o daemon serve** —
`rustploy-daemon/crates/daemon/src/api/agent/manual.md`, o mesmo texto de
`GET /api/agent/manual`, das `instructions` do MCP e do recurso
`rustploy://manual`. Esta parte só resume; não duplique o manual aqui.

## Em três passos

1. **Pegue o token de conexão** — base64 padrão (com `=`) de
   `host:porta::token`, sem esquema: endereço do daemon + token da API numa
   string só. Fica pronto no `config.toml` do daemon, em
   `[api].connection_token` (regravado a cada boot, logo abaixo do `token`);
   também sai de `rustployd access-key` e de Settings → Web Server → Copiar
   (GUI ou webui).
2. **Tire o token da API dele** — `echo '<token de conexão>' | base64 -d` dá
   `host:porta::token`, e o que vem depois do `::` já é o Bearer. Ou deixe o
   daemon fazer a troca:

   ```bash
   curl -s https://SERVIDOR:PORTA/api/login -d '{"key":"<token de conexão>"}'
   # → {"access_token":"…","token_type":"Bearer","ai_agent":{"mcp":"…/mcp","manual":"…/api/agent/manual"}}
   ```

   Formato errado → `400` dizendo qual parte falhou e qual é o formato certo.
   Token de outra instalação → `401`.
3. **Opere**, com `Authorization: Bearer <access_token>`:
   - **MCP** em `POST /mcp` (Streamable HTTP, respostas JSON):
     `claude mcp add --transport http rustploy https://SERVIDOR:PORTA/mcp --header "Authorization: Bearer <token>"`;
     (pronto, com o token, em Settings → Web Server → "Conectar um agente" →
     Copiar, na GUI e na webui; a topbar das duas mostra a qual servidor a
     sessão está ligada);
   - ou o mesmo MCP por `curl`, que é só JSON-RPC num POST;
   - ou `POST /api/rpc` direto com qualquer `Command`.

## As ferramentas MCP

`status` · `list_services` · `list_deploys` · `deploy` (dispara e **espera**,
devolvendo `ok`/`error`/fim do log) · `build_logs` (cursor) · `ingress_routes`
(diagnóstico de 502) · `ingress_reconcile` · `rpc` (qualquer `Command`) ·
`manual`. Implementação: `api/agent/tools.rs`, chamando `dispatch()` em
processo.

## O que mudou em relação à ponte da GUI

- O agente **não precisa** do app desktop aberto, nem estar na mesma máquina.
- O agente **vê o token** (é ele que autentica). O token continua sendo o
  bearer único e sem escopo do daemon — ver "Segurança" na Parte 3.
- As rotas que **dirigiam a janela** (`/agent/ui*`, `/agent/connect`) não
  existem mais: o daemon não tem janela, e o `rpc` cobre tudo o que é dado.
- Upload de zip: `POST /api/services/<id>/archive` com o corpo **binário**
  (não mais um caminho local).

## Por dentro

- `api/agent/access_key.rs` — o codec do token de conexão (base64 padrão de
  `host:porta::token`, o mesmo do `base64.b64encode` do Python; decodifica
  também url-safe e sem padding). Esconde o endereço, não o criptografa. A
  GUI (`views/scripts/access_key.luau`) e a webui (`net/api.js`) têm cópias
  do codec; as três geram bytes idênticos.
- **Os clientes não passam pelo `/api/login`**: GUI e webui decodificam o
  token de conexão e usam o token da API direto como Bearer. O esquema não
  está no token — a GUI sonda `GET /api/version` por https e depois por http
  (o de um servidor salvo vem primeiro); a webui usa a própria origem.
- `api/agent/mod.rs` — `/api/login` (público, antes do gate de bearer), para
  agentes. O token devolvido **é** o `[api].token`: não há sessões. As URLs de
  `ai_agent` usam o header `Host` e o esquema do listener (https quando há
  `[api].domain`). `write_connection_token` grava o `connection_token` no
  `config.toml` no boot (o serde ignora a chave desconhecida, então o parse
  tudo-ou-nada não quebra; não grava se o token veio de `RUSTPLOY_API_TOKEN`).
- `api/agent/mcp.rs` — MCP feito à mão sobre hyper (sem crate de MCP, sem SSE):
  `initialize`, `ping`, `tools/*`, `resources/*`. `GET /mcp` → `405`.
- O future do `dispatch` é enorme; aninhado em outro (o `Snapshot` despacha
  comandos por dentro) estoura a pilha da thread tokio em debug. Por isso
  `tools.rs` o chama com `Box::pin`.

------

# Parte 2 — Trabalhando no código

## Os três repos (este é o agregador `rustploy`)

O projeto são **três repositórios**, reunidos por este repo (o agregador) via git
submodules — a mesma ideia do `DocumentServer` do Euro Office:

```
rustploy/                     ← agregador (VOCÊ ESTÁ AQUI)
├── AGENTS.md, CLAUDE.md, GEMINI.md, Makefile, TODO.md, CONTINUE.md
├── docs/                     planos, relatórios e docs/indice/ (gerado)
├── tools/indexer/            gerador do índice (pacote cargo próprio, fora dos workspaces)
├── .github/workflows/        release.yml (checkout recursivo, builda os três)
├── rustploy-daemon/          daemon, importer, webui, packaging/, scripts/, install.sh
├── rustploy-gui/             cliente glacier-ui (views/, src/, assets/, packaging/)
├── rustploy-docs/            site de documentação (Astro Starlight), deploy na Cloudflare
└── rustploy-shared/          crate publicado no crates.io: modelos, protocolo, templates
```

- Clone com `git clone --recurse-submodules <agregador>`; sem a flag os três
  diretórios ficam vazios. O `Makefile` daqui roda nos três
  (`DAEMON_DIR`/`GUI_DIR`/`SHARED_DIR` mudam os nomes dos diretórios).
- **Mudou código num submodule?** Commit e push **no submodule** e, depois,
  commite aqui o novo ponteiro (`git add rustploy-daemon`…). Sem isso o agregador
  continua apontando para o commit antigo. Mexeu em algo que muda símbolos?
  `make index` e commite `docs/indice/` junto do ponteiro.
- **`rustploy-shared` não é `path`.** O daemon e a gui o consomem **do
  crates.io** (`version = "0.1.0"`), como o `glacier-ui`. Mudou o shared? Bump da
  versão → commit → `git push` → `cargo publish` (`--dry-run` antes) → subir a
  versão no `Cargo.toml` do daemon e da gui → `cargo update -p rustploy-shared`.
  Para **experimentar** sem publicar, use só na linha de comando (nunca commite):
  `cargo check --config 'patch.crates-io.rustploy-shared.path="../rustploy-shared"'`
  (a partir de `rustploy-daemon/` ou `rustploy-gui/`).
- Cada repo tem o seu `Cargo.lock`, o seu `[profile]` e o seu `target/`. O
  `[patch.crates-io]` do `iced_tiny_skia` mora no `Cargo.toml` da **gui**.
- O catálogo de templates (`templates/blueprints/`) é do shared, mas as imagens
  dos logos moram na gui (`assets/blueprint-logos/<id>/<arquivo>`); o shared só
  guarda o nome de cada uma em `templates/logos.txt` (o pacote estourava o limite
  de 10 MB do crates.io com elas).
- **Caminhos neste arquivo:** `docs/…` e `tools/…` são da raiz (agregador).
  `crates/daemon/…`, `crates/importer/…`, `webui/…`, `packaging/…` e `scripts/…`
  são de `rustploy-daemon/`. `views/…`, `assets/…` e o `src/…` da GUI são de
  `rustploy-gui/`; o `src/…` e `templates/…` do shared, de `rustploy-shared/`.

## Achar código: comece por `docs/indice/` (no agregador)

Antes de abrir arquivo ou sair dando `grep` por palavra-chave, leia o índice.
Ele foi feito para gastar pouco token: arquivos como `deploy/executor.rs` passam
de 20 mil tokens lidos inteiros, e o índice responde "onde está X" por uma
fração disso.

1. **`docs/indice/INDEX.md`** (os caminhos do índice já vêm com o repo na
   frente, ex. `rustploy-gui/views/…`): a árvore dos três repositórios, com uma linha de
   responsabilidade por arquivo, e qual índice de área cobre cada diretório.
2. **O índice da área** (`daemon-deploy.md`, `gui-handlers.md`, `webui.md`…):
   structs com campos, enums com variantes, métodos sob `impl Tipo`, funções
   Luau/JS e handlers dos `.gv`, cada um com a primeira frase da sua doc.
   Para seguir uma feature de ponta a ponta, **`comandos.md`** liga cada
   `Command` ao handler no daemon e a quem o chama na GUI, na webui e na API de
   agente, e lista as lacunas de paridade entre os dois clientes.
3. **`grep -n "fn nome" <arquivo>`** dá a linha, e aí se lê só o trecho
   (`offset`/`limit`), nunca o arquivo grande inteiro.

O índice **não tem número de linha, de propósito**: ele mudaria a cada edição.
Tudo é endereçado por caminho + nome, então o índice só envelhece quando um
símbolo é criado, renomeado ou apagado. Nesse caso, rode **`make index`** (ou
`cargo run -p indexer`) e commite `docs/indice/` junto com o ponteiro novo dos submodules. O indexador
lê o toplevel git de onde roda e falha se ele não for o agregador (sem `.gitmodules`). Ele gera e checa tudo
em memória antes de gravar: se um arquivo não parseia, se o `enum Command` some
ou se uma área fica vazia (diretório movido), ele sai com erro dizendo o quê e
**não altera nada**, para o índice nunca ficar pela metade. Os arquivos são gerados:
não edite à mão. A descrição de cada item vem do `//!`/`///` (ou do comentário
de cabeçalho, em Luau/JS/`.gv`/`.gvb` — neste, as linhas `//` do topo), então documentar o código melhora o índice.
O gerador fica em `tools/indexer/` (um pacote cargo avulso, sem workspace), e o
porquê de cada decisão está em `docs/plano-indice-de-codigo.md`.

## Convenções

### Copiar um serviço entre servidores (exportar/importar)

Dentro de um projeto, "Exportar serviço" (tela do serviço) gera um arquivo YAML
(`ServiceBundle`, `rustploy-shared/src/service_bundle.rs`) e "Novo serviço →
Importar" o recria em OUTRO servidor, sempre no projeto aberto, sem deploy e sem
sobrescrever nada. Comandos: `ServiceExportPlan`, `ServiceExport`, `ServiceImport`
(`dry_run` para pré-visualizar). Variáveis do projeto vão só as que o usuário
marca; `secret:NOME` nunca é decifrado. Desenho e o que NÃO atravessa:
`docs/plano-copiar-servico-entre-servidores.md`. Feature de UI: entrou na webui
(`screens/service_bundle.js`) **e** na GUI (`handlers/bundle*.luau`).

### Botão que dispara requisição fica travado até a resposta

Nos DOIS clientes, todo botão que gera uma requisição ao daemon é bloqueado no
clique e liberado quando a requisição termina — sucesso, erro na resposta ou
timeout (60 s no rpc, 10 min no upload de zip). O botão libera no PRIMEIRO toast
da própria ação (a resposta chegou), não no fim do handler: depois do toast os
handlers ainda releem serviço/snapshot, e esperar isso travaria o botão segundos
a mais. Não se faz à mão por botão:

- **webui**: `webui/busy.js` embrulha os métodos do `Alpine.store`/`Alpine.data`;
  se o método devolve Promise, o botão clicado fica `data-busy` até ela assentar
  (CSS em `app.css`). Handler novo só precisa ser `async`/devolver a Promise.
- **GUI**: `scripts/busy.luau` embrulha as funções globais listadas em
  `scripts/busy_actions.luau` (mantida por `tools/busy_actions.py --check`) e
  mantém `ctx.busy_<ação>`; o template liga `disabled = @busy_<ação>` no botão.
  Ação nova em `on_click`/`on_submit` → adicionar à lista e o `disabled`.

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
6. Subir a versão em `rustploy-gui/Cargo.toml` (repo da gui) para a recém-publicada.
7. `cargo check` no `rustploy-gui` para confirmar.

O passo 4 é o que evita a divergência do passo 0 — foi justamente pulá-lo que a
criou, duas vezes.

**Se algo parecer faltar no glacier**, o lugar de consertar é o builder do
`GlacierDaemon`, não um runtime paralelo dentro do `rustploy-gui`. Já houve um
runtime `iced::daemon` inteiro reimplementado aqui (~250 linhas) porque o
builder não expunha multi-janela; o buraco foi fechado na 0.38 e o runtime
local, removido.

### Todo template tem uma casca: `<screen>` ou `<component>`

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

- **`views/app.gvb`** — a raiz é o **`app(...)`** (glacier 0.117+): `id`,
  `single_instance`, `remember_geometry`, a janela **principal** (`size`,
  `min_size`, `decorations`, `icon`) e a `tray`. O `main_window_settings()` em
  `app/mod.rs` ficou só com o que o markup não descreve (`application_id`), e o
  builder não chama `.title()`/`.main_size()`.
- **As telas são filhas do `app`**: `screen(name = app, initial = true)` é a
  principal (inline, com o script `app.luau`) e as cinco janelas auxiliares são
  `screen(name = log | new_job | new_service | new_project | new_registry_token,
  src = "…gvb")`. A `screen` é **só conteúdo** (`title`; `size`/`icon` nela são erro
  de parse): o **tamanho** de cada janela filha vai na chamada dos handlers Luau —
  `open_window{ component = "log", size = "900 560", decorations = false, data = … }`
  (a moldura borderless também vai na chamada) —, e o ícone vem do `app`. Uma janela aberta numa tela do app registra
  **só aquela tela** (e as declarações globais: tema e `app.gss`), então o `init`
  das outras não roda nela. Duas exceções propositais em que só quem abre sabe o
  título: a janela de logs (título dinâmico — "Logs — nginx", "Build — abc123") e
  a edição de job, que sobrepõe o "Novo job" do arquivo.
- A **geometria lembrada** (`remember_window_geometry`) continua ganhando do
  `size` declarado: ele é o tamanho de *primeira* abertura.

Qual casca usar (glacier-ui 0.117+):

- **`app`** — só `views/app.gvb`: o manifesto (o aplicativo, a janela principal,
  a bandeja, o `resources` global e as telas).
- **`screen`** — os arquivos abertos como **tela de outra janela**
  (`new_*_window.gvb`, `new_project_form.gvb`, `log_window.gvb`), apontados por
  `screen(name = …, src = …)` no `app`. Levam só `title`.
- **`component`** — todo o resto, e é a maioria: `shell.gvb`, `home.gvb`,
  `service.gvb`, `login.gvb`, `new_service.gvb`, os dez de `views/components/` e
  os pedaços de `views/shell/`, `views/service/` e `views/home/`.
  Todos eles são importados por outro template (`link(rel = import, …)`), então
  não há janela a que título ou tamanho se aplicariam — e o `component` não
  aceita esses atributos, erra na cara explicando a diferença.

O `<resources>` agrupa o que não desenha (`<style>`, `<script>`, `<link>`,
`<import>`) e vale nas duas cascas; é opcional (um componente sem declaração
nenhuma leva só a casca). Um engano no cabeçalho é erro de parse — atributo
desconhecido, tamanho que não seja par de números, widget dentro do
`<resources>` — e não passa em silêncio. O teste
`janelas_declaram_titulo_e_tamanho`
(`tests/templates_render.rs`) trava os valores das janelas.

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

**Janelas na webui (`wm.js`).** O equivalente das janelas separadas da GUI
(Novo projeto, Novo serviço, Novo job, logs, token de registry, detalhe de
deploy) são janelas flutuantes: arrastáveis pela barra de título, redimensionáveis
(alça no canto), com foco por clique, maximizar (duplo clique ou ponto verde),
Esc fecha a de cima e posição/tamanho lembrados em `localStorage`
(`rustploy.win.<chave>`). Para criar uma: `modal_backdrop` (só a camada, não
bloqueia a página) > `modal_box` com `x-win="'chave'" data-w data-h` > `modal_head`
(título em `.label_cap` + um botão "Fechar", que a diretiva esconde e liga ao ponto
vermelho) + `modal_body`. Estado de abertura mora no store (`showNewJob`,
`projectWin`, `showNewService`, `logWin`…). `icons.js` traz os ícones SVG
(`x-icon="'nome'"`); `x-follow` (directives.js) faz o "tail -f" de logs. Para
desenvolver sem recompilar o daemon a cada edição, sirva `webui/` do disco com um
proxy para `/api` (ver `docs/plano-webui-janelas-e-polimento.md`).

### Nomes: tudo por extenso, em inglês, sem sigla

Vale para a GUI (Luau, `.gvb`) e para a webui (JS, `index.html`), e a regra é a
mesma nos dois lados: **o nome se explica sozinho, sem glossário**. Quem lê
`ctx.docker_cleanup_kinds` não precisa abrir outro arquivo para saber o que é;
com `ctx.dc_kinds` precisaria. Antes de criar um nome, confira se ele passa nas
regras abaixo.

1. **Sem sigla.** Palavra inteira ou composta: `service_form_healthcheck_retries`,
   não `f_hc_retries`; `message`, não `msg`; `delete`, não `del`.
2. **Inglês nos identificadores.** Só o texto mostrado ao usuário fica em
   português (`erro_`, `apontar`, `hora`, `limpa` não existem mais).
3. **O prefixo da tela fica sempre** nas chaves de `ctx` e nos handlers, mesmo
   dentro do arquivo da própria tela: o `ctx` é um espaço de nomes **plano** por
   janela, então `new_job_name` não encurta para `name`.
4. **Webui = o mesmo nome em camelCase** (`docker_cleanup_kind` →
   `dockerCleanupKind`). Se mudar um lado, mude o outro.
5. **Sufixos com significado fixo:** `*_count` (contagem), `*_label` (texto pronto
   para exibir), `*_message` (mensagem de status), `*_error` (erro),
   `is_*`/`has_*`/`can_*` (booleano), `*_id` (identificador — diga **de quê**:
   `project_id`, `provider_id`, nunca `pid`/`id` solto).
6. **Locais seguem a mesma ideia:** nada de `k`, `v`, `r`, `s`, `p`, `ok`, `res`.
   Laço = singular da coleção (`for _, project in ipairs(projects)`); par
   `ok`/`res` de uma chamada = nome da chamada (`service_get_ok`,
   `service_get_response`; no JS, `serviceGetResponse`); parâmetro de
   comparador = `left`/`right`. Índice de laço = `index` (ou algo mais específico).
   Exceção aceita: `_` para valor descartado, `M` para a tabela do módulo.

**Vocabulário dos prefixos** (use exatamente estes, não invente variantes):

| Prefixo | Assunto |
|---|---|
| `new_job_` · `new_service_` · `new_project_` · `new_registry_token_` | janelas/wizards de criação |
| `service_form_` | formulário do serviço (aba General e afins) |
| `service_` · `project_` · `deployment_` · `provider_` | dados do serviço / projeto / deploy / provedor |
| `git_provider_` | cadastro de GitHub/Gitea |
| `healthcheck_` | healthcheck |
| `import_` · `export_` | importar/exportar serviço (bundle) |
| `manifest_` | exportar/importar o manifesto (IaC) |
| `project_env_` · `project_secret_` | variáveis e segredos do projeto |
| `docker_cleanup_` | limpeza automática do Docker |
| `deploy_engine_` | tela da fila de deploys |
| `job_actions_` | painel "Ações do job" (aquecer, rodar, editar…) de uma linha de job |
| `log_window_` | janela de logs |
| `migration_` | migração de banco |
| `shared_database_` | banco do servidor compartilhado |
| `pre_deploy_check_` | gate de pré-deploy |
| `server_settings_` | Settings → Web |
| `host_` | CPU/memória/disco do host |

**Derivados que nascem do nome do handler** — renomeie os dois juntos:
`busy_<handler>` (trava de botão) e `error_<campo>` (o motor deriva de
`form_control`). `tools/busy_actions.py --check` confere a lista de handlers.

**Como renomear com segurança.** Nunca por busca-e-troca de texto. Para Luau,
confira com `luau-lsp analyze` em modo strict (pega nome inexistente) e rode
`cargo test` no `rustploy-gui`; para a webui, `cargo test -p rustploy web_ui`
(Chrome headless). Um renomeador de escopo (que não toca em campo de tabela
`x.nome`, chave de construtor `{ nome = … }` nem string) é o caminho; o histórico e
o mapa completo estão em `docs/plano-nomes-descritivos.md`.

### Luau

**Ferramental.** Type-check toda mudança antes de considerá-la pronta:

```bash
luau-lsp analyze --base-luaurc=.luaurc \
  --definitions=rustploy-gui/views/scripts/glacier.d.luau <arquivo(s)>
```

Não substitui `cargo test --test templates_render (no rustploy-gui)` (o runtime
`mlua` de verdade), mas pega erro de caminho de módulo e de tipo em segundos.
Para o VS Code, a extensão `johnnymorganz.luau-lsp` (config já versionada em
`.luaurc` + `.vscode/settings.json`). O `glacier.d.luau` **precisa** do
tratamento `--definitions=` / `luau-lsp.types.definitionFiles`: sem isso o
editor o interpreta como script comum e levanta ~40 erros falsos. Ver
`docs/luau-modularizacao-pacotes.md`.

**Convenção de `require`** (glacier-ui 0.22+, resolução relativa ao arquivo que
chama, como Node.js):

- irmão no mesmo diretório → nome nu: `require("stream")`;
- pacote pai a partir de dentro de `handlers/` ou `format/` → `require("../state")`;
- do script de entrada (`app.luau`, na raiz) → caminho completo:
  `require("handlers/connection")`.

**Convenção de sintaxe.** Quando o **único** argumento de uma chamada é um table
literal, use a forma sem parênteses — `f{ ... }`, não `f({ ... })` (idem
`toast{...}`, `api:rpc_checked{...}`, `open_window{...}`, `json.array{}`,
`os.time{...}`, `ipairs{...}`). Vale **só** para o único-argumento-table:
chamadas com mais de um argumento (`prune({...}, "msg")`,
`setmetatable({...}, mt)`) mantêm os parênteses. String literal única também
poderia dispensar parênteses, mas **não** adotamos essa forma.

### `rustploy-gui`: sem comentário de bloco, o porquê vai no README

O código do `rustploy-gui` é mantido **enxuto de propósito**. No **Rust**: nada de
comentário `//` e, nos itens, **uma frase** de `///`/`//!` (é a que o `make index` usa
como descrição). Nos **`.gvb`**: no cabeçalho de cada arquivo só a **primeira frase** (o
indexador a usa como descrição do arquivo) e, no corpo, no máximo comentários discretos de
uma linha (divisores `// ── Seção ──`, rótulos curtos) — nada de bloco explicativo de 3 ou
mais linhas, nem em `//` nem em `/* */` dentro do `<style>`. O contexto, o raciocínio e as
decisões descartadas moram no `rustploy-gui/README.md`, escrito como um artigo por assunto
(seção 6 para os templates). Ao mexer no código, não devolva comentários para ele: se algo
precisa de explicação, acrescente-a ao README (citando `arquivo.rs::item` ou
`views/x.gvb`) e deixe no código só a frase.

### Armadilhas de template (`.gv`)

- **Nunca escreva uma tag literal dentro de um comentário** (nem dentro de
  `<style>`): o parser quebra e o erro aponta para a linha errada.
- **Classes seguem BEM** (`bloco`, `bloco__elemento`, `bloco--modificador`, `u-*`
  para utilitário de uma propriedade) — a convenção completa, e onde um seletor
  composto simplifica, está no cabeçalho de `rustploy-gui/views/styles/app.gss`.
  O modificador vai **junto** do base no markup (`class = "btn btn--danger"`).
  Desde o glacier-ui 0.122 o `.gss` aceita `.a > .b` (filho direto), `.a .b`
  (descendente), `row.a` e `.a.b`; `+`, `~`, `[attr]`, `::x`, `:not()` e `:estado`
  num ancestral são **erro de parse**. Uso real: `.nav-item--active > .nav-item__icon`
  (nav_item.gvb) e `.state-cell--ok > text` (state_cell.gvb); o teste
  `nav_item_aceso_recolore_os_filhos_por_combinador` guarda o efeito.
- **O GSS não suporta seletor por vírgula** (`.a, .b { }`, nem dentro de
  `@media`): vira uma chave só, errada, e falha em silêncio. Uma declaração por
  seletor.
- **Condicional e laço são `<template>` ou atributo, nunca `<if>`/`<foreach>`.**
  As tags `<if>`/`<else-if>`/`<else>` viraram
  `<template if=>`/`<template else-if=>`/`<template else>` em todo `.gv`
  (2026-09-11; `<foreach>` já não existia) — mesmo `NodeType` no parser do
  glacier, e alinha com o `<template x-for>` da webui. Um
  `<template if="{x}" equals="y">` envolve **vários** filhos sem criar caixa;
  `if="{x}"` **como atributo** num elemento real condiciona **só aquele
  elemento**. Não reintroduza `<if>`/`<foreach>`.

### Dividir um template grande em arquivos (`.gvb`)

Um arquivo de view passa de ~300 linhas → divida. Todo `if @view/@tab == …` com
corpo maior que ~40 linhas vira arquivo. O padrão que `shell.gvb` e
`service.gvb` seguem: o arquivo grande fica só com o cabeçalho e o **roteador**
(os `if`/`else if` que escolhem o corpo), e cada ramo vira
`views/<area>/<nome>.gvb`, um `component { … }` importado por
`link(rel = import, href = "…", as = Nome)` e instanciado sem argumentos — ele
lê o estado global (`@view`, `@svc_*`…) como o resto. Foi assim que o
`shell.gvb` foi de 904 para ~200 linhas, o `service.gvb` de 1860 para ~315 e o
`home.gvb` de 1900 para ~80, sem mudar o comportamento. Os `component(name = SemX)`
de estado vazio (`fallback`) vão para o arquivo que os usa. O que aprendemos fazendo:

- **A condição vai na CHAMADA, nunca na raiz do arquivo extraído.** Um
  `column(if = @tab, equals = logs)` na raiz do `component` **não é aplicado** —
  o painel aparece em todas as abas, sem erro nenhum. Deixe a raiz sem `if` e
  escreva `ServiceLogsTab(if = @tab, equals = logs)` no ponto de uso (o atributo
  `if` na chamada funciona e preserva a estrutura da árvore). Se o ramo original
  era um `if @x == "y" { … }` com vários filhos, extraia o **corpo** do bloco:
  o `component` aceita várias raízes.
- **O `href` do `link(rel = import)` é relativo ao arquivo que importa**, não à
  raiz de `views/`: de `service.gvb` é `"service/logs.gvb"`, mas de dentro de
  `service/general.gvb` é só `"general_git.gvb"`.
- **Os nomes importados são globais.** Quem importou primeiro registra; os
  arquivos extraídos usam `StatCard`, `LoadingRow`, `StateCell`… sem reimportar.
  O `<style>` inline também é **global** (só `scoped="true"` restringe), então o
  CSS pode ficar no arquivo-mãe. Já os `component(name = SemX)` de estado vazio
  (`fallback`) migram para o arquivo que os usa.
- **O prefixo de dono das ações muda** (`ServiceDetail::open_logs_window` →
  `ServiceLogsTab::open_logs_window`). É inócuo: o dispatch só trata o prefixo
  como dono se ele for um componente registrado com script; senão cai na tela
  atual.
- **String com quebra de linha dentro** (placeholder de textarea com YAML, em
  `service/general_compose.gvb` e `new_job_window.gvb`): use o **`l"""…"""`**
  (glacier-ui 0.119+; `l` de *linhas*, não de raw: mantém as quebras, tira o recuo
  comum e continua interpolando `@nome`). Os outros dois jeitos não servem: `"…"`
  só aceita a quebra **literal** (não interpreta `\n`, e as linhas de continuação
  teriam de ficar coladas na margem), e `"""` dobra as linhas em espaços.
- **Prove que nada mudou.** O `templates_render` só checa `render().is_ok()`: ele
  não pega conteúdo que sumiu ou apareceu na aba errada (foi assim que o `if` na
  raiz passou despercebido). Antes de mover markup, despeje a árvore avaliada
  (`m.evaluated("app")`, `{:#?}`) de cada view/aba — um fator por vez: lista
  vazia/cheia, flags, `provider_tab`… — a partir do HEAD, repita depois e compare
  normalizando `node_id` (contador global) e o prefixo `Dono::`. O ideal é
  que seja **idêntico**. Cubra os estados vazios (`fallback`): os dados "de
  exemplo" não os exercitam.

### Descartar ≠ apagar

Código reutilizável que sai de uso vira comentário com `TODO` no lugar, não
deleção — o histórico do git existe, mas o contexto de "por que isso estava
aqui" se perde.

## Documentação pública (`rustploy-docs/`) e deploy

Quarto submódulo: o site (Astro 7 + Starlight, pt-BR, SSG), publicado na
Cloudflare Workers em `https://rustploy-docs.antoniofernandodj.workers.dev`.
O conteúdo escrito à mão fica em `src/content/docs/**.mdx`; `sync-manual`,
`gen-templates` e `gen-protocol` (rodados por `npm run build`) geram o resto —
páginas geradas não se editam à mão.

**Publicar** (de `rustploy-docs/`):

```bash
export PATH=$HOME/.nvm/versions/node/v24.3.0/bin:$PATH   # Astro 7 exige Node >= 22.12; o padrão da máquina é 20
npm run build          # gera dist/ (~25 s, ~446 páginas)
npx wrangler deploy    # sobe dist/ conforme wrangler.jsonc
```

- Sem o `wrangler.jsonc` o wrangler tenta `astro add cloudflare` e quebra o build.
- Depois: commit + push **no submódulo** e, no agregador, commite o novo
  ponteiro (`git add rustploy-docs`), como nos outros repos.
- Instalação documentada em `src/content/docs/instalacao/`: `cargo install
  rustploy` (daemon, binário `rustployd`) e `cargo install rustploy-gui`.

## Build & Run

Cada repo compila **sozinho** (o `rustploy-shared` vem do crates.io). Da raiz do
agregador, o `Makefile` roda nos três: `make build`, `make check`, `make test`,
`make fmt`, `make clippy` (`make help` lista o resto).

```bash
# Daemon (precisa do socket do Docker). `default-run = "rustployd"`, então o
# `--bin` só é necessário para o outro binário (`rustployd-fw`).
make dev-rustploy            # = cd rustploy-daemon && cargo run -p rustploy

# GUI (rode a partir do diretório do rustploy-gui: o debug lê views/ e assets/
# do disco, relativos ao CWD)
cd rustploy-gui && cargo run
```

**Testes.** No `rustploy-daemon` os diretórios são `crates/daemon` e `crates/importer`, mas
os **pacotes** se chamam `rustploy` e `rustploy-import` — `-p daemon` não
resolve. O daemon não tem lib target, então seus testes ficam sob `--bins`.

```bash
# em rustploy-daemon/: cargo test -p rustploy --bins <nome_do_teste>
#                       cargo check --workspace
# em rustploy-shared/:  cargo test
# em rustploy-gui/:     cargo test --bins
#                       cargo test --test templates_render   # runtime mlua real
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
silêncio**. Para conferir *comportamento* do daemon, use o MCP/`/api/rpc` da
Parte 1; Xvfb só para conferir a tela.

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
                  agentes de IA: POST /api/login + POST /mcp (Parte 1)
```

## Crates

| Crate | Binário | Papel |
|---|---|---|
| `shared` | — | Modelos, tipos do protocolo e structs de config, usados pelo daemon e pela GUI |
| `daemon` | `rustployd` | Servidor: API, banco, Docker, ingress, motor de deploy, registry |
| `rustploy-gui` | `rustploy-gui` | Cliente desktop glacier-ui (XML→iced). Toda a rede e lógica de negócio vive em **Luau** (`views/scripts/`), falando com o daemon pela API HTTP/JSON + SSE |
| `fw-helper` | `rustployd-fw` | Helper privilegiado de firewall (root, socket activation em `/run/rustploy/fw.sock`). O daemon pede allow/deny de portas externas (`daemon/src/firewall.rs`); o helper só aceita portas dentro da faixa `[external_ports]` e só fala com o ufw. **Sem dependência da crate `shared`, de propósito.** Ver `docs/relatorio-porta-externa-automatica.md` |
| `tools/indexer` | `indexer` | Gera `docs/indice/` (`make index`): o mapa de arquivos e símbolos da seção "Achar código" da Parte 2. Não é publicado nem empacotado |

Repos: `shared` é o repo `rustploy-shared`, `rustploy-gui` é o repo `rustploy-gui`;
`daemon`, `fw-helper` (bin `rustployd-fw`, dentro de `crates/daemon`) e o importer são
do repo `rustploy-daemon`; `tools/indexer` é do agregador.

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

`Command`, `Response` e `Event` vivem em `rustploy-shared/src/protocol.rs` e são
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
  nome gravado em `project.network_name`). Não há `volumes.rs`: o rustploy nunca cria
  volume nomeado, só bind mount (`ServiceSpec.volumes`).
- **`api/handlers/docker_inventory.rs`** — listagem do host inteiro para a aba
  Docker (`DockerImages`/`Volumes`/`Networks`/`Containers`), não só o que é do
  rustploy. Imagens e volumes vêm de **uma** chamada `docker system df` — o
  único endpoint do Docker Engine que já calcula contagem de uso de graça;
  networks são cruzadas à mão com `list_containers(all: true)`, porque o
  endpoint de listagem de networks nunca preenche o próprio campo `Containers`.
  Atribuição de projeto/serviço é melhor esforço: imagens por tag
  (`rp_<safe_name>:…` para builds Git, string exata para imagens de registry),
  networks pelo nome gravado em `project.network_name`; volumes não têm atribuição
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

## Ingress proxy: a tabela de rotas

Containers rustploy **não publicam porta no host** — quem fica na porta é o
proxy interno (`crates/daemon/src/ingress/`), e ele repassa para o IP do
container na rede do projeto. É o que torna o swap zero-downtime possível: a
troca é uma escrita atômica na tabela, não um rebind de porta. O desenho longo
está em `docs/ingress-proxy.md`; o TLS automático, em `docs/tls-acme.md`.

A tabela vive **em memória** (`ArcSwap`, lida sem lock pelo proxy), não no
banco. Consequências que valem lembrar:

- ela é escrita em três momentos: no fim de um deploy (`executor.rs`), no boot
  (`recovery::restore_routes`) e no reconcile de 30 em 30 segundos
  (`recovery::reconcile`);
- reiniciar o daemon a reconstrói a partir do Docker — não do banco;
- a ferramenta MCP `ingress_routes` mostra o que ela contém agora, e
  `ingress_reconcile` (`Command::IngressReconcile`) a recalcula sob
  demanda. Antes disso um 502 não tinha diagnóstico nem conserto sem redeploy.

### Armadilha: qual container de uma stack Compose recebe o domínio

Fonte **Git**/**Registry** tem um container por réplica, e a escolha é óbvia.
Uma stack **Compose** tem N containers e só **um** atende o domínio — num
Supabase self-hosted são nove, e quem serve a porta 8000 é o `kong`.

Até 2026-08-28 essa escolha era `find_by_prefix`, que literalmente "pega o
primeiro que encontrar" na ordem que o Docker devolveu. A rota do domínio caía
num container arbitrário da stack e o domínio respondia **502** — com TLS
válido, DNS certo e todos os containers no ar, que é o que tornava o sintoma
difícil de ler. Pior: a ordem não é estável, então o alvo podia mudar sozinho
entre dois boots.

Hoje a escolha é `containers::find_compose_ingress_container`, nesta ordem:

1. **`ingress_service` do `ComposeSource`** — o nome do serviço *dentro do
   arquivo compose* (`kong`). No manifesto é `source.compose_ingress`;
2. **quem expõe a porta que o domínio pede** (`spec.ingress_container_ports()`:
   a porta de cada domínio, mais a `port` do serviço quando há `host_port`).
   Resolve o caso comum sem configuração alguma;
3. **o primeiro por nome**, ordenado — determinístico, com `warn` no log.

Os três pontos que registram rota usam essa mesma função. Se o deploy e o
reconcile escolhessem alvos diferentes, o domínio mudaria de destino sozinho 30
segundos depois de subir.

## `rustploy-gui` (repo `rustploy-gui`, `src/`)

UI declarada em templates **`.gvb`** (markup de blocos: `tag(atributo = valor) { filhos }`,
ligação com `:`, glacier-ui 0.117+), renderizados pela crate publicada
`glacier-ui`. Toda
responsabilidade de rede e de negócio (login, consumidor SSE, navegação, cada
mutação) vive em **Luau** (`views/scripts/`), **não** neste Rust — o `src/` daqui
é o runtime `iced::daemon`, a moldura da janela e a persistência local. (A API
de agente que morava em `src/agent/` saiu em 2026-10-13 — ver Parte 1.)

Rode da raiz do repo `rustploy-gui` (`cargo run`) ou de um layout
empacotado: caminhos de template/script são relativos ao CWD que o glacier
resolve, não necessariamente ao diretório de lançamento.

- **`main.rs`** — entrada fina: `assets::locate_and_chdir()`, depois
  `app::run()`. Desde o glacier 0.36 o app roda sobre **`iced::daemon`**
  (multi-janela), não `iced::application`.
- **`assets.rs`** — localiza a base dos assets no boot e faz `chdir` para lá,
  para toda referência relativa resolver igual independente de como o app foi
  lançado. Ordem: `$RUSTPLOY_UI_ASSETS` → diretório do próprio executável
  (layout portátil/Windows) → `/usr/share/rustploy` (pacote Debian) → diretório
  atual (dev). Confirma a base sondando `views/app.gvb`. Só
  existe em debug: em release os assets são embutidos no binário
  (`embedded.rs`, `include_dir`) e o executável é standalone.
- **`app/mod.rs`** — desde o glacier **0.38**, apenas **configuração do
  `GlacierDaemon`**. O runner da lib cuida do loop `iced::daemon`, do
  motor-por-janela, das janelas-filhas, dos broadcasts entre elas, dos listeners
  globais e das ações `window:*` da titlebar borderless (tratadas contra o `Id`
  da janela em roteamento, **não** via `window::latest()` — no Wayland o
  round-trip perde o serial do pointer-grab e `window:drag` vira no-op
  silencioso). O que é específico do rustploy entra por ganchos:
  `.font()`/`.default_font()` (JetBrains Mono embutida), `.main_template()`
  (carrega o manifesto `app.gvb`), `.main_window()`/
  `.child_window()` (só o `application_id` do Linux), `.lua_extension()`, `.toast_period()` e `.antialiasing(false)`. O
  resto mora no **markup** do `views/app.gvb`: `app(id = rustploy,
  single_instance, remember_geometry, size, min_size, decorations, icon)`, a
  `tray` e as `screen`s. Um `.main(|motor| …)` só serve para registrar um
  `impl Component` em Rust (este app não tem); ele roda **depois** do manifesto.
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
- **`views/`** (todos `.gvb`) — `app.gvb` (o manifesto: `app(...)`, a titlebar +
  handles de resize da principal, que chaveia em `screen`), `login.gvb`,
  `shell.gvb` (sidebar + topbar + o roteador `if @view`; o corpo de cada view mora
  em `views/shell/`: `deployments`, `projects`, `project_services` e, dele, as
  sub-abas `project_env`/`project_secrets`/`project_jobs`), `home.gvb`
  (só o roteador `if @view` das telas globais: Monitoring/Ingress/Deploy Engine/
  Docker/Settings/Schedules; cada uma é um arquivo em `views/home/`, e o Docker e o
  Settings ainda se dividem por sub-aba em `views/home/docker/` e
  `views/home/settings/`), `service.gvb`
  (cabeçalho do serviço, a fileira de abas e o roteador `if @tab`; **cada aba é
  um arquivo em `views/service/`**, e a `general` ainda se divide em
  `general_git`/`general_zip`/`general_gitea`/`general_compose`),
  `new_service.gvb` (wizard), as telas das
  janelas separadas (`new_project_form.gvb`, `log_window.gvb`, `new_job_window.gvb`,
  `new_registry_token_window.gvb`, `new_service_window.gvb`) e `components/*.gvb`.
  Estilizados por `views/styles/app.gss`, linkado **uma vez** no `resources`
  global do `app` — as janelas separadas o recebem de graça (cada uma é um motor
  isolado que carrega as declarações globais do manifesto).
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
  stack. A API de agente (Parte 1) **entrega esse token** ao agente no
  `/api/login` — não há como inventar um escopo que o daemon não tem — se um agente vai operar isso de forma
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
| `rustploy-gui/src/app/` reimplementando o runtime `iced::daemon` (~250 linhas) | ganchos do builder do `GlacierDaemon` | o buraco era de API do glacier; foi fechado na 0.38 |
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
- **Não há schema do protocolo.** A lista de comandos do manual do agente
  (`api/agent/manual.md`) é curada à mão; gerar de verdade exigiria `schemars` sobre a crate `shared`.
- **Não há CLI.** O binário `rustploy` na raiz é um shell script de uma linha
  apontando para um alvo que não é mais construído.
