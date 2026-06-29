# remote-ui — plano de continuação

UI nova do client remoto, em **glacier-ui** (XML declarativo → iced), seguindo
`design/*.png`. Crate nova `crates/remote-ui` (binário `rustploy-remote-ui`),
sem tocar no `remote-client` antigo. Rodar da raiz do workspace:
`cargo run -p remote-ui` (paths de template são relativos ao CWD).

## Estado atual (funcionando)
- **glacier-ui 0.2.3** publicado no crates.io. 0.2.3: **widget `<TextArea>`**
  (editor multiline stateful). Como o `text_editor` do iced é stateful
  (`Content`), o `GlacierUI` mantém um `EditorMap` (binding→`Content`) +
  `editor_synced`; `sync_editors()` (fim de `reevaluate_all`) cria/recarrega o
  buffer a partir do contexto em mudança externa, e `XmlEditorAction` aplica a
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
  `XmlClick(String)` sem payload, e é template-processado). `Root` guarda
  `selected_service` e dispara `ctx.perform(net::fetch_service_detail)` que faz
  `ServiceGet` + `ProjectList` (p/ nome) + `LogsGet` (tail 200) → keys `svc_*`.
  Abas General/Connection/Environment/Healthcheck/Logs + painel lateral
  STATUS/UPTIME/SERVICES + LIVE OUTPUT. Template extraído em
  `templates/service.xml` (importado no `shell.xml`).
- **Abas do detail** (9 no remote-client; faltam só Patches): General/Connection/
  Environment/Domains/Deployments/Healthcheck/Logs/Advanced. Switch via `<if>`
  independentes (sem else-chain). Deployments usa `DeployHistory`; clicar
  "Build log" numa linha (`dep_logs:{id_full}`) → `net::fetch_build_logs`
  (`GetBuildLogs`, one-shot) preenche `dep_build_logs` e mostra o painel.
  Domains/Advanced são leitura do spec.
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
  mantêm as colunas alinhadas. Classes `.card`/`.grid`/`.card_*` no `.iss`.
- **Home screens** (`templates/home.xml`, importado no `shell.xml`; ifs
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
  `shell.xml` (switch `{view}`), `service.xml`, `home.xml`. Estilo: `app.iss` +
  `theme.json`. TODO layout fica nas classes `.iss` (não inline no XML).

## Próximos passos (em ordem)
1. **Formulários editáveis com Save** nas abas Domains/Healthcheck/Advanced
   (hoje leitura). Agora dá com `TextInput`/`TextArea` + `ServiceUpdate` (já há
   `net::run_env_op` como molde; generalizar p/ um spec-update). Falta tb a aba
   **Patches** do remote-client.
2. **Schedules**: sem backend ainda (placeholder). Settings só tem Web Server —
   o remote-client tem mais sub-telas (Git/Registry/Certs…), quase todas
   placeholder lá também.
   Topbar Deploy/Stop All ainda inertes (precisam de contexto/seleção global).
   Tabs Domains/Environment-edit do detail ainda faltam.
3. Sidebar com ícones SVG (já em `assets/icons/`). Botão gradiente real (hoje
   `<Button>` só cor sólida; gradiente já funciona em containers).
4. **Fonte mono** do design: JetBrains Mono está em `assets/fonts/` com o
   `.font()/.default_font()` COMENTADO no `main.rs`. Reabilitar quando
   descobrirmos por que a fonte custom sumia (provável: registrar a fonte e
   garantir que o iced a use; testar `WGPU_BACKEND=gl`).

## Armadilhas aprendidas
- **`width="fill"` dentro de `<Row>` sem `width="fill"`** (Row default=shrink)
  COLAPSA o filho → texto quebra 1 letra/linha (vira "invisível" e estica o
  pai). Regra: toda Row com filho fill precisa `width: fill`. (Foi o que
  quebrava o login — NÃO era fonte.)
- `parse_iss` é estrito: 1 propriedade desconhecida derruba a folha toda.
- **`<else>` só liga ao `<if>` imediatamente anterior** (irmão). Não dá pra
  encadear `if A / if B / else` esperando um switch — o `else` casaria com `B`
  e renderizaria junto com `A`. Para 3+ ramos, **aninhe**: `if A / else (if B /
  else …)`. (É como o switch `{view}` no `shell.xml` cresce.)
- **`ForEach` aninhado** funciona: um item-objeto cujo valor é array vira string
  JSON na chave `var.campo` (ex.: `r.cards`), e o `ForEach` interno aceita
  `items="r.cards"`. Útil p/ simular grid (sem widget de wrap no glacier).
- **`onClick` não tem payload** — vira `XmlClick(String)` e o `value` no
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

## Publicar nova versão do glacier
`cd glacier-ui && cargo build && cargo test && cargo publish -p glacier-ui --allow-dirty`
(token já configurado). Depois bump `glacier-ui = "0.2.x"` no `remote-ui/Cargo.toml`.

## Repos (ambos na branch main, commitar direto)
- glacier-ui: github.com/antoniofernandodj/xml-ui
- rustploy:   github.com/antoniofernandodj/rustploy
