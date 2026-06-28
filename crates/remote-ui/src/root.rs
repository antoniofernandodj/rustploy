//! The single root component: owns connection state, routes UI actions and
//! exposes the network subscription. All screens live in `templates/app.xml`
//! and switch on the `screen`/`view` context keys.

use glacier_ui::{Component, Context, EngineMessage, Template};

#[derive(Default)]
pub struct Root {
    /// Normalized `rwp://…` URL (mirror of the `url` context key on connect).
    addr: String,
    token: Option<String>,
    /// Whether the polling subscription should be live.
    active: bool,
    /// Bumped on every (re)connect so the subscription gets a fresh id.
    seq: u64,
}

impl Component for Root {
    fn name(&self) -> &str {
        "app"
    }

    fn template(&self) -> Template {
        Template::File("crates/remote-ui/templates/app.xml".into())
    }

    fn init(&mut self, ctx: &mut Context) {
        ctx.set("screen", "login");
        ctx.set("view", "deployments");
        ctx.set("url", "rwp://127.0.0.1:8787");
        ctx.set("token", "");
        ctx.set("remember_url", "true");
        ctx.set("remember_token", "false");
        ctx.set("connected", "false");
        ctx.set("error", "");
        ctx.set("status_line", "aguardando conexão");
        // Sensible defaults so the shell renders before the first poll lands.
        ctx.set("daemon_version", "—");
        ctx.set("daemon_uptime", "—");
        ctx.set("services_label", "0/0");
        ctx.set("deployments", "[]");
        ctx.set("deployments_count", "0");
        ctx.set("projects", "[]");
        ctx.set("projects_count", "0");
    }

    fn update(&mut self, action: &str, value: Option<&str>, ctx: &mut Context) {
        match action {
            "url_changed" => {
                if let Some(v) = value {
                    ctx.set("url", v);
                }
            }
            "token_changed" => {
                if let Some(v) = value {
                    ctx.set("token", v);
                }
            }
            "toggle_remember_url" => {
                ctx.set("remember_url", flag(value));
            }
            "toggle_remember_token" => {
                ctx.set("remember_token", flag(value));
            }
            "connect" => {
                let url = ctx.get("url").cloned().unwrap_or_default();
                self.addr = normalize_url(&url);
                let tok = ctx.get("token").cloned().unwrap_or_default();
                self.token = if tok.trim().is_empty() { None } else { Some(tok) };
                self.active = true;
                self.seq += 1;
                ctx.set("error", "");
                ctx.set("status_line", "conectando…");
            }
            "disconnect" => {
                self.active = false;
                self.seq += 1;
                ctx.set("connected", "false");
                ctx.set("screen", "login");
                ctx.set("status_line", "desconectado");
            }
            // Sidebar / tab navigation just flips the active view key.
            "nav" => {
                if let Some(v) = value {
                    ctx.set("view", v);
                }
            }
            _ => {
                // `nav_<view>` shorthand from buttons without a value payload.
                if let Some(view) = action.strip_prefix("nav_") {
                    ctx.set("view", view);
                }
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<EngineMessage> {
        if self.active && !self.addr.is_empty() {
            iced::Subscription::run_with_id(
                self.seq,
                crate::net::poll_stream(self.addr.clone(), self.token.clone()),
            )
        } else {
            iced::Subscription::none()
        }
    }
}

/// Maps a checkbox/toggle payload (`"true"`/`"false"`) to a context flag.
fn flag(value: Option<&str>) -> &'static str {
    match value {
        Some(v) if v.eq_ignore_ascii_case("true") || v == "1" => "true",
        _ => "false",
    }
}

/// Adds the `rwp://` scheme when the user typed a bare authority.
pub fn normalize_url(input: &str) -> String {
    let a = input.trim();
    if a.is_empty() {
        String::new()
    } else if a.contains("://") {
        a.to_string()
    } else {
        format!("rwp://{a}")
    }
}
