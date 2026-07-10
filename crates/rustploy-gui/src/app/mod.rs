//! Rustploy (glacier-ui) — desktop client whose UI is described in XML
//! templates and rendered by the published `glacier-ui` engine. Toda a lógica de
//! rede vive em Luau (`views/scripts/app.luau`), falando HTTP/JSON + SSE com o
//! daemon; este módulo Rust é só a casca da janela (chrome + persistência local).
//!
//! Desde glacier-ui 0.36 o modelo multi-janela roda sobre `iced::daemon` (só ele
//! indexa `view`/`title` por [`window::Id`]). Em vez do runner pronto
//! [`glacier_ui::GlacierDaemon`] — cujo builder não expõe as fontes, a janela
//! borderless, o ícone nem a persistência de geometria de que precisamos — a
//! casca aqui é um runtime `iced::daemon` **próprio**: um [`GlacierUI`]
//! independente por janela ([`Runtime::windows`]), a janela principal com todo o
//! chrome custom, e as janelas-filhas materializadas ao drenar
//! [`GlacierUI::take_pending_windows`] (o que faz `open_window(...)` na Luau /
//! `ctx.open_window(...)` no Rust abrirem janelas de verdade).

mod store;

use std::collections::HashMap;
use std::time::Duration;
use glacier_ui::{
    EngineMessage,
    GlacierUI,
    Element,
    Font,
    Point,
    Subscription,
    Task,
    window,
    Size,
    window::settings::PlatformSpecific,
    WindowSource,
    WindowSpec,
};

/// Fontes embutidas (JetBrains Mono): registradas no builder do daemon e usadas
/// como `default_font` de todas as janelas.
const FONT_REGULAR: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf");

/// Sobe o daemon multi-janela e roda o loop do iced até a última janela fechar.
/// Chamado por `main` depois de `assets::locate_and_chdir()`.
pub(crate) fn run() -> iced::Result {
    iced::daemon(boot, Runtime::update, Runtime::view)
        .title(Runtime::title)
        .theme(Runtime::theme)
        .subscription(Runtime::subscription)
        .font(FONT_REGULAR)
        .font(FONT_BOLD)
        .default_font(Font::with_name("JetBrains Mono"))
        .run()
}

/// Mensagem do daemon. Roteia eventos para o motor da janela certa (por `id`).
/// Toda variante é `Clone` — o iced 0.14 exige `Message: MaybeClone` no `run()`
/// do daemon — então nenhuma carrega um `WindowSpec`/`Box<dyn Component>`; a
/// janela nova é materializada em `update` (com o `Id` síncrono de
/// `window::open`), consumindo `take_pending_windows` logo após o `dispatch`.
#[derive(Debug, Clone)]
pub(crate) enum Message {
    /// Um [`EngineMessage`] destinado ao motor da janela `id`.
    Ui { id: window::Id, msg: EngineMessage },
    /// Uma janela terminou de abrir (retorno de `window::open`). Só informativo
    /// — o `Id` já foi registrado síncrono no `boot`/`open_child`.
    Opened,
    /// Uma janela foi fechada de fato (`window::close_events`): remove o motor
    /// e, se era a última, encerra o app.
    Closed(window::Id),
    /// A OS/WM pediu para fechar uma janela (`window::close_requests`, ANTES do
    /// fechamento). Na principal salvamos a geometria primeiro (ver
    /// [`close_and_save`]); nas demais fechamos direto.
    CloseRequested(window::Id),
    /// Geometria consultada em resposta a um `CloseRequested` da principal:
    /// persiste [`store::WindowState`] e então fecha a janela.
    CloseWithGeometry(window::Id, Size, Option<Point>),
    /// Tick periódico aplicado a **todas** as janelas (hot-reload de arquivos e
    /// expiração de toasts) — cada motor checa os próprios arquivos/toasts.
    TickAll(EngineMessage),
}

/// Estado do daemon: um motor por janela + seus títulos, e o `Id` da janela
/// principal (a que tem o chrome custom e a persistência de geometria/prefs).
pub(crate) struct Runtime {
    windows: HashMap<window::Id, GlacierUI>,
    titles: HashMap<window::Id, String>,
    /// Id da janela principal, conhecido já no `boot` (o `window::open` devolve
    /// o `Id` de imediato). Guardá-lo evita um round-trip `latest()` por ação:
    /// no Wayland, adiar controles de janela por esse round-trip perde o
    /// pointer-grab serial e `window:drag` vira no-op silencioso — tratá-los
    /// síncronos contra o id cacheado é o que torna a titlebar arrastável.
    main_id: window::Id,
}

/// `boot` do iced: constrói o motor principal e abre a janela inicial com todo o
/// chrome custom. `window::open` devolve o `Id` de imediato, então já inserimos
/// o motor em `windows` com essa chave e o guardamos como `main_id`.
fn boot() -> (Runtime, Task<Message>) {
    let mut motor = GlacierUI::new();
    if let Err(e) = motor.register_component("app", "crates/rustploy-gui/views/app.xml") {
        eprintln!("register: {e}");
    }
    seed_prefs(&mut motor);
    motor.set_initial_screen("app");

    let (id, open) = window::open(main_window_settings());
    let mut rt = Runtime {
        windows: HashMap::new(),
        titles: HashMap::new(),
        main_id: id,
    };
    rt.titles.insert(id, "Rustploy".to_string());
    rt.windows.insert(id, motor);
    (rt, open.map(|_| Message::Opened))
}

impl Runtime {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Opened => Task::none(),
            Message::Closed(id) => {
                self.windows.remove(&id);
                self.titles.remove(&id);
                if self.windows.is_empty() {
                    iced::exit()
                } else {
                    Task::none()
                }
            }
            // A OS/WM pediu para fechar (Alt+F4, botão da WM, fim de sessão). Na
            // principal salvamos a geometria antes; nas filhas fechamos direto
            // (o `close_events` subsequente removerá o motor). O botão de fechar
            // da titlebar custom NÃO passa por aqui — vem como ação `window:close`
            // interceptada em `route`.
            Message::CloseRequested(id) => {
                if id == self.main_id {
                    close_and_save(id)
                } else {
                    window::close(id)
                }
            }
            Message::CloseWithGeometry(id, size, position) => {
                store::WindowState {
                    width: size.width,
                    height: size.height,
                    x: position.map(|p| p.x),
                    y: position.map(|p| p.y),
                }
                .save();
                window::close(id)
            }
            Message::TickAll(msg) => {
                // Aplica o tick a cada janela (clonando a mensagem por janela).
                let ids: Vec<window::Id> = self.windows.keys().copied().collect();
                let tasks: Vec<_> =
                    ids.into_iter().map(|id| self.route(id, msg.clone())).collect();
                Task::batch(tasks)
            }
            Message::Ui { id, msg } => self.route(id, msg),
        }
    }

    /// Despacha `msg` ao motor da janela `id` e, em seguida, abre quaisquer
    /// janelas que aquele motor tenha pedido durante o `dispatch`. Intercepta
    /// antes as ações `window:*` da titlebar custom da janela principal (contra
    /// o id cacheado — ver [`Runtime::main_id`]).
    fn route(&mut self, id: window::Id, msg: EngineMessage) -> Task<Message> {
        // Controles de janela da titlebar borderless (só a principal os tem).
        // Tratados síncronos contra o id cacheado para manter o pointer-grab
        // serial vivo no Wayland; o motor também os trataria (via `latest()`),
        // mas por um round-trip que perderia o serial do drag.
        if id == self.main_id {
            if let EngineMessage::UiClick(action) = &msg {
                if let Some(cmd) = action.strip_prefix("window:") {
                    if cmd == "close" {
                        return close_and_save(id);
                    }
                    return window_control(id, cmd);
                }
            }
        }

        // Persistência de Prefs de login (o formulário vive na janela principal):
        // no connect (o Luau já gravou url/token no contexto antes de suspender
        // no fetch) e nos toggles "lembrar". Despacha primeiro, para o contexto
        // refletir a ação, depois grava. Ver `seed_prefs`/`persist_prefs`.
        let persist = id == self.main_id && should_persist(&msg);

        // 1. despacha ao motor da janela (borrow escopado)
        let ui_task = match self.windows.get_mut(&id) {
            Some(engine) => engine.dispatch(&msg).map(move |m| Message::Ui { id, msg: m }),
            None => return Task::none(),
        };
        if persist {
            self.persist_prefs();
        }
        let mut tasks = vec![ui_task];

        // 2. drena os pedidos de janela nova desse motor e abre cada um
        let pending = self
            .windows
            .get_mut(&id)
            .map(|e| e.take_pending_windows())
            .unwrap_or_default();
        for spec in pending {
            tasks.push(self.open_child(spec));
        }
        Task::batch(tasks)
    }

    /// Materializa um [`WindowSpec`] (de `take_pending_windows`) numa janela
    /// nova: constrói um motor fresco, abre a janela (o `Id` vem síncrono) e
    /// registra motor + título. `Named` já vem resolvido para `File` pelo motor
    /// de origem, então [`build_engine`] só trata `Component`/`File`.
    fn open_child(&mut self, spec: WindowSpec) -> Task<Message> {
        let WindowSpec { source, title, size, resizable } = spec;
        let (engine, fallback_title) = build_engine(source);
        let (w, h) = size.unwrap_or((640.0, 480.0));
        let settings = window::Settings {
            size: Size::new(w, h),
            resizable,
            ..window::Settings::default()
        };
        let (id, open) = window::open(settings);
        self.titles.insert(id, title.unwrap_or(fallback_title));
        self.windows.insert(id, engine);
        open.map(|_| Message::Opened)
    }

    fn view(&self, id: window::Id) -> Element<'_, Message> {
        match self.windows.get(&id) {
            Some(engine) => match engine.render_current() {
                Ok(elem) => elem.map(move |msg| Message::Ui { id, msg }),
                Err(e) => iced::widget::text(e).into(),
            },
            None => iced::widget::text("").into(),
        }
    }

    fn title(&self, id: window::Id) -> String {
        self.titles.get(&id).cloned().unwrap_or_else(|| "Rustploy".to_string())
    }

    fn theme(&self, id: window::Id) -> iced::Theme {
        self.windows.get(&id).map(|e| e.theme()).unwrap_or(iced::Theme::Dark)
    }

    fn subscription(&self) -> Subscription<Message> {
        // Listeners globais de evento, registrados UMA vez no daemon (drag-end
        // p/ reorder, Tab p/ foco, resize p/ `@media`): reimplementados aqui
        // porque o glacier só os expõe via `GlacierDaemon::run()` — as variantes
        // de `EngineMessage` que produzem são públicas. Usam o `window::Id` que
        // o callback recebe para rotear ao motor certo.
        let mut subs = vec![
            iced::event::listen_with(|e, s, id| {
                drag_end_from_event(e, s, id).map(|msg| Message::Ui { id, msg })
            }),
            iced::event::listen_with(|e, s, id| {
                tab_focus_from_event(e, s, id).map(|msg| Message::Ui { id, msg })
            }),
            iced::event::listen_with(|e, s, id| {
                viewport_from_event(e, s, id).map(|msg| Message::Ui { id, msg })
            }),
            window::close_events().map(Message::Closed),
            window::close_requests().map(Message::CloseRequested),
            GlacierUI::reload_subscription(Duration::from_millis(500)).map(Message::TickAll),
            GlacierUI::toast_subscription(Duration::from_millis(250)).map(Message::TickAll),
        ];

        // Subscriptions por-motor (streams `sse`/`websocket`): marcadas com o
        // `id` da janela via `.with(id)` (o `map` do iced exige closure não
        // capturante). Streams já vêm isolados por `engine_id` no glacier.
        for (id, engine) in &self.windows {
            subs.push(
                engine
                    .subscription()
                    .with(*id)
                    .map(|(id, msg)| Message::Ui { id, msg }),
            );
        }
        Subscription::batch(subs)
    }

    /// Grava [`store::Prefs`] a partir do contexto da janela principal: só guarda
    /// url/token quando o respectivo "lembrar" está ligado (senão limpa o campo).
    /// Chamado no connect e nos toggles (ver `route`).
    fn persist_prefs(&self) {
        let motor = match self.windows.get(&self.main_id) {
            Some(m) => m,
            None => return,
        };
        let g = |k: &str| motor.get_data(k).cloned().unwrap_or_default();
        let remember_url = g("remember_url") == "true";
        let remember_token = g("remember_token") == "true";
        store::Prefs {
            remember_url,
            remember_token,
            url: if remember_url { Some(g("url")) } else { None },
            token: if remember_token { Some(g("token")) } else { None },
        }
        .save();
    }
}

/// Decide se a ação deve disparar a persistência das Prefs de login. O
/// `login.xml` é IMPORTADO em `app.xml` (`<link rel=import as=Login>`), então as
/// ações chegam com namespace do owner (`Login::connect`,
/// `Login::toggle_remember_url`); comparamos só o sufixo (após `::`).
fn should_persist(msg: &EngineMessage) -> bool {
    let action = match msg {
        EngineMessage::UiClick(a) => a.as_str(),
        EngineMessage::UiSubmit { action, .. } => action.as_str(),
        EngineMessage::UiInputChanged { action, .. } => action.as_str(),
        _ => return false,
    };
    let bare = action.rsplit("::").next().unwrap_or(action);
    bare == "connect" || bare.starts_with("toggle_remember")
}

/// Semeia o contexto do glacier com as Prefs de login salvas, para o formulário
/// nascer preenchido. Os nomes de chave batem com os `formControl`/`checked` do
/// `login.xml` (`url`/`token`/`remember_url`/`remember_token`).
fn seed_prefs(motor: &mut GlacierUI) {
    let prefs = store::Prefs::load();
    motor.define_data("remember_url", if prefs.remember_url { "true" } else { "false" });
    motor.define_data("remember_token", if prefs.remember_token { "true" } else { "false" });
    if let Some(url) = prefs.url.filter(|_| prefs.remember_url) {
        motor.define_data("url", &url);
    }
    if let Some(token) = prefs.token.filter(|_| prefs.remember_token) {
        motor.define_data("token", &token);
    }
}

/// Constrói um [`GlacierUI`] novo para uma janela-filha a partir da sua fonte, e
/// devolve também o título de fallback (nome do componente). `Named` já deve ter
/// sido resolvido para `File` no motor de origem (por isso aqui é erro).
fn build_engine(source: WindowSource) -> (GlacierUI, String) {
    let mut engine = GlacierUI::new();
    let title = match source {
        WindowSource::Component(comp) => {
            let name = comp.name().to_string();
            if let Err(e) = engine.register(comp) {
                eprintln!("open_window: falha ao registrar componente: {e}");
            }
            engine.set_initial_screen(&name);
            name
        }
        WindowSource::File(path) => {
            let name = file_stem(&path);
            if let Err(e) = engine.register_component(&name, &path) {
                eprintln!("open_window: falha ao carregar '{path}': {e}");
            }
            engine.set_initial_screen(&name);
            name
        }
        WindowSource::Named(name) => {
            eprintln!("open_window: fonte 'Named({name})' não resolvida; janela vazia");
            name
        }
    };
    (engine, title)
}

/// Nome de componente derivado do caminho de um arquivo (o stem, sem extensão).
fn file_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("janela")
        .to_string()
}

/// Queries the window's *actual current* size and position (a fresh
/// `window::size`/`window::position` round-trip, not a value cached from past
/// resize/move events) and persists it before closing.
///
/// Querying fresh — rather than tracking `Event::Resized`/`Moved` — sidesteps
/// a real bug we hit: on this Wayland setup, an early spurious `Resized` event
/// during the window's xdg-shell configure handshake reported the `min_size`
/// (1000×680) rather than the actual requested/rendered size, so a tracked
/// value got permanently poisoned to the minimum before the user ever touched
/// the window. Asking "what is the size right now" at the moment of closing
/// has no such staleness window. `window::position` legitimately returns
/// `None` on Wayland (the protocol never exposes window position at all) —
/// that's not fixable here, so `WindowState.x`/`.y` just stay unset.
fn close_and_save(id: window::Id) -> Task<Message> {
    window::size(id).then(move |size| {
        window::position(id).map(move |position| Message::CloseWithGeometry(id, size, position))
    })
}

/// Builds the main window's settings, restoring the last remembered size/position
/// ([`store::WindowState`]) so the app reopens where it was left. Falls back to
/// the default 1280×820 at the platform-default placement on first launch, or
/// when no position was ever saved (e.g. Wayland, which never reports one to
/// restore). Borderless (`decorations: false`) — the OS titlebar is replaced by
/// a custom one in `views/app.xml`; its `window:*` actions are handled in
/// [`Runtime::route`]. `exit_on_close_request: false` routes the WM's own close
/// through `close_requests` so geometry is saved before the window closes.
fn main_window_settings() -> window::Settings {
    let ws = store::WindowState::load();
    let min = Size::new(480.0, 680.0);
    let position = match (ws.x, ws.y) {
        (Some(x), Some(y)) => window::Position::Specific(Point::new(x, y)),
        _ => window::Position::Default,
    };
    window::Settings {
        size: Size::new(ws.width.max(min.width), ws.height.max(min.height)),
        position,
        min_size: Some(min),
        // Taskbar / dock icon while the app runs (Windows taskbar, X11 dock).
        // Embedded so it works regardless of CWD; on Wayland the dock icon
        // instead comes from the `.desktop` file matched by app id, see the
        // Debian package assets in `Cargo.toml`.
        icon: window::icon::from_file_data(
            include_bytes!("../../assets/rustploy.png"),
            None,
        )
        .ok(),
        decorations: false,
        // Não fecha sozinho no pedido da WM: `close_requests` nos deixa salvar a
        // geometria em `close_and_save` antes de fechar de fato.
        exit_on_close_request: false,
        // `application_id` only exists on the Linux (X11/Wayland) variant of
        // `PlatformSpecific`; other platforms expose different fields, so the
        // whole block is gated per target to keep the Windows build compiling.
        platform_specific: platform_specific(),
        ..Default::default()
    }
}

#[cfg(target_os = "linux")]
fn platform_specific() -> PlatformSpecific {
    PlatformSpecific {
        application_id: "rustploy-gui".to_string(),
        ..Default::default()
    }
}

#[cfg(not(target_os = "linux"))]
fn platform_specific() -> PlatformSpecific {
    PlatformSpecific::default()
}

/// Maps a `window:<cmd>` action to its iced window task, driven against the
/// known window id (so drag/resize keep the live pointer-grab serial on
/// Wayland — a deferred `latest()` round-trip would lose it). `resize:<dir>`
/// starts an interactive border/corner resize (`drag_resize`).
fn window_control(id: window::Id, cmd: &str) -> Task<Message> {
    if let Some(dir) = cmd.strip_prefix("resize:") {
        return match resize_direction(dir) {
            Some(d) => window::drag_resize(id, d),
            None => Task::none(),
        };
    }
    match cmd {
        "minimize" => window::minimize(id, true),
        "maximize" | "toggle_maximize" => window::toggle_maximize(id),
        "close" => window::close(id),
        "drag" => window::drag(id),
        _ => Task::none(),
    }
}

/// Parses a resize-handle direction token (`se`, `e`, `s`, …) into the iced
/// window `Direction`. Mirrors the tokens used by the resize handles in
/// `views/app.xml`.
fn resize_direction(s: &str) -> Option<iced::window::Direction> {
    use iced::window::Direction::*;
    Some(match s {
        "n" => North,
        "s" => South,
        "e" => East,
        "w" => West,
        "ne" => NorthEast,
        "nw" => NorthWest,
        "se" => SouthEast,
        "sw" => SouthWest,
        _ => return None,
    })
}

// ── Mapeadores de evento globais (reimplementados) ───────────────────────────
// O glacier 0.36 moveu esses listeners para dentro de `GlacierDaemon::run()` e
// não os expõe publicamente; como rodamos nosso próprio `iced::daemon`, os
// reimplementamos aqui a partir das variantes públicas de `EngineMessage`.

/// Botão esquerdo solto em qualquer lugar → fim de um drag de reorder em curso.
fn drag_end_from_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: window::Id,
) -> Option<EngineMessage> {
    match event {
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left)) => {
            Some(EngineMessage::DragEnd)
        }
        _ => None,
    }
}

/// Tab / Shift+Tab → move o foco entre widgets focáveis. Os text inputs do iced
/// não avançam o foco no Tab sozinhos, então o motor o dirige.
fn tab_focus_from_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: window::Id,
) -> Option<EngineMessage> {
    use iced::keyboard::{key::Named, Event as Kbd, Key};
    match event {
        iced::Event::Keyboard(Kbd::KeyPressed { key: Key::Named(Named::Tab), modifiers, .. }) => {
            Some(if modifiers.shift() { EngineMessage::FocusPrev } else { EngineMessage::FocusNext })
        }
        _ => None,
    }
}

/// Resize da janela → [`EngineMessage::Viewport`], para os blocos `@media`
/// reavaliarem contra o novo tamanho.
fn viewport_from_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: window::Id,
) -> Option<EngineMessage> {
    match event {
        iced::Event::Window(window::Event::Resized(size)) => {
            Some(EngineMessage::Viewport { width: size.width, height: size.height })
        }
        _ => None,
    }
}
