//! Rustploy (glacier-ui) — desktop client whose UI is described in KDL
//! templates and rendered by the published `glacier-ui` engine. The network
//! layer runs through glacier-ui's async bridge (effects + subscriptions).

mod net;
mod root;
mod rwp;
mod store;
mod wizard;

use glacier_ui::{EngineMessage, GlacierUI, ToastSpec};
use iced::{Element, Point, Subscription, Task, window, Size, window::settings::PlatformSpecific};
use std::time::Duration;
use root::Root;

const DEFAULT_RWP_PORT: u16 = 8787;

/// Parses an `rwp://`/`rwps://` URL into the `host:port` target for the TCP
/// connection, filling in the default port when omitted.
pub fn connect_target(url: &str) -> anyhow::Result<String> {
    let a = url.trim();
    let authority = match a.split_once("://") {
        Some((scheme, rest)) => {
            anyhow::ensure!(
                scheme.eq_ignore_ascii_case("rwp") ||
                scheme.eq_ignore_ascii_case("rwps"),
                "esquema não suportado: {scheme}:// — use rwp:// ou rwps://"
            );
            rest
        }
        None => a,
    };

    let authority = authority
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim();

    anyhow::ensure!(!authority.is_empty(), "URL sem host");
    let has_port = match authority.rfind(':') {
        Some(idx) => authority[idx + 1..]
            .chars()
            .all(|c| c.is_ascii_digit())
            && !authority[idx + 1..].is_empty()
            && !authority.contains(']'), // crude IPv6 guard
        None => false,
    };
    Ok(if has_port {
        authority.to_string()
    } else {
        format!("{authority}:{DEFAULT_RWP_PORT}")
    })
}

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
        // Flags bare a nível de aplicação que nossos templates colocam na própria
        // linha (continuação): o glacier-ui já conhece os flags de framework/widget
        // (`bold`, `secure`, `navigateBack`), mas a diretiva `else` é nossa, então
        // registramos antes de parsear qualquer template — senão um `else` solto
        // viraria um nó irmão espúrio que engole as propriedades seguintes
        // (class/onClick).
        glacier_ui::register_bare_flags(["else", "senao"]);

        let mut motor = GlacierUI::new();
        if let Err(e) = motor.load_stylesheet("crates/rustploy-gui/styles/app.gss") {
            eprintln!("stylesheet: {e}");
        }
        if let Err(e) = motor.register(Box::new(Root::default())) {
            eprintln!("register: {e}");
        }
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
                // Intercept a toast request riding along a `ContextPatch`
                // (see `net::{TOAST_KIND_KEY, TOAST_MSG_KEY}`) — the reserved
                // pairs an async effect or `poll_stream` uses to ask for a
                // toast, since neither has a `Context` to call
                // `ctx.show_toast` on directly. Show it via `GlacierUI`'s own
                // host-app API and strip the pairs so they never land as
                // meaningless context keys.
                let event = match event {
                    EngineMessage::ContextPatch(pairs) => {
                        let (rest, toast) = extract_toast(pairs);
                        if let Some(spec) = toast {
                            self.motor.show_toast(spec);
                        }
                        EngineMessage::ContextPatch(rest)
                    }
                    other => other,
                };
                self.motor.dispatch(&event).map(Message::Engine)
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
    let min = Size::new(1000.0, 680.0);
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
        // `templates/app.kdl` (drag region + minimize/maximize/close). The
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

/// Pulls a toast request out of a `ContextPatch`'s pairs (see
/// `net::{TOAST_KIND_KEY, TOAST_MSG_KEY}` for why it travels this way instead
/// of through `Context::show_toast`), returning the spec (if any pair asked
/// for one) alongside the rest of the pairs with the reserved keys removed —
/// they're host-app state, never meant to reach the visible context.
fn extract_toast(pairs: Vec<(String, String)>) -> (Vec<(String, String)>, Option<ToastSpec>) {
    let mut kind = None;
    let mut message = None;
    let rest = pairs
        .into_iter()
        .filter(|(k, v)| match k.as_str() {
            net::TOAST_KIND_KEY => {
                kind = Some(v.clone());
                false
            }
            net::TOAST_MSG_KEY => {
                message = Some(v.clone());
                false
            }
            _ => true,
        })
        .collect();
    let spec = message.map(|m| match kind.as_deref() {
        Some("error") => ToastSpec::error(m),
        Some("warning") => ToastSpec::warning(m),
        Some("info") => ToastSpec::info(m),
        _ => ToastSpec::success(m),
    });
    (rest, spec)
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
/// `templates/app.kdl`.
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
