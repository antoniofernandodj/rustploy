# rustploy-gui — plano de continuação

UI nova do client remoto, em **glacier-ui** (KDL declarativo → iced), seguindo
`design/*.png`. Crate nova `crates/rustploy-gui` (binário `rustploy-gui`),
sem tocar no `remote-client` antigo. Rodar da raiz do workspace:
`cargo run -p rustploy-gui` (paths de template são relativos ao CWD).

## ⚠️ PENDENTE — teste manual (não feito ainda)
A validação automática é só headless (`tests/templates_render.rs`: parse + render
da árvore, sem display). **Falta testar em runtime contra um daemon** — não dá
pra capturar tela aqui (Wayland). Rodar `cargo run -p rustploy-gui`, conectar e
conferir:
- **Saves** persistem de fato (re-fetch após `ServiceUpdate`): General (editar
  branch/imagem), Domains, Healthcheck, Advanced, Settings (web server).
- **Env**: adicionar/remover var, editar/Importar `.env`, Exportar `.env`.
- **Logs ao vivo**: aba Logs / LIVE OUTPUT (runtime) e Build log (deployment em
  andamento) crescendo em tempo real; "Copiar tudo" e seleção/Ctrl+C.
- **Ações**: Deploy/Reload/Rebuild/Stop, Stop All do topbar (agora `Command::StopAllManaged`,
  ver seção "Docker: inventário + limpeza" abaixo — confirmar que realmente para todo
  serviço, mesmo um com status desatualizado no banco).
- **Telas**: Monitoring (métricas host+container), Ingress, Docker (as 4 sub-abas:
  Containers/Images/Volumes/Networks — nunca rodei contra um daemon real com imagens/
  volumes/networks de verdade pra conferir os dados e os botões "Limpar sem uso").
- **Timer de deploy** (badge "⏱ Ns" no header do serviço): confirmar visualmente que
  tickar 1s/2s/3s… funciona e que trava certo no sucesso/falha.
- **Busca do topbar**: confirmar que filtra Deployments/Projects/Docker em tempo real.
- **Tamanho/posição da janela**: redimensionar, fechar pelo X, reabrir — size deve
  bater; position fica sempre em branco no Wayland (ver "Armadilhas aprendidas").

## Estado atual (funcionando)
- **glacier-ui atualmente na 0.4.3** (subiu de 0.3.1 desde a última vez que esta seção
  foi escrita — ver changelog logo abaixo pras versões intermediárias). Principais
  mudanças 0.4.x:
  - **0.4.3**: `secure`/`password` (flags nuas do `TextInput`) viraram built-in do
    framework — não precisam mais ser registrados pelo app.
  - **0.4.2 → 0.4.3**: correção do pré-processador KDL — um flag nu (`secure`, `else`)
    numa linha de continuação própria (não colada no nó) virava um nó-filho espúrio que
    engolia as propriedades seguintes (`value`/`onChange`/`class`). Flags de aplicação
    (não intrínsecas a um widget) precisam ser registrados via
    `glacier_ui::register_bare_flags([...])` — em `rustploy-gui` isso é feito no topo de
    `App::boot()` (`["else", "senao"]`).
  - Loading gate na aba **General** do detalhe do serviço: enquanto o fetch (spec +
    contas/repos/branches Gitea) não completa, mostra "Carregando…" em vez dos
    `<Select>` piscarem vazios antes de terem opções.
  - **Timer de deploy ao vivo**: `Deploy`/`Rebuild` armam um `DeployTrack` (id do
    deployment + `started_at` do servidor); um badge "⏱ Ns" no header do serviço
    atualiza a cada segundo (tick de 1Hz local, sem RPC) enquanto o deploy roda; ao
    chegar num estado terminal (detectado no tick de 2s já existente), o badge some e
    o resultado final ("deploy concluído em Xs" / "deploy falhou após Xs · ESTADO")
    aparece colorido (verde/vermelho) onde antes só tinha `svc_action_msg` cinza.
    `Reload` não dispara o timer (não é um deploy — só stop/start do container).
  - **Tamanho/posição da janela lembrados**: `store::WindowState` persistido em
    `~/.local/share/rustploy/rustploy-gui-window.json`. `main.rs` chama
    `app::window_settings()` (lê o JSON) **antes** de o iced criar a janela; o
    salvamento consulta `window::size`/`window::position` **na hora exata do
    fechamento** (`app.rs::close_and_save`, via `Task::then` encadeado), tanto no botão
    "X" da titlebar quanto num `CloseRequested` do SO/WM (`exit_on_close_request(false)`
    habilitado pra isso). Ver "Armadilhas aprendidas" abaixo pro porquê de NÃO rastrear
    via `Event::Resized`/`Moved` acumulados.
  - **Aba Docker com sub-abas Containers/Images/Volumes/Networks**: as 3 novas listam
    todo o host Docker (não só recursos do Rustploy), com "EM USO"/"SEM USO" por linha e
    botão "Limpar sem uso" cada. Daemon: `docker_inventory.rs` novo (`docker system df`
    pra imagens/volumes com contagem de uso de graça; networks cruzadas manualmente
    contra `list_containers`). Ver `AGENTS.md` §17 pro detalhe completo (fonte dos
    dados, como a atribuição de projeto/serviço é inferida, o que não é possível pra
    volumes).
  - **Topbar**: busca (`search_changed`) agora filtra de verdade
    (Deployments/Projects/Docker, case-insensitive, ao vivo) — antes só capturava o
    texto sem efeito nenhum. Botão "Deploy" removido (só navegava pra Projects). "Stop
    All" virou 1 RPC (`Command::StopAllManaged`) que para todo serviço do Rustploy
    reaplicando a lógica real de `service_stop` por serviço, sem depender do status
    atual no banco — mas **continua restrito a containers com label
    `rustploy.managed=true`**, nunca mexe em containers de fora do Rustploy no mesmo
    host.
  - Todas essas mudanças (exceto a fixação do glacier-ui em si) só existem no
    working tree — **não foram testadas contra um daemon rodando de verdade** (ver
    "PENDENTE — teste manual" no topo deste arquivo).
- **glacier-ui 0.3.1** publicado no crates.io (iced **0.14**). 0.3.1: atributo
  universal **`onDoubleClick`** (duplo-clique → ação; usado na titlebar para
  maximizar/restaurar via `onDoubleClick="window:maximize"`). 0.3.0 (breaking,
  iced 0.14): **resize de janela interativo** (`window:resize:<dir>` →
  `window::drag_resize`) + atributo universal **`cursor`** (mostra o ícone de
  redimensionar no hover via `mouse::Interaction`). 0.2.9: **controles de janela
  built-in** (`window:minimize`/`maximize`/`close`/`drag`) + atributo universal
  **`onPress`** (envolve em `mouse_area`, dispara no pressionar — base do arraste).
  - **Janela sem borda** (`decorations:false` no `main.rs`): a **titlebar
    customizada** (região de arraste `on_press="window:drag"` + botões `—`/`▢`/`✕`)
    e uma **moldura de 6px de alças de resize** (bordas/cantos com `cursor` +
    `on_press="window:resize:<dir>"`) ficam em `views/app.xml`. Os controles
    `window:*` são tratados no `main.rs` **contra o `window::Id` cacheado** (no
    boot via `window::latest`): no Wayland, adiar via `latest()` perde o serial
    do grab e o drag/resize não pegam. iced 0.14: `iced::application(boot, …)
    .title(…).run()`; `Subscription::run_with(key, fn(&key)->stream)` (chave
    `PollKey`, Hash só do `seq`); `stream::channel` com `Sender` anotado.
- **glacier-ui 0.2.7**: `<TextInput secure="true">`
  (mascara senhas/tokens). 0.2.6: **widget `<Select>`** (dropdown `pick_list`,
  opções de array JSON do contexto, estilizável via `.gss`; aliases
  `Dropdown`/`PickList`/`ComboBox`/`Seletor`). 0.2.4: ação built-in
  `clipboard:<key>` (copia valor do contexto p/ a área de transferência).
  0.2.3: **widget `<TextArea>`**
  (editor multiline stateful). Como o `text_editor` do iced é stateful
  (`Content`), o `GlacierUI` mantém um `EditorMap` (binding→`Content`) +
  `editor_synced`; `sync_editors()` (fim de `reevaluate_all`) cria/recarrega o
  buffer a partir do contexto em mudança externa, e `UiEditorAction` aplica a
  edição e espelha o texto de volta no contexto. 0.2.2: **fix `if/else`/`ForEach`
  aninhados dentro de `ForEach`** (deixava o grid Projects invisível). 0.2.0:
  widgets `Svg/Scrollable/Checkbox/Toggle/Rule`; atributos `font`/`gradient`/
  `textAlign`; ponte async (`ContextPatch`, `ctx.perform`, `dispatch -> Task`,
  `Component::subscription` + `GlacierUI::subscription`).
- **Login** (img 5), **Deployments** (img 3), **Projects** (img 2) e
  **Service detail** (img 4) prontos, com dados reais do daemon via polling RWP.
  Persistência de "remember server/token" (`store.rs`).
- **Service detail** (`view=service`, sub-`{tab}`): clicar "Open" num card →
  `open_service:<id>` (id codificado na própria ação, pois `onClick` vira
  `UiClick(String)` sem payload, e é template-processado). `Root` guarda
  `selected_service` e dispara `ctx.perform(net::fetch_service_detail)` que faz
  `ServiceGet` + `ProjectList` (p/ nome) + `LogsGet` (tail 200) → keys `svc_*`.
  Abas General/Connection/Environment/Healthcheck/Logs + painel lateral
  STATUS/UPTIME/SERVICES + LIVE OUTPUT. Template extraído em
  `views/service.xml` (importado no `shell.xml`).
- **Abas do detail** (9 no remote-client; faltam só Patches): General/Connection/
  Environment/Domains/Deployments/Healthcheck/Logs/Advanced. Switch via `<if>`
  independentes (sem else-chain). Deployments usa `DeployHistory`; clicar
  "Build log" numa linha (`dep_logs:{id_full}`) seleciona o deployment
  (`selected_deploy_shared`) → stream faz seed (`GetBuildLogs`) + acumula
  `Event::BuildLog` **ao vivo** (ring `BUILD_RING=2000`) em `dep_build_logs`.
  Cada painel de log tem "Copiar tudo" (`clipboard:` + `*_text` plano).
- **Formulários editáveis** (Domains/Healthcheck/Advanced): campos `f_*`
  populados no fetch, inputs com `onChange="field:<key>"` (handler genérico que
  faz `ctx.set`), Toggle TLS, seletor de kind (`hckind:<k>`), Save → `SpecOp`
  (`net::run_spec_op`: `ServiceGet` → muta spec → `ServiceUpdate` → re-fetch).
  **General** (source/build) também é editável: form genérico Git/Registry
  (repo_url/image, branch, port, user/credentials, build path, watch paths,
  submodules, dockerfile/context/stage) → `SpecOp::General` reconstrói o
  `ServiceSource` (mantém Git se já era ou se a URL parece repo via
  `looks_like_git_url`, senão Registry; preserva `provider_id`).
- **Provider sub-abas Git | Gitea no General** (feito, fiel ao remote-client):
  `prov_tab` (`git`/`gitea`) controla um sub-tab bar no topo do form General; a
  aba **Gitea só aparece quando há provider conectado** (`<if cond="{gitea_count}"
  not_equals="0">`). `open_service` dispara `fetch_git_providers` (popula
  `gitea_count`/`gitea_providers`); `fetch_service_detail` define `prov_tab` =
  `gitea` quando o source já tem `provider_id` (e ecoa `gitea_provider_id`).
  - **Git**: form de URL/imagem crua (o de antes).
  - **Gitea**: APENAS seleciona contas já conectadas (cadastro/conexão é em
    Settings → Git). Três `<Select>` (dropdowns) — conta (`gitea_providers`,
    onChange `gitea_provider_pick` → `GitRepoList`), repositório (`gitea_repos`,
    onChange `gitea_repo_pick`: `find_repo` resolve clone_url/default_branch do
    JSON, preenche `f_repo_url`/`f_branch` → `GitBranchList`) e branch
    (`gitea_branches`, onChange `field:f_branch`). Sem botão de OAuth aqui. +
    campos de build.
  - **Widget `<Select>` (glacier 0.2.6)**: dropdown `pick_list` que lê um array
    JSON do contexto (mesmo formato do ForEach), com `labelField`/`valueField`,
    valor selecionado via chave, emite `onChange` com o valor escolhido;
    estilizável via `.gss` (classe `.select`: background/border/color). Antes era
    lista de botões (feio); agora são selects de verdade, como no remote-client.
  - `gen_save` (ambas as abas) lê `prov_tab`: Gitea passa `gitea_provider_id`,
    Git passa vazio → em `apply_spec_op` vazio = `provider_id: None` (desvincula),
    preenchido = vincula. `prov:<git|gitea>` troca a sub-aba (entrar em Gitea com
    conta já escolhida recarrega repos).
  - **Auto-load**: ao abrir um serviço já gitea-bound, `fetch_service_detail`
    pré-carrega `gitea_repos` (+ `gitea_branches` do repo casado por `clone_url`).
    `gitea_providers`/`gitea_count` são buscados no connect (poll_stream) e em
    `open_service`. Daemon: `GitRepoList` exige token OAuth (`usable_token`);
    erros legíveis via `resp_msg` (`erro: <code>: <message>`).
- **Settings → Git** (cadastro/conexão de contas, sub-aba ao lado de Web Server;
  `settings_tab` = `web`/`git`): lista contas conectadas (`gitea_providers`:
  nome · base_url · método · @login) com "Remover" (`gp_delete:<id>` →
  `GitProviderDelete`); formulário "Conectar conta Gitea" — Nome, Base URL,
  Método (`gp_mode` oauth/pat), e por método: OAuth (Client ID, Client Secret
  `secure`, Redirect URI copiável `{gp_redirect}` = `{domain}/oauth/gitea/callback`)
  ou PAT (token `secure`). "Conectar" (`gp_connect` → `net::git_provider_connect`:
  `GitProviderCreate`; se OAuth dispara `GitOAuthStart` e abre o navegador via
  `xdg-open`; PAT já fica usável) e "Atualizar lista" (`gp_refresh`). Campos
  `secure` usam o `<TextInput secure="true">` do glacier 0.2.7.
- **Validação headless**: `tests/templates_render.rs` registra os templates a
  partir da raiz do workspace e renderiza login + todas as views + todas as abas
  do service (incl. editor `.env` e painel de build log) — pega KDL malformado e
  propriedade `.gss` desconhecida sem precisar de display. (`cargo test -p rustploy-gui`.)
- **Copiar / selecionar**: glacier 0.2.4 tem ação `clipboard:<key>`. Aba Logs é
  um `<TextArea value="svc_logs_text">` (selecionável/Ctrl+C) + "Copiar tudo";
  Connection tem "Copiar" por valor (`clipboard:svc_port` etc.).
- **Edição de env vars**: aba Environment tem (a) form Adicionar (KEY+value) e ✕
  por linha → `net::run_env_op` (`ServiceGet` → muta `env_vars` → `ServiceUpdate`
  → re-fetch); (b) **editor `.env`** colapsável: botão `.env`/`Fechar .env`
  (`env_text_open`) revela um `<TextArea value="svc_env_text">` com botões
  Importar (`EnvOp::ImportDotenv`: parse `KEY=VALUE`, `#` comentário,
  `<secret:NAME>` volta a Secret, **substitui todas**) e Cancelar (descarta via
  `svc_env_text_orig`, cópia pristina salva no fetch); e "Exportar .env" grava
  `~/<svc>.env`. "Adicionar" com KEY existente substitui (= editar).
- **Ações reais**: botões Deploy/Reload/Rebuild/Stop do detail ligados via
  `Root::service_action` → `net::run_service_action` (roda `DeployStart`/
  `ServiceReload`/`ServiceStop` e re-busca o detail; resultado em
  `svc_action_msg`). Falta ligar Deploy/Stop All do topbar (precisam de
  contexto/seleção global).
- **Logs ao vivo** (LIVE OUTPUT): o daemon (`logs.rs::stream_loop`) já faz tail
  de todos os containers rodando e publica `Event::LogLine` no bus, sem precisar
  de `LogsSubscribe` — então a conexão de eventos (`Subscribe None`) já recebe.
  `Root` compartilha `selected_service` com o stream via `Arc<Mutex<String>>`
  (sem reiniciar a subscription ao trocar de serviço). O stream mantém um ring
  buffer por serviço (`LOG_RING=200`), faz seed do histórico (`LogsGet`) quando
  a seleção muda, e a cada `LogLine` do serviço selecionado re-emite `svc_logs`.
  Sai do detail (`nav_*`) → limpa a seleção e para de emitir.
- **Projects**: grid de cards de serviço (nome, badge de status, CPU%, MEM).
  `net.rs` faz fan-out de `ServiceList` por projeto a cada poll; CPU/MEM vêm do
  stream de eventos (`ContainerMetrics`, publicado p/ todos os serviços rodando
  — basta `Subscribe { service_id: None }`), acumulados num `HashMap` e
  mesclados nos cards. glacier não tem grid com wrap → fatiei em linhas de 3
  (`GRID_COLS`) no `service_rows_json` (`[{"cards":[…]}]`) e renderizo com
  `<ForEach>` aninhado (`items="r.cards"`); fillers invisíveis (`filler="1"`)
  mantêm as colunas alinhadas. Classes `.card`/`.grid`/`.card_*` no `.gss`.
- **Home screens** (`views/home.xml`, importado no `shell.xml`; ifs
  independentes por `{view}`): **Monitoring** (stat cards de host CPU/MEM/DISK/
  LOAD via `Event::SystemMetrics` + tabela por container via `ContainerMetrics`),
  **Ingress** (rotas derivadas dos serviços com domínio), **Docker** (containers
  derivados dos serviços: imagem/estado/container id), **Settings** (web server
  do daemon: `GetDaemonSettings` no connect → `ss_domain`/`ss_email` editáveis →
  `settings_save` → `SetDaemonSettings`). Schedules/Support são placeholders.
- **Topbar**: `Deploy` leva ao grid Projects (seleção); `Stop All` →
  `net::stop_all` (para todos Running/Degraded).
- Arquitetura: `Root` (Component único) detém estado + subscription de rede;
  `net.rs` faz polling (DaemonStatus/RecentDeployments/ProjectList+ServiceList) →
  `ContextPatch`; settings buscado 1x no connect (não no poll, p/ não sobrescrever
  edição). `rwp.rs` = transporte. Telas: `app.xml` (switch `{screen}`), `login.xml`,
  `shell.xml` (switch `{view}`), `service.xml`, `home.xml`. Estilo: `app.gss` +
  `theme.json`. TODO layout fica nas classes `.gss` (não inline no KDL).

## Próximos passos (em ordem)
1. **Aba Patches** do remote-client; **Schedules** sem backend (placeholder).
   Settings só tem Web Server (remote-client tem Git/Registry/Certs… quase todas
   placeholder lá também).
2. Sidebar com ícones SVG (já em `assets/icons/`). Botão gradiente real (hoje
   `<Button>` só cor sólida; gradiente já funciona em containers).
3. **Fonte mono** do design: JetBrains Mono está em `assets/fonts/` com o
   `.font()/.default_font()` COMENTADO no `main.rs`. Reabilitar quando
   descobrirmos por que a fonte custom sumia (provável: registrar a fonte e
   garantir que o iced a use; testar `WGPU_BACKEND=gl`).

## Armadilhas aprendidas
- **`width="fill"` dentro de `<Row>` sem `width="fill"`** (Row default=shrink)
  COLAPSA o filho → texto quebra 1 letra/linha (vira "invisível" e estica o
  pai). Regra: toda Row com filho fill precisa `width: fill`. (Foi o que
  quebrava o login — NÃO era fonte.)
- `parse_gss` é estrito: 1 propriedade desconhecida derruba a folha toda.
- **`<else>` só liga ao `<if>` imediatamente anterior** (irmão). Não dá pra
  encadear `if A / if B / else` esperando um switch — o `else` casaria com `B`
  e renderizaria junto com `A`. Para 3+ ramos, **aninhe**: `if A / else (if B /
  else …)`. (É como o switch `{view}` no `shell.xml` cresce.)
- **`ForEach` aninhado** funciona: um item-objeto cujo valor é array vira string
  JSON na chave `var.campo` (ex.: `r.cards`), e o `ForEach` interno aceita
  `items="r.cards"`. Útil p/ simular grid (sem widget de wrap no glacier).
- **`onClick` não tem payload** — vira `UiClick(String)` e o `value` no
  `update` é sempre `None` em cliques. Para passar dado (ex.: id da linha),
  codifique na própria ação: `onClick="open_service:{c.id}"` (a ação é
  template-processada no eval) e faça `action.strip_prefix("open_service:")` no
  `update`. Mesmo padrão p/ `tab:<nome>`.
- `Column`/`Row` não têm `onClick`; só `Button` clica. Card clicável = um
  `<Button>` dentro do card (ou o card inteiro vira Button text-only).
- Não consigo screenshot (Wayland sem grim; `import`/D-Bus negados) — depender
  do usuário enviar imagem.
- Disco do `/` enche fácil; `target/` do glacier chegou a 19G. `cargo clean` lá
  se faltar espaço.
- **Não confiar em `Event::Resized`/`Moved` acumulados pra saber "o tamanho/posição
  atual da janela" no Wayland/GNOME deste ambiente**: um `Resized` espúrio chega logo no
  início (durante a negociação xdg-shell da janela) reportando o `min_size` em vez do
  tamanho real — se você guarda isso num campo e usa o último valor visto ao fechar, o
  valor salvo fica preso no mínimo pra sempre, mesmo que o usuário tenha redimensionado
  de verdade depois. A correção foi trocar por uma consulta ativa
  (`window::size(id)`/`window::position(id)`, via `Task::then` encadeado) bem no momento
  do fechamento — pergunta "qual é o tamanho AGORA" em vez de confiar em histórico de
  eventos. Também: GNOME faz snap automático (estica pra altura cheia da tela) quando a
  largura pedida bate com a largura da tela — não é bug do app, é o WM reagindo a uma
  janela "encostada na borda".
- **Fechar de verdade requer `exit_on_close_request(false)` no builder do
  `iced::application`** — sem isso, o clique em "X" (que já chama `window::close`
  diretamente) funciona, mas um `CloseRequested` vindo do SO/WM (Alt+F4, sessão
  encerrando) fecha a janela **sem rodar nenhum código do app antes** — não dá pra
  interceptar pra salvar estado nesse caminho por padrão.
- Testar via `timeout N ./binário` só prova que não deu panic — `timeout` manda SIGTERM,
  que NÃO passa pelo `CloseRequested`/clique no X, então nada que dependa de um
  fechamento "de verdade" (como salvar geometria da janela) roda nesse teste. Pra validar
  de fato: rodar em background (`nohup ... &`), pedir pro usuário interagir com a janela
  real na tela dele (mesma máquina/display), e inspecionar os arquivos/logs depois.

## Publicar nova versão do glacier
`cd glacier-ui && cargo build && cargo test && cargo publish` (token já
configurado; o working dir precisa estar limpo — se houver arquivo solto não
relacionado, `git stash` antes ou `--allow-dirty`). Depois bump
`glacier-ui = "0.2.x"` no `rustploy-gui/Cargo.toml`.

## Repos (ambos na branch main, commitar direto)
- glacier-ui: github.com/antoniofernandodj/xml-ui
- rustploy:   github.com/antoniofernandodj/rustploy
