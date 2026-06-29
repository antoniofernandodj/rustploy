//! The single root component: owns connection state, routes UI actions and
//! exposes the network subscription. All screens live in `templates/app.xml`
//! and switch on the `screen`/`view` context keys.

use glacier_ui::{Component, Context, EngineMessage, Template};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct Root {
    /// Normalized `rwp://…` URL (mirror of the `url` context key on connect).
    addr: String,
    token: Option<String>,
    /// Whether the polling subscription should be live.
    active: bool,
    /// Bumped on every (re)connect so the subscription gets a fresh id.
    seq: u64,
    /// Id of the service currently open in the detail view (`view=service`).
    selected_service: String,
    /// Shared with the network subscription so the live log stream knows which
    /// service's `LogLine` events to surface, without restarting the stream.
    selected_shared: Arc<Mutex<String>>,
}

impl Root {
    /// Fires a lifecycle command for the currently-selected service through the
    /// async bridge, then refreshes the detail panel. No-op without a selection.
    fn service_action(
        &self,
        ctx: &mut Context,
        make: impl FnOnce(String) -> shared::Command,
    ) {
        if self.selected_service.is_empty() || self.addr.is_empty() {
            return;
        }
        let id = self.selected_service.clone();
        let cmd = make(id.clone());
        ctx.set("svc_action_msg", "enviando…");
        ctx.perform(crate::net::run_service_action(
            self.addr.clone(),
            self.token.clone(),
            cmd,
            id,
        ));
    }

    /// Applies an environment-variable edit to the selected service through the
    /// async bridge, then refreshes the detail panel.
    fn env_op(&self, ctx: &mut Context, op: crate::net::EnvOp) {
        if self.selected_service.is_empty() || self.addr.is_empty() {
            return;
        }
        ctx.set("svc_action_msg", "salvando…");
        ctx.perform(crate::net::run_env_op(
            self.addr.clone(),
            self.token.clone(),
            self.selected_service.clone(),
            op,
        ));
    }
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

        // Prefill from saved preferences (remembered URL/token).
        let prefs = crate::store::Prefs::load();
        let url = prefs
            .url
            .filter(|_| prefs.remember_url)
            .unwrap_or_else(|| "rwp://127.0.0.1:8787".to_string());
        let token = prefs.token.filter(|_| prefs.remember_token).unwrap_or_default();
        ctx.set("url", url);
        ctx.set("token", token);
        ctx.set("remember_url", bool_str(prefs.remember_url));
        ctx.set("remember_token", bool_str(prefs.remember_token));
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
        ctx.set("services", "[]");
        ctx.set("services_count", "0");
        ctx.set("service_rows", "[]");
        // Service detail (view=service) defaults.
        ctx.set("tab", "general");
        ctx.set("svc_loading", "false");
        ctx.set("svc_error", "");
        ctx.set("svc_name", "—");
        ctx.set("svc_project", "—");
        ctx.set("svc_status_label", "—");
        ctx.set("svc_status_color", "#8B949E");
        ctx.set("svc_source_kind", "—");
        ctx.set("svc_source_detail", "—");
        ctx.set("svc_build", "—");
        ctx.set("svc_port", "—");
        ctx.set("svc_host_port", "—");
        ctx.set("svc_domain", "—");
        ctx.set("svc_tls", "—");
        ctx.set("svc_replicas", "—");
        ctx.set("svc_db_kind", "—");
        ctx.set("svc_hc", "—");
        ctx.set("svc_run_command", "—");
        ctx.set("svc_run_args", "—");
        ctx.set("svc_env", "[]");
        ctx.set("svc_env_count", "0");
        ctx.set("svc_env_text", "");
        ctx.set("svc_logs", "[]");
        ctx.set("svc_logs_count", "0");
        ctx.set("svc_deployments", "[]");
        ctx.set("svc_deployments_count", "0");
        ctx.set("svc_action_msg", "");
        ctx.set("env_new_key", "");
        ctx.set("env_new_val", "");
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
            "env_new_key_changed" => {
                if let Some(v) = value {
                    ctx.set("env_new_key", v);
                }
            }
            "env_new_val_changed" => {
                if let Some(v) = value {
                    ctx.set("env_new_val", v);
                }
            }
            "env_add" => {
                let key = ctx.get("env_new_key").cloned().unwrap_or_default();
                let value = ctx.get("env_new_val").cloned().unwrap_or_default();
                if !key.trim().is_empty() {
                    self.env_op(ctx, crate::net::EnvOp::Set {
                        key: key.trim().to_string(),
                        value,
                    });
                    ctx.set("env_new_key", "");
                    ctx.set("env_new_val", "");
                }
            }
            "toggle_remember_url" => {
                ctx.set("remember_url", flag(value));
                save_prefs(ctx);
            }
            "toggle_remember_token" => {
                ctx.set("remember_token", flag(value));
                save_prefs(ctx);
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
                save_prefs(ctx);
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
            // Service lifecycle actions (operate on the open service).
            "svc_deploy" | "svc_rebuild" => {
                self.service_action(ctx, |id| shared::Command::DeployStart { service_id: id });
            }
            "svc_reload" => {
                self.service_action(ctx, |id| shared::Command::ServiceReload { service_id: id });
            }
            "svc_stop" => {
                self.service_action(ctx, |id| shared::Command::ServiceStop { service_id: id });
            }
            _ => {
                // `nav_<view>` shorthand from buttons without a value payload.
                if let Some(view) = action.strip_prefix("nav_") {
                    ctx.set("view", view);
                    // Leaving the detail view: stop surfacing its live logs.
                    self.selected_service.clear();
                    if let Ok(mut sel) = self.selected_shared.lock() {
                        sel.clear();
                    }
                    return;
                }
                // `open_service:<id>` — open the detail view and fetch its data.
                if let Some(id) = action.strip_prefix("open_service:") {
                    self.selected_service = id.to_string();
                    if let Ok(mut sel) = self.selected_shared.lock() {
                        *sel = id.to_string();
                    }
                    ctx.set("selected_service", id);
                    ctx.set("view", "service");
                    ctx.set("tab", "general");
                    ctx.set("svc_loading", "true");
                    ctx.set("svc_error", "");
                    let (addr, token, sid) =
                        (self.addr.clone(), self.token.clone(), id.to_string());
                    if !addr.is_empty() {
                        ctx.perform(crate::net::fetch_service_detail(addr, token, sid));
                    }
                    return;
                }
                // `tab:<name>` — switch the active sub-tab in the detail view.
                if let Some(tab) = action.strip_prefix("tab:") {
                    ctx.set("tab", tab);
                    return;
                }
                // `env_del:<key>` — remove an environment variable.
                if let Some(key) = action.strip_prefix("env_del:") {
                    self.env_op(ctx, crate::net::EnvOp::Delete { key: key.to_string() });
                    return;
                }
                // `env_import` — replace all vars with the edited `.env` blob.
                if action == "env_import" {
                    let text = ctx.get("svc_env_text").cloned().unwrap_or_default();
                    self.env_op(ctx, crate::net::EnvOp::ImportDotenv(text));
                    return;
                }
                // `env_export` — dump the current `.env` blob to a file.
                if action == "env_export" {
                    let body = ctx.get("svc_env_text").cloned().unwrap_or_default();
                    let name = ctx.get("svc_name").cloned().unwrap_or_else(|| "service".into());
                    let dir = std::env::var("HOME").unwrap_or_else(|_| ".".into());
                    let path = format!("{dir}/{name}.env");
                    match std::fs::write(&path, body) {
                        Ok(_) => ctx.set("svc_action_msg", format!("exportado para {path}")),
                        Err(e) => ctx.set("svc_action_msg", format!("erro ao exportar: {e}")),
                    }
                }
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<EngineMessage> {
        if self.active && !self.addr.is_empty() {
            iced::Subscription::run_with_id(
                self.seq,
                crate::net::poll_stream(
                    self.addr.clone(),
                    self.token.clone(),
                    self.selected_shared.clone(),
                ),
            )
        } else {
            iced::Subscription::none()
        }
    }
}

/// Renders a bool as the context flag string.
fn bool_str(b: bool) -> &'static str {
    if b { "true" } else { "false" }
}

/// Persists the current login preferences from the context. The URL/token are
/// only stored when their respective "remember" flag is on.
fn save_prefs(ctx: &Context) {
    let on = |k: &str| ctx.get(k).map(|v| v == "true").unwrap_or(false);
    let remember_url = on("remember_url");
    let remember_token = on("remember_token");
    crate::store::Prefs {
        remember_url,
        remember_token,
        url: if remember_url { ctx.get("url").cloned() } else { None },
        token: if remember_token { ctx.get("token").cloned() } else { None },
    }
    .save();
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
