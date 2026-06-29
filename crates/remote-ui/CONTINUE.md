# remote-ui — plano de continuação

UI nova do client remoto, em **glacier-ui** (XML declarativo → iced), seguindo
`design/*.png`. Crate nova `crates/remote-ui` (binário `rustploy-remote-ui`),
sem tocar no `remote-client` antigo. Rodar da raiz do workspace:
`cargo run -p remote-ui` (paths de template são relativos ao CWD).

## Estado atual (funcionando)
- **glacier-ui 0.2.1** publicado no crates.io. Adições sobre a 0.1.2: widgets
  `Svg/Scrollable/Checkbox/Toggle/Rule`; atributos `font`/`gradient`/`textAlign`;
  ponte async (`ContextPatch`, `ctx.perform`, `dispatch -> iced::Task`,
  `Component::subscription` + `GlacierUI::subscription`); `Button` centraliza
  rótulo via `textAlign`.
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
- **Ações reais**: botões Deploy/Reload/Rebuild/Stop do detail ligados via
  `Root::service_action` → `net::run_service_action` (roda `DeployStart`/
  `ServiceReload`/`ServiceStop` e re-busca o detail; resultado em
  `svc_action_msg`). Falta ligar Deploy/Stop All do topbar (precisam de
  contexto/seleção global).
- **Projects**: grid de cards de serviço (nome, badge de status, CPU%, MEM).
  `net.rs` faz fan-out de `ServiceList` por projeto a cada poll; CPU/MEM vêm do
  stream de eventos (`ContainerMetrics`, publicado p/ todos os serviços rodando
  — basta `Subscribe { service_id: None }`), acumulados num `HashMap` e
  mesclados nos cards. glacier não tem grid com wrap → fatiei em linhas de 3
  (`GRID_COLS`) no `service_rows_json` (`[{"cards":[…]}]`) e renderizo com
  `<ForEach>` aninhado (`items="r.cards"`); fillers invisíveis (`filler="1"`)
  mantêm as colunas alinhadas. Classes `.card`/`.grid`/`.card_*` no `.iss`.
- Arquitetura: `Root` (Component único) detém estado + subscription de rede;
  `net.rs` faz polling (DaemonStatus/RecentDeployments/ProjectList) → `ContextPatch`;
  `rwp.rs` = transporte. Telas: `templates/app.xml` (switch `{screen}` login/shell),
  `login.xml`, `shell.xml` (switch `{view}`). Estilo: `styles/app.iss` + `theme.json`.
  TODO layout fica nas classes `.iss` (não inline no XML).

## Próximos passos (em ordem)
1. **Logs ao vivo no detail**: hoje `LIVE OUTPUT` é um tail one-shot (`LogsGet`)
   buscado ao abrir/agir. Ligar streaming real (`LogsSubscribe` no stream de
   eventos → acumular `Event::LogLine` por service e mesclar no `svc_logs`
   quando `selected_service` casar). Tabs Domains/Environment-edit ainda faltam.
2. **Monitoring/Schedules/Ingress/Docker/Settings/Support**: telas restantes.
   Topbar Deploy/Stop All ainda inertes (precisam de contexto/seleção global).
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
