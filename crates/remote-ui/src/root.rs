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
    /// Same, for the deployment whose `BuildLog` events feed the Deployments tab.
    selected_deploy_shared: Arc<Mutex<String>>,
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

    /// Applies a form-driven spec edit (Domains/Healthcheck/Advanced) and
    /// refreshes the detail panel.
    fn spec_op(&self, ctx: &mut Context, op: crate::net::SpecOp) {
        if self.selected_service.is_empty() || self.addr.is_empty() {
            return;
        }
        ctx.set("svc_action_msg", "salvando…");
        ctx.perform(crate::net::run_spec_op(
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
        // Home screens (Monitoring / Ingress / Docker / Settings).
        ctx.set("ingress", "[]");
        ctx.set("ingress_count", "0");
        ctx.set("docker_rows", "[]");
        ctx.set("monitoring", "[]");
        ctx.set("sys_cpu", "—");
        ctx.set("sys_mem", "—");
        ctx.set("sys_disk", "—");
        ctx.set("sys_load", "—");
        ctx.set("ss_domain", "");
        ctx.set("ss_email", "");
        ctx.set("settings_msg", "");
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
        ctx.set("svc_logs_text", "");
        ctx.set("dep_build_text", "");
        ctx.set("svc_deployments", "[]");
        ctx.set("svc_deployments_count", "0");
        ctx.set("svc_action_msg", "");
        ctx.set("dep_selected", "");
        ctx.set("dep_build_logs", "[]");
        ctx.set("dep_build_count", "0");
        ctx.set("env_new_key", "");
        ctx.set("env_new_val", "");
        ctx.set("env_text_open", "false");
        ctx.set("svc_env_text_orig", "");
        // Editable spec form fields.
        ctx.set("f_domain", "");
        ctx.set("f_host_port", "");
        ctx.set("f_tls", "false");
        ctx.set("f_hc_kind", "tcp");
        ctx.set("f_hc_path", "");
        ctx.set("f_hc_status", "");
        ctx.set("f_hc_interval", "");
        ctx.set("f_hc_timeout", "");
        ctx.set("f_hc_retries", "");
        ctx.set("f_hc_start", "");
        ctx.set("f_replicas", "");
        ctx.set("f_run_command", "");
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
            // Topbar: Deploy points the user at the project grid to pick a
            // service; Stop All stops every running service.
            "deploy" => {
                ctx.set("view", "projects");
            }
            "stop_all" => {
                if !self.addr.is_empty() {
                    ctx.set("status_line", "parando todos…");
                    ctx.perform(crate::net::stop_all(self.addr.clone(), self.token.clone()));
                }
            }
            // Settings (daemon web server) fields + save.
            "ss_domain_changed" => {
                if let Some(v) = value {
                    ctx.set("ss_domain", v);
                }
            }
            "ss_email_changed" => {
                if let Some(v) = value {
                    ctx.set("ss_email", v);
                }
            }
            "settings_save" => {
                if !self.addr.is_empty() {
                    let domain = ctx.get("ss_domain").cloned().unwrap_or_default();
                    let email = ctx.get("ss_email").cloned().unwrap_or_default();
                    ctx.set("settings_msg", "salvando…");
                    ctx.perform(crate::net::save_settings(
                        self.addr.clone(),
                        self.token.clone(),
                        domain,
                        email,
                    ));
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
            // Save handlers for the editable spec forms.
            "dom_save" => {
                let op = crate::net::SpecOp::Domains {
                    domain: ctx.get("f_domain").cloned().unwrap_or_default(),
                    host_port: ctx.get("f_host_port").cloned().unwrap_or_default(),
                    tls: ctx.get("f_tls").map(|v| v == "true").unwrap_or(false),
                };
                self.spec_op(ctx, op);
            }
            "hc_save" => {
                let g = |k: &str| ctx.get(k).cloned().unwrap_or_default();
                let op = crate::net::SpecOp::Healthcheck {
                    kind: g("f_hc_kind"),
                    http_path: g("f_hc_path"),
                    expected_status: g("f_hc_status"),
                    interval: g("f_hc_interval"),
                    timeout: g("f_hc_timeout"),
                    retries: g("f_hc_retries"),
                    start_period: g("f_hc_start"),
                };
                self.spec_op(ctx, op);
            }
            "adv_save" => {
                let op = crate::net::SpecOp::Advanced {
                    replicas: ctx.get("f_replicas").cloned().unwrap_or_default(),
                    run_command: ctx.get("f_run_command").cloned().unwrap_or_default(),
                };
                self.spec_op(ctx, op);
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
                    if let Ok(mut d) = self.selected_deploy_shared.lock() {
                        d.clear();
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
                    ctx.set("dep_selected", "");
                    if let Ok(mut d) = self.selected_deploy_shared.lock() {
                        d.clear();
                    }
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
                // `field:<key>` — generic form input/toggle → set the context key.
                if let Some(key) = action.strip_prefix("field:") {
                    if let Some(v) = value {
                        ctx.set(key, v);
                    }
                    return;
                }
                // `hckind:<kind>` — pick the healthcheck kind.
                if let Some(kind) = action.strip_prefix("hckind:") {
                    ctx.set("f_hc_kind", kind);
                    return;
                }
                // `dep_logs:<id>` — load a deployment's build log (and stream it).
                if let Some(id) = action.strip_prefix("dep_logs:") {
                    ctx.set("dep_selected", id);
                    ctx.set("dep_build_logs", "[]");
                    ctx.set("dep_build_count", "0");
                    if let Ok(mut sel) = self.selected_deploy_shared.lock() {
                        *sel = id.to_string();
                    }
                    if !self.addr.is_empty() {
                        ctx.perform(crate::net::fetch_build_logs(
                            self.addr.clone(),
                            self.token.clone(),
                            id.to_string(),
                        ));
                    }
                    return;
                }
                // `env_del:<key>` — remove an environment variable.
                if let Some(key) = action.strip_prefix("env_del:") {
                    self.env_op(ctx, crate::net::EnvOp::Delete { key: key.to_string() });
                    return;
                }
                // `env_text_toggle` — open/close the `.env` editor.
                if action == "env_text_toggle" {
                    let open = ctx.get("env_text_open").map(|v| v == "true").unwrap_or(false);
                    ctx.set("env_text_open", if open { "false" } else { "true" });
                    return;
                }
                // `env_text_cancel` — close the editor and discard edits.
                if action == "env_text_cancel" {
                    let orig = ctx.get("svc_env_text_orig").cloned().unwrap_or_default();
                    ctx.set("svc_env_text", orig);
                    ctx.set("env_text_open", "false");
                    return;
                }
                // `env_import` — replace all vars with the edited `.env` blob.
                if action == "env_import" {
                    let text = ctx.get("svc_env_text").cloned().unwrap_or_default();
                    ctx.set("env_text_open", "false");
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
                    self.selected_deploy_shared.clone(),
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
