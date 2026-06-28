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
- **Login** (design img 5) e **Deployments** (img 3) prontos, com dados reais do
  daemon via polling RWP. Persistência de "remember server/token" (`store.rs`).
- Arquitetura: `Root` (Component único) detém estado + subscription de rede;
  `net.rs` faz polling (DaemonStatus/RecentDeployments/ProjectList) → `ContextPatch`;
  `rwp.rs` = transporte. Telas: `templates/app.xml` (switch `{screen}` login/shell),
  `login.xml`, `shell.xml` (switch `{view}`). Estilo: `styles/app.iss` + `theme.json`.
  TODO layout fica nas classes `.iss` (não inline no XML).

## Próximos passos (em ordem)
1. **Projects** (design img 2): grid de cards de serviço com nome, badge
   Running, CPU%, Memory. View `projects` no shell. Adicionar comando/polling
   de serviços por projeto. Card via classe `.card`.
2. **Service detail** (design img 4): header (nome + estado + botões Deploy/
   Reload/Rebuild/Stop), abas (General/Connection/Environment/Domains/
   Healthcheck/Logs/...), painel Source Provider/Build Engine, painel lateral
   UPTIME/SERVICES + LIVE OUTPUT (logs). Usar `view=service` + sub-`{tab}`.
3. **Ações reais**: ligar botões Deploy/Stop All/Stop/Reload a `ctx.perform`
   + `net::run_command` (já existe, hoje `#[allow(dead_code)]`). Precisa de
   seleção de serviço (clicar numa linha da tabela → `view=service`, guardar id).
4. **Monitoring/Schedules/Ingress/Docker/Settings/Support**: telas restantes.
5. Sidebar com ícones SVG (já em `assets/icons/`). Botão gradiente real (hoje
   `<Button>` só cor sólida; gradiente já funciona em containers).
6. **Fonte mono** do design: JetBrains Mono está em `assets/fonts/` com o
   `.font()/.default_font()` COMENTADO no `main.rs`. Reabilitar quando
   descobrirmos por que a fonte custom sumia (provável: registrar a fonte e
   garantir que o iced a use; testar `WGPU_BACKEND=gl`).

## Armadilhas aprendidas
- **`width="fill"` dentro de `<Row>` sem `width="fill"`** (Row default=shrink)
  COLAPSA o filho → texto quebra 1 letra/linha (vira "invisível" e estica o
  pai). Regra: toda Row com filho fill precisa `width: fill`. (Foi o que
  quebrava o login — NÃO era fonte.)
- `parse_iss` é estrito: 1 propriedade desconhecida derruba a folha toda.
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
