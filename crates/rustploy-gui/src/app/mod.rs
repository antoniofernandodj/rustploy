//! Rustploy (glacier-ui) — desktop client whose UI is described in KDL
//! templates and rendered by the published `glacier-ui` engine. The network
//! layer runs through glacier-ui's async bridge (effects + subscriptions).

// Parte 2 da migração RWP→HTTP/Luau: a camada de rede Rust do GUI foi
// substituída por `<script>` Luau (templates/lib/app.luau + lib/net/api.luau +
// lib/fmt.luau). Estes módulos ficam COMENTADOS (não removidos) até o corte
// final — `root` (o Root monolítico), `net` (poll/view/RwpClient), `rwp` (o
// cliente do protocolo binário) e `wizard`. `store` continua (geometria/Prefs).
// TODO(corte-final): remover estes arquivos e o crate `shared::Rwp*`.
// mod net;
// mod root;
// mod rwp;
// mod wizard;
mod store;

use glacier_ui::{EngineMessage, GlacierUI};
use iced::{Element, Point, Subscription, Task, window, Size, window::settings::PlatformSpecific};
use std::time::Duration;

// TODO(corte-final): `connect_target`/`DEFAULT_RWP_PORT` eram do transporte RWP
// (host:port TCP). A normalização de URL agora vive em Luau (normalize_url em
// app.luau), que fala HTTP(S). Mantidos comentados enquanto `rwp.rs` existir.
// const DEFAULT_RWP_PORT: u16 = 8787;
//
// pub fn connect_target(url: &str) -> anyhow::Result<String> { /* ... */ }

/// App-level message: either an engine event from glacier-ui, the resolved id
/// of our (single) window (cached on startup so the custom titlebar can drive
/// window controls synchronously), the OS/WM asking to close that window, or
/// the geometry queried in response to that — see [`close_and_save`].
#[derive(Debug, Clone)]
pub(crate) enum Message {
    Engine(EngineMessage),
    WindowReady(Option<window::Id>),
    CloseRequested(window::Id),
    CloseWithGeometry(window::Id, Size, Option<Point>),
    /// TEMP DEBUG: see `subscription`'s `debug_events`.
    DebugIgnore,
}

pub(crate) struct App {
    motor: GlacierUI,
    /// Cached id of the main window. We resolve it once at boot instead of
    /// per-action: on Wayland, deferring window controls through a `get_latest`
    /// round-trip loses the pointer-grab serial, so `window:drag` silently
    /// no-ops. Handling them synchronously against the cached id makes the
    /// borderless titlebar actually draggable.
    window_id: Option<window::Id>,
}

impl App {
    pub(crate) fn boot() -> (Self, Task<Message>) {
        // Nota: as diretivas bare `else`/`senao` (sem valor) agora são
        // normalizadas pelo próprio glacier-ui (ver `eval.rs`), então não é mais
        // preciso registrá-las aqui — o antigo `register_bare_flags` saiu na 0.14.
        let mut motor = GlacierUI::new();
        if let Err(e) = motor.load_stylesheet("crates/rustploy-gui/styles/app.gss") {
            eprintln!("stylesheet: {e}");
        }
        // O componente "app" é o template app.xml com <script src="lib/app.luau">.
        // O glacier auto-liga um LuauComponent quando o template tem <script>, então
        // toda a lógica (login/navegação/SSE/ações) roda em Luau — sem Root Rust.
        if let Err(e) = motor.register_component("app", "crates/rustploy-gui/templates/app.xml") {
            eprintln!("register: {e}");
        }
        // Prefs de login (URL/token lembrados): o Luau não escreve arquivo, então
        // a persistência local fica em Rust. Aqui semeamos o contexto para o
        // formulário nascer preenchido; `persist_prefs` (em `update`) grava no
        // connect/toggle. Ver `store::Prefs`.
        seed_prefs(&mut motor);
        motor.set_initial_screen("app");
        (
            Self { motor, window_id: None },
            window::latest().map(Message::WindowReady),
        )
    }

    pub(crate) fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::WindowReady(id) => {
                self.window_id = id;
                Task::none()
            }
            // The OS/WM asked to close (Alt+F4, session end, …) — the
            // titlebar's own close button takes a separate path below since
            // it never raises this event (see `Engine` / `window:close`).
            Message::CloseRequested(id) => close_and_save(id),
            Message::DebugIgnore => Task::none(),
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
            Message::Engine(event) => {
                // Intercept the built-in `window:*` actions emitted by the
                // custom titlebar and run them against the cached window id
                // (see `window_id`). Everything else is a normal engine event.
                if let EngineMessage::UiClick(action) = &event {
                    if let (Some(cmd), Some(id)) =
                        (action.strip_prefix("window:"), self.window_id)
                    {
                        if cmd == "close" {
                            return close_and_save(id);
                        }
                        return window_control(id, cmd);
                    }
                }
                // Persistência de Prefs de login: no connect (o Luau já gravou
                // url/token no contexto antes de suspender no fetch) e nos toggles
                // "lembrar". Despacha primeiro, para o contexto refletir a ação,
                // depois grava. Ver `seed_prefs`/`persist_prefs`.
                let persist = matches!(&event,
                    EngineMessage::UiClick(a) | EngineMessage::UiSubmit { action: a, .. }
                        if a == "connect")
                    || matches!(&event,
                        EngineMessage::UiInputChanged { action: a, .. }
                            if a.starts_with("toggle_remember"));
                let task = self.motor.dispatch(&event).map(Message::Engine);
                if persist {
                    self.persist_prefs();
                }
                task
            }
        }
    }

    pub(crate) fn view(&self) -> Element<'_, Message> {
        self.motor
            .render_current()
            .unwrap_or_else(|e| iced::widget::text(e).into())
            .map(Message::Engine)
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let engine = Subscription::batch([
            self.motor.subscription(),
            GlacierUI::reload_subscription(Duration::from_millis(500)),
            GlacierUI::toast_subscription(Duration::from_millis(250)),
        ])
        .map(Message::Engine);
        let close_requests = window::close_requests().map(Message::CloseRequested);
        let debug_events = window::events().map(|(_id, _event)| {
            Message::DebugIgnore
        });
        Subscription::batch([engine, close_requests, debug_events])
    }

    pub(crate) fn theme(&self) -> iced::Theme {
        self.motor.theme()
    }

    /// Grava [`store::Prefs`] a partir do contexto atual: só guarda url/token
    /// quando o respectivo "lembrar" está ligado (senão limpa o campo). Chamado
    /// no connect e nos toggles (ver `update`).
    fn persist_prefs(&self) {
        let g = |k: &str| self.motor.get_data(k).cloned().unwrap_or_default();
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

/// Builds the initial window settings for `main`'s `iced::application(...)`
/// builder, restoring the last remembered size/position ([`store::WindowState`])
/// so the app reopens where it was left. Falls back to the default 1280×820 at
/// the platform-default placement on first launch, or when no position was
/// ever saved (e.g. Wayland, which never reports one to restore).
pub(crate) fn window_settings() -> window::Settings {
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
        // Borderless: the OS titlebar is replaced by a custom one defined in
        // `templates/app.xml` (drag region + minimize/maximize/close). The
        // `window:*` actions it emits are handled in `update` against the
        // cached window id (see `App::window_id`).
        decorations: false,
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
            Some(d) => {
                window::drag_resize(id, d)
            },
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
/// `templates/app.xml`.
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
