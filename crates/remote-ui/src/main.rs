//! Rustploy Remote (glacier-ui) — desktop client whose UI is described in KDL
//! templates and rendered by the published `glacier-ui` engine. The network
//! layer runs through glacier-ui's async bridge (effects + subscriptions).

mod net;
mod root;
mod rwp;
mod store;

use glacier_ui::{EngineMessage, GlacierUI};
use iced::{Element, Subscription, Task};
use std::time::Duration;

const DEFAULT_RWP_PORT: u16 = 8787;

/// Parses an `rwp://`/`rwps://` URL into the `host:port` target for the TCP
/// connection, filling in the default port when omitted.
pub fn connect_target(url: &str) -> anyhow::Result<String> {
    let a = url.trim();
    let authority = match a.split_once("://") {
        Some((scheme, rest)) => {
            anyhow::ensure!(
                scheme.eq_ignore_ascii_case("rwp") || scheme.eq_ignore_ascii_case("rwps"),
                "esquema não suportado: {scheme}:// — use rwp:// ou rwps://"
            );
            rest
        }
        None => a,
    };
    let authority = authority.split(['/', '?', '#']).next().unwrap_or("").trim();
    anyhow::ensure!(!authority.is_empty(), "URL sem host");
    let has_port = match authority.rfind(':') {
        Some(idx) => authority[idx + 1..].chars().all(|c| c.is_ascii_digit())
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

/// App-level message: either an engine event from glacier-ui, or the resolved
/// id of our (single) window, cached on startup so the custom titlebar can
/// drive window controls synchronously.
#[derive(Debug, Clone)]
enum Message {
    Engine(EngineMessage),
    WindowReady(Option<iced::window::Id>),
}

struct App {
    motor: GlacierUI,
    /// Cached id of the main window. We resolve it once at boot instead of
    /// per-action: on Wayland, deferring window controls through a `get_latest`
    /// round-trip loses the pointer-grab serial, so `window:drag` silently
    /// no-ops. Handling them synchronously against the cached id makes the
    /// borderless titlebar actually draggable.
    window_id: Option<iced::window::Id>,
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let mut motor = GlacierUI::new();
        if let Err(e) = motor.load_stylesheet("crates/remote-ui/styles/app.gss") {
            eprintln!("stylesheet: {e}");
        }
        if let Err(e) = motor.register(Box::new(root::Root::default())) {
            eprintln!("register: {e}");
        }
        motor.set_initial_screen("app");
        (
            Self { motor, window_id: None },
            iced::window::latest().map(Message::WindowReady),
        )
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::WindowReady(id) => {
                self.window_id = id;
                Task::none()
            }
            Message::Engine(event) => {
                // Intercept the built-in `window:*` actions emitted by the
                // custom titlebar and run them against the cached window id
                // (see `window_id`). Everything else is a normal engine event.
                if let EngineMessage::UiClick(action) = &event {
                    if let (Some(cmd), Some(id)) =
                        (action.strip_prefix("window:"), self.window_id)
                    {
                        return window_control(id, cmd);
                    }
                }
                self.motor.dispatch(&event).map(Message::Engine)
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        self.motor
            .render_current()
            .unwrap_or_else(|e| iced::widget::text(e).into())
            .map(Message::Engine)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.motor.subscription(),
            GlacierUI::reload_subscription(Duration::from_millis(500)),
        ])
        .map(Message::Engine)
    }

    fn theme(&self) -> iced::Theme {
        self.motor.theme()
    }
}

/// Maps a `window:<cmd>` action to its iced window task, driven against the
/// known window id (so drag/resize keep the live pointer-grab serial on
/// Wayland — a deferred `latest()` round-trip would lose it). `resize:<dir>`
/// starts an interactive border/corner resize (`drag_resize`).
fn window_control(id: iced::window::Id, cmd: &str) -> Task<Message> {
    use iced::window;
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

fn main() -> iced::Result {
    iced::application(App::boot, App::update, App::view)
        .title("Rustploy Remote")
        .subscription(App::subscription)
        .theme(App::theme)
        // TODO(fonte): retomar a fonte monoespaçada do design (JetBrains Mono,
        // em assets/fonts/) quando resolvermos por que a fonte custom some neste
        // ambiente (iced/wgpu não desenhava os glifos). Por ora usamos a fonte
        // interna do iced, que renderiza de forma confiável.
        // .font(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice())
        // .font(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf").as_slice())
        // .default_font(iced::Font::with_name("JetBrains Mono"))
        .window(iced::window::Settings {
            size: iced::Size::new(1280.0, 820.0),
            min_size: Some(iced::Size::new(1000.0, 680.0)),
            // Borderless: the OS titlebar is replaced by a custom one defined in
            // `templates/app.kdl` (drag region + minimize/maximize/close). The
            // `window:*` actions it emits are handled in `update` against the
            // cached window id (see `App::window_id`).
            decorations: false,
            platform_specific: iced::window::settings::PlatformSpecific {
                application_id: "rustploy-remote-ui".to_string(),
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}
