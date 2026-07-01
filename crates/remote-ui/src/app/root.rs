//! The single root component: owns connection state, routes UI actions and
//! exposes the network subscription. All screens live in `templates/app.kdl`
//! and switch on the `screen`/`view` context keys.

use glacier_ui::{Component, Context, EngineMessage, Template};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

/// Identity + payload for the network subscription. iced 0.14's
/// `Subscription::run_with` takes `(data, fn(&data) -> Stream)` where `data`
/// must be `Hash` (it decides when to restart the subscription) and the builder
/// is a non-capturing `fn`. We hash only `seq` — bumped on every (re)connect —
/// and carry the connection details for the builder to clone.
#[derive(Clone)]
struct PollKey {
    seq: u64,
    addr: String,
    token: Option<String>,
    selected: Arc<Mutex<String>>,
    selected_deploy: Arc<Mutex<String>>,
    deploy_track: Arc<Mutex<super::net::DeployTrack>>,
    search: Arc<Mutex<String>>,
}

impl Hash for PollKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.seq.hash(state);
    }
}

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
    /// Identity + `started_at` of the deploy started from the open service's
    /// detail panel. Set by `start_deploy`'s `ctx.perform` future; read by the
    /// poll loop to tick `svc_deploy_elapsed` (1Hz) and detect completion.
    deploy_shared: Arc<Mutex<super::net::DeployTrack>>,
    /// Current topbar search term, shared with the poll loop so it can filter
    /// deployments/services/Docker rows without a context round-trip (the
    /// poll loop never sees the live `Context`, only what it patches into it).
    search_shared: Arc<Mutex<String>>,
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
        ctx.set("svc_action_color", "#8B949E");
        ctx.perform(super::net::run_service_action(
            self.addr.clone(),
            self.token.clone(),
            cmd,
            id,
        ));
    }

    /// Starts a deploy for the currently-selected service and arms the live
    /// "1s, 2s, 3s…" elapsed timer (ticked by the poll subscription via
    /// `deploy_shared`), mirroring the old `remote-client`'s deploy duration
    /// display. No-op without a selection.
    fn start_deploy(&self, ctx: &mut Context) {
        if self.selected_service.is_empty() || self.addr.is_empty() {
            return;
        }
        ctx.set("svc_action_msg", "enviando…");
        ctx.set("svc_action_color", "#8B949E");
        ctx.perform(super::net::start_deploy(
            self.addr.clone(),
            self.token.clone(),
            self.selected_service.clone(),
            self.deploy_shared.clone(),
        ));
    }

    /// Applies an environment-variable edit to the selected service through the
    /// async bridge, then refreshes the detail panel.
    fn env_op(&self, ctx: &mut Context, op: super::net::EnvOp) {
        if self.selected_service.is_empty() || self.addr.is_empty() {
            return;
        }
        ctx.set("svc_action_msg", "salvando…");
        ctx.set("svc_action_color", "#8B949E");
        ctx.perform(super::net::run_env_op(
            self.addr.clone(),
            self.token.clone(),
            self.selected_service.clone(),
            op,
        ));
    }

    /// Applies a form-driven spec edit (Domains/Healthcheck/Advanced) and
    /// refreshes the detail panel.
    fn spec_op(&self, ctx: &mut Context, op: super::net::SpecOp) {
        if self.selected_service.is_empty() || self.addr.is_empty() {
            return;
        }
        ctx.set("svc_action_msg", "salvando…");
        ctx.set("svc_action_color", "#8B949E");
        ctx.perform(super::net::run_spec_op(
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
        Template::File("crates/remote-ui/templates/app.kdl".into())
    }

    fn init(&mut self, ctx: &mut Context) {
        ctx.set("screen", "login");
        ctx.set("view", "deployments");

        // Prefill from saved preferences (remembered URL/token).
        let prefs = super::store::Prefs::load();
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
        // Cleared by the poll loop's first successful tick; while true, every
        // data-driven screen (Deployments/Projects/Monitoring/Ingress/Docker)
        // shows a "Carregando dados…" placeholder instead of an empty table.
        ctx.set("data_loading", "true");
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
        // Docker tab sub-tabs: Containers (docker_rows, above) / Images /
        // Volumes / Networks — the whole host's inventory, not just
        // rustploy-managed resources (see `docker_inventory` on the daemon).
        ctx.set("docker_tab", "containers");
        ctx.set("docker_msg", "");
        ctx.set("docker_images", "[]");
        ctx.set("docker_images_count", "0");
        ctx.set("docker_images_only_used", "false");
        ctx.set("docker_prune_all_images", "false");
        ctx.set("docker_volumes", "[]");
        ctx.set("docker_volumes_count", "0");
        ctx.set("docker_volumes_only_used", "false");
        ctx.set("docker_prune_all_volumes", "false");
        ctx.set("docker_networks", "[]");
        ctx.set("docker_networks_count", "0");
        ctx.set("docker_networks_only_used", "false");
        ctx.set("monitoring", "[]");
        // Topbar search: filters deployments/services/Docker rows (see
        // `search_changed` and `net::poll_stream`'s `search` parameter).
        ctx.set("search", "");
        ctx.set("sys_cpu", "—");
        ctx.set("sys_mem", "—");
        ctx.set("sys_disk", "—");
        ctx.set("sys_load", "—");
        ctx.set("ss_domain", "");
        ctx.set("ss_email", "");
        ctx.set("settings_msg", "");
        // Settings → Git (provider management).
        ctx.set("settings_tab", "web");
        ctx.set("gp_name", "");
        ctx.set("gp_base_url", "");
        ctx.set("gp_mode", "oauth");
        ctx.set("gp_client_id", "");
        ctx.set("gp_client_secret", "");
        ctx.set("gp_pat", "");
        ctx.set("gp_redirect", super::net::oauth_redirect_uri(""));
        ctx.set("gp_msg", "");
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
        ctx.set("svc_internal_url", "—");
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
        ctx.set("svc_action_color", "#8B949E");
        // Live deploy timer ("1s, 2s, 3s…" while a deploy started from this
        // panel is running; frozen + colored once it finishes — see
        // `start_deploy`/`poll_stream`).
        ctx.set("svc_deploy_running", "false");
        ctx.set("svc_deploy_elapsed", "");
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
        ctx.set("f_repo_url", "");
        ctx.set("f_branch", "");
        ctx.set("f_username", "");
        ctx.set("f_credentials", "");
        ctx.set("f_build_path", "");
        ctx.set("f_watch_paths", "");
        ctx.set("f_submodules", "false");
        ctx.set("f_dockerfile", "");
        ctx.set("f_context_path", "");
        ctx.set("f_build_stage", "");
        ctx.set("f_gen_port", "");
        // General provider sub-tab (Git | Gitea) + Gitea picker.
        ctx.set("prov_tab", "git");
        ctx.set("gitea_providers", "[]");
        ctx.set("gitea_count", "0");
        ctx.set("gitea_repos", "[]");
        ctx.set("gitea_branches", "[]");
        ctx.set("gitea_provider_id", "");
        ctx.set("gitea_repo", "");
        ctx.set("gitea_msg", "");
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
                    ctx.perform(super::net::stop_all(self.addr.clone(), self.token.clone()));
                }
            }
            // Topbar search: filters deployments/services/Docker rows. The
            // poll loop (not the live Context) builds those rows, so the term
            // is mirrored into `search_shared` for it to read each tick.
            "search_changed" => {
                if let Some(v) = value {
                    ctx.set("search", v);
                    if let Ok(mut s) = self.search_shared.lock() {
                        *s = v.to_string();
                    }
                }
            }
            // Docker tab: remove unused images/volumes/networks, then refresh
            // that sub-tab immediately (the perform's own pairs include it —
            // no need to wait for the next poll tick).
            "docker_prune_images" => {
                if !self.addr.is_empty() {
                    let all = ctx.get("docker_prune_all_images").map(|v| v == "true").unwrap_or(false);
                    ctx.set("docker_msg", if all { "removendo TODAS as imagens sem uso…" } else { "removendo imagens dangling…" });
                    ctx.perform(super::net::prune_docker_images(self.addr.clone(), self.token.clone(), all));
                }
            }
            "docker_prune_volumes" => {
                if !self.addr.is_empty() {
                    let all = ctx.get("docker_prune_all_volumes").map(|v| v == "true").unwrap_or(false);
                    ctx.set("docker_msg", "removendo volumes sem uso…");
                    ctx.perform(super::net::prune_docker_volumes(self.addr.clone(), self.token.clone(), all));
                }
            }
            "docker_prune_networks" => {
                if !self.addr.is_empty() {
                    ctx.set("docker_msg", "removendo redes sem uso…");
                    ctx.perform(super::net::prune_docker_networks(self.addr.clone(), self.token.clone()));
                }
            }
            // Settings (daemon web server) fields + save.
            "ss_domain_changed" => {
                if let Some(v) = value {
                    ctx.set("ss_domain", v);
                    // Keep the OAuth Redirect URI shown in Settings → Git in sync.
                    ctx.set("gp_redirect", super::net::oauth_redirect_uri(v));
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
                    ctx.perform(super::net::save_settings(
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
                    self.env_op(ctx, super::net::EnvOp::Set {
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
                ctx.set("data_loading", "true");
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
                self.start_deploy(ctx);
            }
            "svc_reload" => {
                self.service_action(ctx, |id| shared::Command::ServiceReload { service_id: id });
            }
            "svc_stop" => {
                self.service_action(ctx, |id| shared::Command::ServiceStop { service_id: id });
            }
            // Save handlers for the editable spec forms.
            "dom_save" => {
                let op = super::net::SpecOp::Domains {
                    domain: ctx.get("f_domain").cloned().unwrap_or_default(),
                    host_port: ctx.get("f_host_port").cloned().unwrap_or_default(),
                    tls: ctx.get("f_tls").map(|v| v == "true").unwrap_or(false),
                };
                self.spec_op(ctx, op);
            }
            "hc_save" => {
                let g = |k: &str| ctx.get(k).cloned().unwrap_or_default();
                let op = super::net::SpecOp::Healthcheck {
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
                let op = super::net::SpecOp::Advanced {
                    replicas: ctx.get("f_replicas").cloned().unwrap_or_default(),
                    run_command: ctx.get("f_run_command").cloned().unwrap_or_default(),
                };
                self.spec_op(ctx, op);
            }
            "gen_save" => {
                let g = |k: &str| ctx.get(k).cloned().unwrap_or_default();
                let op = super::net::SpecOp::General {
                    repo_url: g("f_repo_url"),
                    branch: g("f_branch"),
                    username: g("f_username"),
                    credentials: g("f_credentials"),
                    build_path: g("f_build_path"),
                    watch_paths: g("f_watch_paths"),
                    submodules: ctx.get("f_submodules").map(|v| v == "true").unwrap_or(false),
                    dockerfile: g("f_dockerfile"),
                    context_path: g("f_context_path"),
                    build_stage: g("f_build_stage"),
                    port: g("f_gen_port"),
                    // Git sub-tab detaches (empty); Gitea sub-tab binds the id.
                    provider_id: if ctx.get("prov_tab").map(|v| v == "gitea").unwrap_or(false) {
                        g("gitea_provider_id")
                    } else {
                        String::new()
                    },
                };
                self.spec_op(ctx, op);
            }
            // Gitea picker (Select-based): account → repos → branches.
            "gitea_provider_pick" => {
                if let Some(id) = value {
                    ctx.set("gitea_provider_id", id);
                    ctx.set("gitea_repo", "");
                    ctx.set("gitea_repos", "[]");
                    ctx.set("gitea_branches", "[]");
                    if !self.addr.is_empty() && !id.is_empty() {
                        ctx.set("gitea_msg", "carregando repositórios…");
                        ctx.perform(super::net::fetch_git_repos(
                            self.addr.clone(),
                            self.token.clone(),
                            id.to_string(),
                        ));
                    }
                }
            }
            "gitea_repo_pick" => {
                if let Some(full_name) = value {
                    // Resolve clone_url + default_branch from the loaded repo list
                    // (the Select only carries the chosen value = full_name).
                    let repos = ctx.get("gitea_repos").cloned().unwrap_or_default();
                    let (clone_url, default_branch) = find_repo(&repos, full_name);
                    ctx.set("gitea_repo", full_name);
                    if !clone_url.is_empty() {
                        ctx.set("f_repo_url", clone_url);
                    }
                    if !default_branch.is_empty() {
                        ctx.set("f_branch", default_branch);
                    }
                    ctx.set("gitea_branches", "[]");
                    let pid = ctx.get("gitea_provider_id").cloned().unwrap_or_default();
                    if !self.addr.is_empty() && !pid.is_empty() {
                        ctx.set("gitea_msg", "carregando branches…");
                        ctx.perform(super::net::fetch_git_branches(
                            self.addr.clone(),
                            self.token.clone(),
                            pid,
                            full_name.to_string(),
                        ));
                    }
                }
            }
            // Settings → Git: connect a provider, refresh, switch method.
            "gp_connect" => {
                if !self.addr.is_empty() {
                    let g = |k: &str| ctx.get(k).cloned().unwrap_or_default();
                    let (name, base_url, mode, client_id, client_secret, pat) = (
                        g("gp_name"), g("gp_base_url"), g("gp_mode"),
                        g("gp_client_id"), g("gp_client_secret"), g("gp_pat"),
                    );
                    ctx.set("gp_msg", "conectando…");
                    ctx.perform(super::net::git_provider_connect(
                        self.addr.clone(),
                        self.token.clone(),
                        name, base_url, mode, client_id, client_secret, pat,
                    ));
                }
            }
            "gp_refresh" => {
                if !self.addr.is_empty() {
                    ctx.perform(super::net::git_providers_only(
                        self.addr.clone(),
                        self.token.clone(),
                    ));
                }
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
                    // Deploy timer: keep ticking if this exact service still has
                    // the tracked deploy running (e.g. the user navigated away
                    // and back); otherwise clear the stale message/timer from
                    // whatever was last open — the poll loop only re-populates
                    // these keys while `deploy_shared.service_id` matches.
                    let still_running = self
                        .deploy_shared
                        .lock()
                        .map(|t| t.service_id == id && t.running)
                        .unwrap_or(false);
                    if !still_running {
                        ctx.set("svc_action_msg", "");
                        ctx.set("svc_action_color", "#8B949E");
                        ctx.set("svc_deploy_running", "false");
                        ctx.set("svc_deploy_elapsed", "");
                    }
                    let (addr, token, sid) =
                        (self.addr.clone(), self.token.clone(), id.to_string());
                    if !addr.is_empty() {
                        // One-shot load of everything the detail needs — including
                        // the Gitea provider/repo/branch lists — so `svc_loading`
                        // only clears once the General-tab selects can be populated.
                        ctx.perform(super::net::fetch_service_detail(
                            addr.clone(),
                            token.clone(),
                            sid,
                        ));
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
                        ctx.perform(super::net::fetch_build_logs(
                            self.addr.clone(),
                            self.token.clone(),
                            id.to_string(),
                        ));
                    }
                    return;
                }
                // `prov:<git|gitea>` — switch the General provider sub-tab. On
                // entering Gitea with a provider already chosen, load its repos.
                if let Some(which) = action.strip_prefix("prov:") {
                    ctx.set("prov_tab", which);
                    if which == "gitea" {
                        let pid = ctx.get("gitea_provider_id").cloned().unwrap_or_default();
                        let repos = ctx.get("gitea_repos").cloned().unwrap_or_default();
                        if !self.addr.is_empty()
                            && !pid.is_empty()
                            && (repos.is_empty() || repos == "[]")
                        {
                            ctx.set("gitea_msg", "carregando repositórios…");
                            ctx.perform(super::net::fetch_git_repos(
                                self.addr.clone(),
                                self.token.clone(),
                                pid,
                            ));
                        }
                    }
                    return;
                }
                // `docker_tab:<containers|images|volumes|networks>` — switch the
                // active Docker sub-tab.
                if let Some(t) = action.strip_prefix("docker_tab:") {
                    ctx.set("docker_tab", t);
                    return;
                }
                // `settings_tab:<web|git>` — switch the Settings sub-tab.
                if let Some(t) = action.strip_prefix("settings_tab:") {
                    ctx.set("settings_tab", t);
                    return;
                }
                // `gp_mode:<oauth|pat>` — pick the Git connect method.
                if let Some(m) = action.strip_prefix("gp_mode:") {
                    ctx.set("gp_mode", m);
                    return;
                }
                // `gp_delete:<id>` — remove a connected Git provider.
                if let Some(id) = action.strip_prefix("gp_delete:") {
                    if !self.addr.is_empty() {
                        ctx.set("gp_msg", "removendo…");
                        ctx.perform(super::net::git_provider_delete(
                            self.addr.clone(),
                            self.token.clone(),
                            id.to_string(),
                        ));
                    }
                    return;
                }
                // `env_del:<key>` — remove an environment variable.
                if let Some(key) = action.strip_prefix("env_del:") {
                    self.env_op(ctx, super::net::EnvOp::Delete { key: key.to_string() });
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
                    self.env_op(ctx, super::net::EnvOp::ImportDotenv(text));
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
            let key = PollKey {
                seq: self.seq,
                addr: self.addr.clone(),
                token: self.token.clone(),
                selected: self.selected_shared.clone(),
                selected_deploy: self.selected_deploy_shared.clone(),
                deploy_track: self.deploy_shared.clone(),
                search: self.search_shared.clone(),
            };
            iced::Subscription::run_with(key, |k: &PollKey| {
                super::net::poll_stream(
                    k.addr.clone(),
                    k.token.clone(),
                    k.selected.clone(),
                    k.selected_deploy.clone(),
                    k.deploy_track.clone(),
                    k.search.clone(),
                )
            })
        } else {
            iced::Subscription::none()
        }
    }
}

/// Looks up `(clone_url, default_branch)` for `full_name` in the `gitea_repos`
/// JSON array. Returns empty strings when not found.
fn find_repo(repos_json: &str, full_name: &str) -> (String, String) {
    serde_json::from_str::<serde_json::Value>(repos_json)
        .ok()
        .and_then(|v| v.as_array().cloned())
        .and_then(|arr| {
            arr.into_iter().find(|r| {
                r.get("full_name").and_then(|n| n.as_str()) == Some(full_name)
            })
        })
        .map(|r| {
            let s = |k: &str| r.get(k).and_then(|v| v.as_str()).unwrap_or_default().to_string();
            (s("clone_url"), s("default_branch"))
        })
        .unwrap_or_default()
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
    super::store::Prefs {
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
