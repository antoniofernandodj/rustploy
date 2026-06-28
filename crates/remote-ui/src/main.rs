//! Rustploy Remote (glacier-ui) — desktop client whose UI is described in XML
//! templates and rendered by the published `glacier-ui` engine. The network
//! layer runs through glacier-ui's async bridge (effects + subscriptions).

mod net;
mod root;
mod rwp;

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

struct App {
    motor: GlacierUI,
}

impl App {
    fn boot() -> (Self, Task<EngineMessage>) {
        let mut motor = GlacierUI::new();
        if let Err(e) = motor.load_stylesheet("crates/remote-ui/styles/app.iss") {
            eprintln!("stylesheet: {e}");
        }
        if let Err(e) = motor.register(Box::new(root::Root::default())) {
            eprintln!("register: {e}");
        }
        motor.set_initial_screen("app");
        (Self { motor }, Task::none())
    }

    fn update(&mut self, msg: EngineMessage) -> Task<EngineMessage> {
        self.motor.dispatch(&msg)
    }

    fn view(&self) -> Element<'_, EngineMessage> {
        self.motor.render_current().unwrap_or_else(|e| {
            iced::widget::text(e).into()
        })
    }

    fn subscription(&self) -> Subscription<EngineMessage> {
        Subscription::batch([
            self.motor.subscription(),
            GlacierUI::reload_subscription(Duration::from_millis(500)),
        ])
    }

    fn theme(&self) -> iced::Theme {
        self.motor.theme()
    }
}

fn main() -> iced::Result {
    iced::application("Rustploy Remote", App::update, App::view)
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
            platform_specific: iced::window::settings::PlatformSpecific {
                application_id: "rustploy-remote-ui".to_string(),
                ..Default::default()
            },
            ..Default::default()
        })
        .run_with(App::boot)
}
