//! The single root component: owns connection state, routes UI actions and
//! exposes the network subscription. All screens live in `templates/app.xml`
//! and switch on the `screen`/`view` context keys.

use glacier_ui::{
    ButtonRole, Component, Context, DialogButton, DialogIcon, DialogSpec, EngineMessage, Form,
    FormBuilder, FormControl, Template, ToastSpec,
};
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
    client: super::net::RwpClient,
    selected: Arc<Mutex<String>>,
    selected_deploy: Arc<Mutex<String>>,
    deploy_track: Arc<Mutex<super::net::DeployTrack>>,
    search: Arc<Mutex<String>>,
    selected_project: Arc<Mutex<String>>,
    search_cache: Arc<Mutex<super::net::SearchCache>>,
}

impl Hash for PollKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.seq.hash(state);
    }
}

pub struct Root {
    /// Normalized `rwp://…` URL (mirror of the `url` context key on connect).
    addr: String,
    token: Option<String>,
    /// Shared, lazily-(re)connected RPC connection to the daemon — every
    /// `ctx.perform` action and the poll loop clone this instead of each
    /// opening (and dropping) their own connection. Rebuilt in `submit_login`
    /// so it always matches `addr`/`token`.
    client: super::net::RwpClient,
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
    /// Id of the project currently open in the detail view (`view=project_services`).
    /// Shared with the poll loop so it can keep re-filtering `project_services`
    /// from the same `all` it already fetches, without a dedicated RPC.
    selected_project_shared: Arc<Mutex<String>>,
    /// Last raw snapshot the poll loop fetched, so `search_changed` can rebuild
    /// the filtered lists instantly on each keystroke instead of waiting for
    /// the next 2s poll tick (see `net::SearchCache`/`net::search_pairs`).
    search_cache: Arc<Mutex<super::net::SearchCache>>,
    /// RAM snapshot of the last successful service/project detail fetch, so
    /// reopening a detail view paints instantly (cache-aside) instead of showing
    /// a spinner until the fetch lands. Written through by the `*_cached` fetches
    /// (see `net::DetailCache`).
    detail_cache: super::net::DetailCacheHandle,

    // ── glacier-ui `Form`s (validated field groups) ────────────────────
    //
    // Only forms whose fields are NEVER repopulated by an async fetch outside
    // `update()` (a `ctx.perform` result lands via `EngineMessage::ContextPatch`,
    // which `GlacierUI::dispatch` merges straight into its own context and
    // *never* routes through `Component::update` — a `Form` living on `Root`
    // has no way to learn about it) get a real, persistent `Form` here, driven
    // generically by `has_control`/`set_value` in `update()`. Screens whose
    // fields DO get repopulated that way (Settings, Git provider connect, new
    // project, and every service-detail form) validate ad hoc instead, with a
    // throwaway `Form` built from the live context at submit time (see
    // `validate_ad_hoc`) — a persistent `Form` there would go stale the moment
    // a fetch lands, and the next keystroke would flush its stale values back
    // over context, clobbering whatever the fetch just wrote.
    login_form: Form,
    edit_project_form: Form,
    env_add_form: Form,
    penv_add_form: Form,
}

impl Default for Root {
    fn default() -> Self {
        Self {
            addr: String::default(),
            token: None,
            client: super::net::RwpClient::default(),
            active: false,
            seq: 0,
            selected_service: String::default(),
            selected_shared: Arc::default(),
            selected_deploy_shared: Arc::default(),
            deploy_shared: Arc::default(),
            search_shared: Arc::default(),
            selected_project_shared: Arc::default(),
            search_cache: Arc::default(),
            detail_cache: Arc::default(),
            login_form: FormBuilder::new("login")
                .control(FormControl::new("url", "").required())
                .control(FormControl::new("token", ""))
                .on_submit(|form, ctx| {
                    form.validate();
                    form.errors_to_context(ctx, "erro_");
                })
                .build(),
            edit_project_form: FormBuilder::new("edit_project")
                .control(FormControl::new("edit_proj_name", "").required())
                .control(FormControl::new("edit_proj_desc", ""))
                .on_submit(|form, ctx| {
                    form.validate();
                    form.errors_to_context(ctx, "erro_");
                })
                .build(),
            env_add_form: FormBuilder::new("env_add")
                .control(FormControl::new("env_new_key", "").required())
                .control(FormControl::new("env_new_val", ""))
                .on_submit(|form, ctx| {
                    form.validate();
                    form.errors_to_context(ctx, "erro_");
                })
                .build(),
            penv_add_form: FormBuilder::new("penv_add")
                .control(FormControl::new("penv_new_key", "").required())
                .control(FormControl::new("penv_new_val", ""))
                .on_submit(|form, ctx| {
                    form.validate();
                    form.errors_to_context(ctx, "erro_");
                })
                .build(),
        }
    }
}

/// Validates `controls` (built fresh from the live context by the caller)
/// against their validators and publishes the first error of each under
/// `"{error_prefix}{control name}"` — the ad hoc counterpart of a persistent
/// `Form` for screens whose fields can be repopulated by an async fetch (see
/// the comment on `Root`'s `Form` fields for why those can't own a real one).
/// Returns whether every control passed.
fn validate_ad_hoc(ctx: &mut Context, error_prefix: &str, controls: Vec<FormControl>) -> bool {
    let mut builder = FormBuilder::new(error_prefix);
    for c in controls {
        builder = builder.control(c);
    }
    let mut form = builder.build();
    let ok = form.validate();
    form.errors_to_context(ctx, error_prefix);
    ok
}

/// A `FormControl` seeded from the current value of context key `key`,
/// required (non-empty).
fn required_field(ctx: &Context, key: &str) -> FormControl {
    FormControl::new(key, ctx.get(key).cloned().unwrap_or_default()).required()
}

/// A `FormControl` seeded from the current value of context key `key`,
/// optional but must be numeric if non-empty (used for port/replica/interval
/// -style fields where the daemon expects an integer).
fn optional_numeric_field(ctx: &Context, key: &str) -> FormControl {
    FormControl::new(key, ctx.get(key).cloned().unwrap_or_default()).pattern(r"^$|^[0-9]+$")
}

/// Publishes a persistent `Form`'s values and cached errors into the
/// context, under `"{control name}"` / `"erro_{control name}"` — same
/// convention `validate_ad_hoc` uses for the non-persistent forms.
fn sync_form(form: &Form, ctx: &mut Context) {
    form.sync_to_context(ctx);
    form.errors_to_context(ctx, "erro_");
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
        ctx.perform(super::net::Services::new(self.client.clone()).run_action(cmd, id));
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
        ctx.perform(super::net::Services::new(self.client.clone()).start_deploy(
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
        ctx.perform(super::net::Services::new(self.client.clone()).run_env_op(
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
        ctx.perform(super::net::Services::new(self.client.clone()).run_spec_op(
            self.selected_service.clone(),
            op,
        ));
    }

    /// `Root`'s persistent `Form`s (see the field-level comment on why only
    /// these get one), for the generic `has_control`-driven field
    /// routing in `update()`.
    fn forms_mut(&mut self) -> [&mut Form; 4] {
        [
            &mut self.login_form,
            &mut self.edit_project_form,
            &mut self.env_add_form,
            &mut self.penv_add_form,
        ]
    }

    /// Runs the login form's validator, and on success connects — the body of
    /// the old `"connect"` action, now gated on `Form::is_valid` instead of a
    /// bare empty-string check.
    fn submit_login(&mut self, ctx: &mut Context) {
        self.login_form.submit(ctx);
        if !self.login_form.is_valid() {
            return;
        }
        let url = self.login_form.value("url").to_string();
        let tok = self.login_form.value("token").to_string();
        self.addr = normalize_url(&url);
        self.token = if tok.trim().is_empty() { None } else { Some(tok) };
        self.client = super::net::RwpClient::new(self.addr.clone(), self.token.clone());
        self.active = true;
        self.seq += 1;
        ctx.set("error", "");
        ctx.set("status_line", "conectando…");
        ctx.set("data_loading", "true");
        save_prefs(ctx);
    }

    /// Runs the project-edit form's validator, and on success saves — the
    /// body of the old `"save_project_edit"` action.
    fn submit_edit_project(&mut self, ctx: &mut Context) {
        self.edit_project_form.submit(ctx);
        if !self.edit_project_form.is_valid() {
            return;
        }
        let id = ctx.get("selected_project_id").cloned().unwrap_or_default();
        let name = self.edit_project_form.value("edit_proj_name").to_string();
        let desc = self.edit_project_form.value("edit_proj_desc").to_string();
        if !id.is_empty() && !self.addr.is_empty() {
            ctx.set("proj_action_msg", "salvando…");
            ctx.perform(super::net::Projects::new(self.client.clone()).update(id, name, desc));
        }
    }

    /// Runs the env-var-add form's validator, and on success adds the
    /// variable and clears the form — the body of the old `"env_add"` action.
    fn submit_env_add(&mut self, ctx: &mut Context) {
        self.env_add_form.submit(ctx);
        if !self.env_add_form.is_valid() {
            return;
        }
        let key = self.env_add_form.value("env_new_key").trim().to_string();
        let value = self.env_add_form.value("env_new_val").to_string();
        self.env_op(ctx, super::net::EnvOp::Set { key, value });
        self.env_add_form.set_value("env_new_key", "");
        self.env_add_form.set_value("env_new_val", "");
        self.env_add_form.sync_to_context(ctx);
    }

    /// Applies an edit to the PROJECT-level env vars (inherited by every
    /// service of the project at deploy time) through the async bridge.
    /// Operates on the project open in `project_services`.
    fn project_env_op(&self, ctx: &mut Context, op: super::net::ProjectEnvOp) {
        let pid = ctx.get("selected_project_id").cloned().unwrap_or_default();
        if pid.is_empty() || self.addr.is_empty() {
            return;
        }
        ctx.set("proj_action_msg", "salvando…");
        ctx.perform(super::net::Projects::new(self.client.clone()).run_env_op(pid, op));
    }

    /// Runs the project-env-add form's validator, and on success adds the
    /// variable and clears the form — mirror of [`Self::submit_env_add`] for
    /// the project's "Variáveis" tab.
    fn submit_penv_add(&mut self, ctx: &mut Context) {
        self.penv_add_form.submit(ctx);
        if !self.penv_add_form.is_valid() {
            return;
        }
        let key = self.penv_add_form.value("penv_new_key").trim().to_string();
        let value = self.penv_add_form.value("penv_new_val").to_string();
        self.project_env_op(ctx, super::net::ProjectEnvOp::Set { key, value });
        self.penv_add_form.set_value("penv_new_key", "");
        self.penv_add_form.set_value("penv_new_val", "");
        self.penv_add_form.sync_to_context(ctx);
    }

    // ── Wizard "Novo serviço" (Application / Database / Compose / Template) ──
    //
    // Estado inteiro em chaves de contexto `ns_*` (campos digitados via o
    // `field:` genérico), validação ad hoc no `ns_create` — mesmo racional das
    // service-detail forms: os campos são repovoados por ações do próprio
    // wizard (escolher banco/template pré-preenche), então um `Form`
    // persistente ficaria stale.

    /// Clears every wizard field back to a pristine state (entering the wizard
    /// or switching the picked type).
    fn ns_reset_fields(ctx: &mut Context) {
        for k in [
            "ns_name", "ns_app_name", "ns_db_name", "ns_db_user", "ns_db_password",
            "ns_db_root_password", "ns_image", "ns_msg", "ns_tsearch", "erro_ns_name",
            "ns_db_kind", "ns_db_label", "ns_template_id", "ns_template_name",
        ] {
            ctx.set(k, "");
        }
        ctx.set("ns_use_replica", "false");
        ctx.set("ns_template_vars", "[]");
        for k in ["ns_db_has_dbname", "ns_db_has_user", "ns_db_has_rootpw", "ns_db_has_replica"] {
            ctx.set(k, "false");
        }
    }

    /// Rebuilds the template catalog list from the current search term.
    fn ns_templates_refresh(ctx: &mut Context) {
        let search = ctx.get("ns_tsearch").cloned().unwrap_or_default();
        ctx.set("ns_templates", super::wizard::templates_json(&search));
    }

    /// Pre-fills the database form for the picked kind: default image and
    /// user, generated passwords, and the per-kind field-visibility flags the
    /// KDL branches on.
    fn ns_pick_db(ctx: &mut Context, db: super::wizard::DbKind) {
        ctx.set("ns_db_kind", db.kind_id());
        ctx.set("ns_db_label", db.label());
        ctx.set("ns_image", db.default_image());
        ctx.set("ns_db_name", if db.has_db_name() { "app" } else { "" });
        ctx.set("ns_db_user", db.default_user());
        ctx.set("ns_db_password", super::wizard::token_urlsafe(22));
        ctx.set("ns_db_root_password", super::wizard::token_urlsafe(22));
        ctx.set("ns_use_replica", "false");
        ctx.set("ns_db_has_dbname", bool_str(db.has_db_name()));
        ctx.set("ns_db_has_user", bool_str(db.has_user()));
        ctx.set("ns_db_has_rootpw", bool_str(db.has_root_password()));
        ctx.set("ns_db_has_replica", bool_str(db.has_replica_sets()));
        ctx.set("ns_step", "db_form");
    }

    /// Pre-fills the (reused) database form for the picked broker. Brokers ride
    /// the same `db_form` step and `ns_db_*` context keys as databases — only
    /// the visibility flags differ (no db name / root pw / replica; a user only
    /// for RabbitMQ). The picked kind lives in `ns_db_kind`, so `ns_create`/
    /// `ns_back` tell broker from database via `BrokerKind::from_str`.
    fn ns_pick_broker(ctx: &mut Context, broker: super::wizard::BrokerKind) {
        ctx.set("ns_db_kind", broker.kind_id());
        ctx.set("ns_db_label", broker.label());
        ctx.set("ns_image", broker.default_image());
        ctx.set("ns_db_name", "");
        ctx.set("ns_db_user", broker.default_user());
        ctx.set("ns_db_password", super::wizard::token_urlsafe(22));
        ctx.set("ns_db_root_password", "");
        ctx.set("ns_use_replica", "false");
        ctx.set("ns_db_has_dbname", bool_str(false));
        ctx.set("ns_db_has_user", bool_str(broker.has_user()));
        ctx.set("ns_db_has_rootpw", bool_str(false));
        ctx.set("ns_db_has_replica", bool_str(false));
        ctx.set("ns_step", "db_form");
    }

    /// Loads the picked template into the form step: default service name and
    /// one `ns_tv_<idx>` context key per configurable variable (the KDL's
    /// `ForEach` binds each input to its key by interpolation).
    fn ns_pick_template(ctx: &mut Context, id: &str) {
        let Some(t) = super::wizard::find_template(id) else {
            return;
        };
        ctx.set("ns_template_id", t.id);
        ctx.set("ns_template_name", t.name);
        ctx.set("ns_name", super::wizard::template_slug(t));
        ctx.set("ns_template_vars", super::wizard::template_vars_json(t));
        // Uma chave por variável editável (só os domínios); o usuário preenche,
        // então nasce vazia (o placeholder mostra a dica).
        for i in 0..shared::templates::editable_vars(t).len() {
            ctx.set(&format!("ns_tv_{i}"), "");
        }
        ctx.set("ns_step", "template_form");
    }

    /// One step back in the wizard flow (mirrors the old `remote-client`'s
    /// `NsBack`).
    fn ns_back(ctx: &mut Context) {
        let step = ctx.get("ns_step").cloned().unwrap_or_default();
        match step.as_str() {
            "pick_db" | "pick_broker" | "app_form" | "compose_form" | "pick_template" => {
                ctx.set("ns_step", "pick_type");
            }
            // O `db_form` é compartilhado por bancos e brokers; volta para o
            // picker de origem conforme o tipo escolhido (gravado em ns_db_kind).
            "db_form" => {
                let kind = ctx.get("ns_db_kind").cloned().unwrap_or_default();
                if super::wizard::BrokerKind::from_str(&kind).is_some() {
                    ctx.set("ns_step", "pick_broker");
                } else {
                    ctx.set("ns_step", "pick_db");
                }
            }
            "template_form" => ctx.set("ns_step", "pick_template"),
            // No primeiro passo, voltar = sair do wizard.
            _ => {
                ctx.set("view", "project_services");
                ctx.set("ns_step", "");
            }
        }
        ctx.set("ns_msg", "");
    }

    /// Validates the current wizard step, builds the `ServiceSpec` and sends
    /// `Command::ServiceCreate` — the `ns_create` action.
    fn submit_ns_create(&mut self, ctx: &mut Context) {
        if self.addr.is_empty() {
            return;
        }
        let project_id = ctx.get("selected_project_id").cloned().unwrap_or_default();
        if project_id.is_empty() {
            return;
        }
        let step = ctx.get("ns_step").cloned().unwrap_or_default();

        // O nome é obrigatório, exceto no template (que tem um slug default).
        if step != "template_form"
            && !validate_ad_hoc(ctx, "erro_", vec![required_field(ctx, "ns_name")])
        {
            return;
        }
        let g = |k: &str| ctx.get(k).cloned().unwrap_or_default();
        // "App Name" opcional tem precedência sobre o nome, como no
        // `remote-client` (NsForm::to_spec).
        let name = if g("ns_app_name").trim().is_empty() {
            g("ns_name").trim().to_string()
        } else {
            g("ns_app_name").trim().to_string()
        };

        // Pré-check de nome duplicado no projeto (o backend também valida — ver
        // db::services::create). Compara por nome normalizado, que é o que vira
        // o container/DNS `rp_<safe_name>`.
        let new_safe = shared::normalize_name(&name);
        let existing: Vec<String> = ctx
            .get("proj_service_names")
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default();
        if existing.iter().any(|n| shared::normalize_name(n) == new_safe) {
            ctx.set(
                "ns_msg",
                format!("já existe um serviço com o nome \"{name}\" neste projeto"),
            );
            return;
        }

        let spec = match step.as_str() {
            "app_form" => Some(super::wizard::app_spec(name, project_id)),
            "compose_form" => Some(super::wizard::compose_spec(name, project_id)),
            // `db_form` é compartilhado por bancos e brokers (mesmos campos
            // ns_db_*). Tenta banco; se o kind_id for de broker, monta o broker.
            "db_form" => {
                let kind = g("ns_db_kind");
                let input = super::wizard::DbFormInput {
                    db_name: g("ns_db_name").trim().to_string(),
                    user: g("ns_db_user").trim().to_string(),
                    password: g("ns_db_password").trim().to_string(),
                    root_password: g("ns_db_root_password").trim().to_string(),
                    image: g("ns_image"),
                    use_replica_sets: g("ns_use_replica") == "true",
                };
                if let Some(db) = super::wizard::DbKind::from_str(&kind) {
                    Some(super::wizard::db_spec(db, name, project_id, &input))
                } else {
                    super::wizard::BrokerKind::from_str(&kind)
                        .map(|b| super::wizard::broker_spec(b, name, project_id, &input))
                }
            }
            "template_form" => super::wizard::find_template(&g("ns_template_id")).map(|t| {
                let values: Vec<String> = (0..shared::templates::editable_vars(t).len())
                    .map(|i| g(&format!("ns_tv_{i}")))
                    .collect();
                super::wizard::template_spec(t, name, project_id, &values)
            }),
            _ => None,
        };
        if let Some(spec) = spec {
            ctx.set("ns_msg", "criando…");
            ctx.perform(super::net::Services::new(self.client.clone()).create(spec));
        }
    }

    /// Validates `ss_email` ad hoc (see the comment on `Root`'s `Form`
    /// fields — Settings is repopulated by an async fetch on connect, so it
    /// can't own a persistent `Form`), and on success saves — the body of the
    /// old `"settings_save"` action.
    fn submit_settings(&mut self, ctx: &mut Context) {
        if self.addr.is_empty() {
            return;
        }
        let domain = ctx.get("ss_domain").cloned().unwrap_or_default();
        let email = ctx.get("ss_email").cloned().unwrap_or_default();
        let email_field = FormControl::new("ss_email", email.clone())
            .pattern(r"^$|^[^@\s]+@[^@\s]+\.[^@\s]+$");
        if !validate_ad_hoc(ctx, "erro_", vec![email_field]) {
            return;
        }
        ctx.set("settings_msg", "salvando…");
        ctx.perform(super::net::Daemon::new(self.client.clone()).save_settings(domain, email));
    }

    /// Validates `gp_name` ad hoc (Git provider connect is cleared by an
    /// async fetch on success, same reasoning as Settings), and on success
    /// connects — the body of the old `"gp_connect"` action.
    fn submit_git_provider(&mut self, ctx: &mut Context) {
        if self.addr.is_empty() {
            return;
        }
        let g = |k: &str| ctx.get(k).cloned().unwrap_or_default();
        let (name, base_url, mode, client_id, client_secret, pat) = (
            g("gp_name"), g("gp_base_url"), g("gp_mode"),
            g("gp_client_id"), g("gp_client_secret"), g("gp_pat"),
        );
        if !validate_ad_hoc(ctx, "erro_", vec![required_field(ctx, "gp_name")]) {
            return;
        }
        ctx.set("gp_msg", "conectando…");
        ctx.perform(super::net::GitProviders::new(self.client.clone()).connect(
            name, base_url, mode, client_id, client_secret, pat,
        ));
    }

    /// Validates `new_proj_name` ad hoc (the fields are cleared by an async
    /// fetch on success, same reasoning as Settings), and on success creates
    /// the project — the body of the old `"create_project"` action.
    fn submit_new_project(&mut self, ctx: &mut Context) {
        if self.addr.is_empty() {
            return;
        }
        let name = ctx.get("new_proj_name").cloned().unwrap_or_default();
        let desc = ctx.get("new_proj_desc").cloned().unwrap_or_default();
        if !validate_ad_hoc(ctx, "erro_", vec![required_field(ctx, "new_proj_name")]) {
            return;
        }
        let search = ctx.get("search").cloned().unwrap_or_default();
        ctx.perform(super::net::Projects::new(self.client.clone()).create(name, desc, search));
    }

    /// Adiciona uma rota de domínio (`dom_add`). Valida a porta opcional ad hoc
    /// (service-detail forms são repovoados pelo fetch do `open_service`, então
    /// não têm `Form` persistente — ver o comentário nos `Form` de `Root`) e,
    /// no sucesso, limpa os campos do formulário de adição.
    fn submit_domain_add(&mut self, ctx: &mut Context) {
        let fields = vec![optional_numeric_field(ctx, "f_port")];
        if !validate_ad_hoc(ctx, "erro_", fields) {
            return;
        }
        let domain = ctx.get("f_domain").cloned().unwrap_or_default();
        if domain.trim().is_empty() {
            ctx.set("erro_f_domain", "informe um domínio");
            return;
        }
        ctx.set("erro_f_domain", "");
        let op = super::net::SpecOp::DomainAdd {
            domain,
            port: ctx.get("f_port").cloned().unwrap_or_default(),
            tls: ctx.get("f_tls").map(|v| v == "true").unwrap_or(false),
        };
        self.spec_op(ctx, op);
        ctx.set("f_domain", "");
        ctx.set("f_port", "");
        ctx.set("f_tls", "false");
    }

    /// Remove a rota de domínio `domain` (`dom_del:{domain}`).
    fn domain_remove(&self, ctx: &mut Context, domain: String) {
        self.spec_op(ctx, super::net::SpecOp::DomainRemove { domain });
    }

    /// Salva a porta TCP crua do host (`dom_hostport_save`).
    fn submit_host_port(&mut self, ctx: &mut Context) {
        let fields = vec![optional_numeric_field(ctx, "f_host_port")];
        if !validate_ad_hoc(ctx, "erro_", fields) {
            return;
        }
        let op = super::net::SpecOp::HostPort {
            host_port: ctx.get("f_host_port").cloned().unwrap_or_default(),
        };
        self.spec_op(ctx, op);
    }

    /// Validates the numeric healthcheck fields ad hoc, and on success saves
    /// — the body of the old `"hc_save"` action.
    fn submit_healthcheck(&mut self, ctx: &mut Context) {
        let fields = vec![
            optional_numeric_field(ctx, "f_hc_status"),
            optional_numeric_field(ctx, "f_hc_interval"),
            optional_numeric_field(ctx, "f_hc_timeout"),
            optional_numeric_field(ctx, "f_hc_retries"),
            optional_numeric_field(ctx, "f_hc_start"),
        ];
        if !validate_ad_hoc(ctx, "erro_", fields) {
            return;
        }
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

    /// Validates `f_replicas` ad hoc, and on success saves — the body of the
    /// old `"adv_save"` action.
    fn submit_advanced(&mut self, ctx: &mut Context) {
        let fields = vec![optional_numeric_field(ctx, "f_replicas")];
        if !validate_ad_hoc(ctx, "erro_", fields) {
            return;
        }
        let op = super::net::SpecOp::Advanced {
            replicas: ctx.get("f_replicas").cloned().unwrap_or_default(),
            run_command: ctx.get("f_run_command").cloned().unwrap_or_default(),
        };
        self.spec_op(ctx, op);
    }

    /// Validates `f_gen_port` ad hoc, and on success saves — the body of the
    /// old `"gen_save"` action. No validators beyond that: the Git and Gitea
    /// sub-tabs share these keys but populate `f_repo_url`/`f_branch`
    /// differently (typed vs. derived from a picker), so a stricter
    /// `required()` here could reject a legitimate Gitea-derived save.
    fn submit_general(&mut self, ctx: &mut Context) {
        let fields = vec![optional_numeric_field(ctx, "f_gen_port")];
        if !validate_ad_hoc(ctx, "erro_", fields) {
            return;
        }
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

    /// Salva o YAML editado de um serviço Compose (`compose_save`).
    fn submit_compose(&mut self, ctx: &mut Context) {
        let content = ctx.get("svc_compose").cloned().unwrap_or_default();
        self.spec_op(ctx, super::net::SpecOp::Compose { content });
    }
}

impl Component for Root {
    fn name(&self) -> &str {
        "app"
    }

    fn template(&self) -> Template {
        Template::File("crates/rustploy-gui/templates/app.xml".into())
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
        self.login_form.set_value("url", url);
        self.login_form.set_value("token", token);
        sync_form(&self.login_form, ctx);
        sync_form(&self.edit_project_form, ctx);
        sync_form(&self.env_add_form, ctx);
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
        // Projeto aberto (view=project_services): sub-abas Serviços | Variáveis.
        ctx.set("proj_tab", "services");
        ctx.set("proj_env", "[]");
        ctx.set("proj_env_count", "0");
        // Wizard "Novo serviço" (view=new_service) — ver os métodos `ns_*`.
        ctx.set("ns_step", "");
        ctx.set("ns_msg", "");
        ctx.set("ns_templates", "[]");
        ctx.set("ns_dbs", super::wizard::db_rows_json());
        ctx.set("ns_brokers", super::wizard::broker_rows_json());
        Self::ns_reset_fields(ctx);
        // Home screens (Monitoring / Ingress / Docker / Settings).
        ctx.set("ingress", "[]");
        ctx.set("ingress_count", "0");
        // Deploy Engine.
        ctx.set("eng_active", "[]");
        ctx.set("eng_recent", "[]");
        ctx.set("eng_active_count", "0");
        ctx.set("eng_recent_count", "0");
        ctx.set("eng_success_24h", "0");
        ctx.set("eng_failed_24h", "0");
        ctx.set("eng_total_24h", "0");
        ctx.set("eng_uptime", "—");
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
        ctx.set("svc_project_id", "");
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
        ctx.set("env_text_open", "false");
        ctx.set("svc_env_text_orig", "");
        // Editable spec form fields.
        ctx.set("f_domain", "");
        ctx.set("f_port", "");
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
        // `ss_domain` also keeps the OAuth Redirect URI (Settings → Git) in
        // sync — special-cased ahead of the generic form-field loop below
        // since `ss_domain`/`ss_email` aren't a persistent `Form` (see the
        // comment on `Root`'s `Form` fields): they're repopulated by an async
        // fetch on connect, so they're just plain context keys here, same as
        // before.
        if action == "ss_domain" {
            if let Some(v) = value {
                ctx.set("ss_domain", v);
                ctx.set("gp_redirect", super::net::oauth_redirect_uri(v));
            }
            return;
        }
        // Generic routing for `Root`'s persistent `Form`s: any action that
        // names one of their controls updates it and re-publishes the form's
        // values/errors, without a per-field match arm.
        for form in self.forms_mut() {
            if form.has_control(action) {
                form.set_value(action, value.unwrap_or_default());
                sync_form(form, ctx);
                return;
            }
        }
        match action {
            // Topbar: Deploy points the user at the project grid to pick a
            // service; Stop All stops every running service.
            "deploy" => {
                ctx.set("view", "projects");
            }
            "stop_all" => {
                if !self.addr.is_empty() {
                    ctx.show_dialog(confirm_dialog(
                        "Parar todos os serviços?",
                        "Todos os serviços em execução serão parados agora. Você pode reiniciá-los depois com um novo deploy.",
                        "Parar todos",
                        "do_stop_all",
                    ));
                }
            }
            "do_stop_all" => {
                if !self.addr.is_empty() {
                    ctx.set("status_line", "parando todos…");
                    ctx.perform(super::net::Daemon::new(self.client.clone()).stop_all());
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
                    // Instant filter: rebuild the affected lists from the last
                    // poll snapshot right now, instead of waiting up to 2s for
                    // the next poll tick (see `net::search_pairs`). No-op before
                    // the first poll has populated the cache.
                    let pid = ctx.get("selected_project_id").cloned().unwrap_or_default();
                    if let Ok(cache) = self.search_cache.lock() {
                        if !cache.is_empty() {
                            for (k, val) in super::net::search_pairs(&cache, v, &pid) {
                                ctx.set(&k, val);
                            }
                        }
                    }
                }
            }
            // Docker tab: remove unused images/volumes/networks, then refresh
            // that sub-tab immediately (the perform's own pairs include it —
            // no need to wait for the next poll tick).
            "docker_prune_images" => {
                if !self.addr.is_empty() {
                    ctx.show_dialog(confirm_dialog(
                        "Limpar imagens sem uso?",
                        "As imagens Docker sem uso serão removidas agora. Essa limpeza não pode ser desfeita.",
                        "Limpar",
                        "do_docker_prune_images",
                    ));
                }
            }
            "do_docker_prune_images" => {
                if !self.addr.is_empty() {
                    let all = ctx.get("docker_prune_all_images").map(|v| v == "true").unwrap_or(false);
                    ctx.set("docker_msg", if all { "removendo TODAS as imagens sem uso…" } else { "removendo imagens dangling…" });
                    ctx.perform(super::net::Docker::new(self.client.clone()).prune_images(all));
                }
            }
            "docker_prune_volumes" => {
                if !self.addr.is_empty() {
                    ctx.show_dialog(confirm_dialog(
                        "Limpar volumes sem uso?",
                        "Os volumes Docker sem uso serão removidos agora. Essa limpeza não pode ser desfeita.",
                        "Limpar",
                        "do_docker_prune_volumes",
                    ));
                }
            }
            "do_docker_prune_volumes" => {
                if !self.addr.is_empty() {
                    let all = ctx.get("docker_prune_all_volumes").map(|v| v == "true").unwrap_or(false);
                    ctx.set("docker_msg", "removendo volumes sem uso…");
                    ctx.perform(super::net::Docker::new(self.client.clone()).prune_volumes(all));
                }
            }
            "docker_prune_networks" => {
                if !self.addr.is_empty() {
                    ctx.show_dialog(confirm_dialog(
                        "Limpar redes sem uso?",
                        "As redes Docker sem uso serão removidas agora. Essa limpeza não pode ser desfeita.",
                        "Limpar",
                        "do_docker_prune_networks",
                    ));
                }
            }
            "do_docker_prune_networks" => {
                if !self.addr.is_empty() {
                    ctx.set("docker_msg", "removendo redes sem uso…");
                    ctx.perform(super::net::Docker::new(self.client.clone()).prune_networks());
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
            "ss_email" => {
                if let Some(v) = value {
                    ctx.set("ss_email", v);
                }
            }
            "settings_save" => self.submit_settings(ctx),
            "env_add" => self.submit_env_add(ctx),
            // Env vars de nível de projeto (aba "Variáveis" do projeto aberto).
            "penv_add" => self.submit_penv_add(ctx),
            "penv_reorder" => {
                if let Some(v) = value {
                    if let Ok(keys) = serde_json::from_str::<Vec<String>>(v) {
                        self.project_env_op(ctx, super::net::ProjectEnvOp::Reorder(keys));
                    }
                }
            }
            // Wizard "Novo serviço" (aberto a partir do projeto).
            "new_service_open" => {
                let pid = ctx.get("selected_project_id").cloned().unwrap_or_default();
                if !pid.is_empty() {
                    Self::ns_reset_fields(ctx);
                    ctx.set("ns_step", "pick_type");
                    ctx.set("view", "new_service");
                }
            }
            "ns_back" => Self::ns_back(ctx),
            "ns_cancel" => {
                ctx.set("view", "project_services");
                ctx.set("ns_step", "");
                ctx.set("ns_msg", "");
            }
            "ns_create" => self.submit_ns_create(ctx),
            // Busca do catálogo de templates (onChange do input).
            "ns_tsearch" => {
                if let Some(v) = value {
                    ctx.set("ns_tsearch", v);
                    Self::ns_templates_refresh(ctx);
                }
            }
            // `env_reorder` — the glacier-ui drag-and-drop `onReorder` action
            // for the Environment tab's `kv_list` (see `service.xml`), value
            // is a JSON array of `key`s in their new order.
            "env_reorder" => {
                if let Some(v) = value {
                    if let Ok(keys) = serde_json::from_str::<Vec<String>>(v) {
                        self.env_op(ctx, super::net::EnvOp::Reorder(keys));
                    }
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
            "connect" => self.submit_login(ctx),
            "disconnect" => {
                self.active = false;
                self.seq += 1;
                ctx.set("connected", "false");
                ctx.set("screen", "login");
                ctx.set("status_line", "desconectado");
                ctx.show_toast(ToastSpec::info("Desconectado."));
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
                ctx.show_dialog(confirm_dialog(
                    "Parar serviço?",
                    "O tráfego para este serviço será interrompido até um novo deploy.",
                    "Parar",
                    "do_svc_stop",
                ));
            }
            "do_svc_stop" => {
                self.service_action(ctx, |id| shared::Command::ServiceStop { service_id: id });
            }
            // Save handlers for the editable spec forms.
            "dom_add" => self.submit_domain_add(ctx),
            "dom_hostport_save" => self.submit_host_port(ctx),
            "hc_save" => self.submit_healthcheck(ctx),
            "adv_save" => self.submit_advanced(ctx),
            "gen_save" => self.submit_general(ctx),
            "compose_save" => self.submit_compose(ctx),
            "compose_cancel" => {
                let orig = ctx.get("svc_compose_orig").cloned().unwrap_or_default();
                ctx.set("svc_compose", orig);
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
                        ctx.perform(super::net::GitProviders::new(self.client.clone()).fetch_repos(id.to_string()));
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
                        ctx.perform(super::net::GitProviders::new(self.client.clone()).fetch_branches(
                            pid,
                            full_name.to_string(),
                        ));
                    }
                }
            }
            // Settings → Git: connect a provider, refresh, switch method.
            "gp_connect" => self.submit_git_provider(ctx),
            "gp_refresh" => {
                if !self.addr.is_empty() {
                    ctx.perform(super::net::GitProviders::new(self.client.clone()).refresh());
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
                    // Leaving `project_services`: stop re-filtering it on every tick.
                    if let Ok(mut p) = self.selected_project_shared.lock() {
                        p.clear();
                    }
                    ctx.close_dialog();
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
                    ctx.set("svc_error", "");
                    ctx.set("dep_selected", "");
                    if let Ok(mut d) = self.selected_deploy_shared.lock() {
                        d.clear();
                    }
                    // Cache-aside: se já temos o último detalhe deste serviço em
                    // RAM, pinta na hora (as chaves incluem svc_loading=false) e o
                    // fetch abaixo só refresca em background; senão mostra o spinner.
                    let cached = self.detail_cache.lock().ok().and_then(|c| c.service(id));
                    match cached {
                        Some(pairs) => {
                            for (k, v) in pairs {
                                ctx.set(&k, v);
                            }
                        }
                        None => ctx.set("svc_loading", "true"),
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
                    let sid = id.to_string();
                    if !self.addr.is_empty() {
                        // One-shot load of everything the detail needs — including
                        // the Gitea provider/repo/branch lists — written through to
                        // `detail_cache` so the next open of this service is instant.
                        ctx.perform(super::net::Services::new(self.client.clone()).fetch_detail_cached(
                            self.detail_cache.clone(),
                            sid,
                        ));
                    }
                    return;
                }
                // `open_project:<id>` — open a project's service list and fetch it.
                // Also reachable as the service detail's "‹ Projects" back button
                // (with the open service's own project id), so it clears the
                // detail view's selection too — same cleanup as `nav_*` — in case
                // it's navigating away from `view=service`, not just the grid.
                if let Some(id) = action.strip_prefix("open_project:") {
                    self.selected_service.clear();
                    if let Ok(mut sel) = self.selected_shared.lock() {
                        sel.clear();
                    }
                    if let Ok(mut d) = self.selected_deploy_shared.lock() {
                        d.clear();
                    }
                    if let Ok(mut sel) = self.selected_project_shared.lock() {
                        *sel = id.to_string();
                    }
                    ctx.set("selected_project_id", id);
                    ctx.set("view", "project_services");
                    ctx.set("proj_tab", "services");
                    ctx.set("editing_project", "false");
                    ctx.set("proj_action_msg", "");
                    // Cache-aside: pinta a lista de serviços do último snapshot em
                    // RAM (proj_loading=false vem nas chaves), refresca abaixo.
                    let cached = self.detail_cache.lock().ok().and_then(|c| c.project(id));
                    match cached {
                        Some(pairs) => {
                            for (k, v) in pairs {
                                ctx.set(&k, v);
                            }
                        }
                        None => ctx.set("proj_loading", "true"),
                    }
                    if !self.addr.is_empty() {
                        ctx.perform(super::net::Projects::new(self.client.clone()).fetch_services_cached(
                            self.detail_cache.clone(),
                            id.to_string(),
                        ));
                    }
                    return;
                }
                // `edit_project_toggle` — reveal the inline name/description form,
                // seeded from the currently displayed project.
                if action == "edit_project_toggle" {
                    self.edit_project_form.set_value("edit_proj_name", ctx.get("proj_name").cloned().unwrap_or_default());
                    self.edit_project_form.set_value("edit_proj_desc", ctx.get("proj_description").cloned().unwrap_or_default());
                    sync_form(&self.edit_project_form, ctx);
                    ctx.set("editing_project", "true");
                    return;
                }
                if action == "cancel_project_edit" {
                    ctx.set("editing_project", "false");
                    return;
                }
                if action == "save_project_edit" {
                    self.submit_edit_project(ctx);
                    return;
                }
                // `create_project` — Projects tab "Novo projeto" bar.
                if action == "create_project" {
                    self.submit_new_project(ctx);
                    return;
                }
                // `svc_stop_id:<id>` — "Parar" a service listed under a project
                // (any card, not just the one open in the detail view).
                if let Some(id) = action.strip_prefix("svc_stop_id:") {
                    ctx.show_dialog(confirm_dialog(
                        "Parar serviço?",
                        "O tráfego para este serviço será interrompido até um novo deploy.",
                        "Parar",
                        format!("do_svc_stop_id:{id}"),
                    ));
                    return;
                }
                if let Some(id) = action.strip_prefix("do_svc_stop_id:") {
                    if !self.addr.is_empty() {
                        ctx.set("proj_action_msg", "enviando…");
                        let pid = ctx.get("selected_project_id").cloned().unwrap_or_default();
                        ctx.perform(super::net::Projects::new(self.client.clone()).run_service_action(
                            shared::Command::ServiceStop { service_id: id.to_string() },
                            pid,
                        ));
                    }
                    return;
                }
                // Destructive actions show a confirmation dialog (glacier-ui's
                // `dialogs` module) instead of running immediately; the dialog's
                // confirm button carries the `do_*`-prefixed action below.
                if let Some(id) = action.strip_prefix("delete_project:") {
                    ctx.show_dialog(confirm_dialog(
                        "Remover projeto?",
                        "Essa ação não pode ser desfeita.",
                        "Remover",
                        format!("do_delete_project:{id}"),
                    ));
                    return;
                }
                if let Some(id) = action.strip_prefix("do_delete_project:") {
                    if !self.addr.is_empty() {
                        ctx.set("proj_action_msg", "removendo…");
                        let search = ctx.get("search").cloned().unwrap_or_default();
                        ctx.perform(super::net::Projects::new(self.client.clone()).delete(id.to_string(), search));
                    }
                    return;
                }
                if let Some(id) = action.strip_prefix("stop_delete_service:") {
                    ctx.show_dialog(confirm_dialog(
                        "Parar e remover serviço?",
                        "Essa ação não pode ser desfeita.",
                        "Remover",
                        format!("do_stop_delete_service:{id}"),
                    ));
                    return;
                }
                if let Some(id) = action.strip_prefix("do_stop_delete_service:") {
                    if !self.addr.is_empty() {
                        ctx.set("proj_action_msg", "removendo…");
                        let pid = ctx.get("selected_project_id").cloned().unwrap_or_default();
                        ctx.perform(super::net::Projects::new(self.client.clone()).stop_and_delete_service(
                            id.to_string(),
                            pid,
                        ));
                    }
                    return;
                }
                if let Some(id) = action.strip_prefix("delete_deployment:") {
                    ctx.show_dialog(confirm_dialog(
                        "Remover deployment?",
                        "Os logs de build deste deployment também serão apagados. Essa ação não pode ser desfeita.",
                        "Remover",
                        format!("do_delete_deployment:{id}"),
                    ));
                    return;
                }
                if let Some(id) = action.strip_prefix("do_delete_deployment:") {
                    if !self.addr.is_empty() {
                        // Close the build-log panel if it's open for the
                        // deployment being removed — its content is gone.
                        if ctx.get("dep_selected").map(|s| s == id).unwrap_or(false) {
                            ctx.set("dep_selected", "");
                            ctx.set("dep_build_logs", "[]");
                            ctx.set("dep_build_count", "0");
                            if let Ok(mut sel) = self.selected_deploy_shared.lock() {
                                sel.clear();
                            }
                        }
                        ctx.set("svc_action_msg", "removendo…");
                        ctx.perform(super::net::Services::new(self.client.clone()).delete_deployment(
                            self.selected_service.clone(),
                            id.to_string(),
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
                        ctx.perform(super::net::Services::new(self.client.clone()).fetch_build_logs(id.to_string()));
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
                            ctx.perform(super::net::GitProviders::new(self.client.clone()).fetch_repos(pid));
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
                    ctx.show_dialog(confirm_dialog(
                        "Remover provedor Git?",
                        "Serviços vinculados a esta conta perdem a integração de auto-deploy.",
                        "Remover",
                        format!("do_gp_delete:{id}"),
                    ));
                    return;
                }
                if let Some(id) = action.strip_prefix("do_gp_delete:") {
                    if !self.addr.is_empty() {
                        ctx.set("gp_msg", "removendo…");
                        ctx.perform(super::net::GitProviders::new(self.client.clone()).delete(id.to_string()));
                    }
                    return;
                }
                // `env_del:<key>` — remove an environment variable.
                if let Some(key) = action.strip_prefix("env_del:") {
                    self.env_op(ctx, super::net::EnvOp::Delete { key: key.to_string() });
                    return;
                }
                // `dom_del:<domain>` — remove uma rota de domínio do serviço.
                if let Some(domain) = action.strip_prefix("dom_del:") {
                    self.domain_remove(ctx, domain.to_string());
                    return;
                }
                // `penv_del:<key>` — remove a PROJECT-level environment variable.
                if let Some(key) = action.strip_prefix("penv_del:") {
                    self.project_env_op(ctx, super::net::ProjectEnvOp::Delete { key: key.to_string() });
                    return;
                }
                // `proj_tab:<services|env>` — sub-aba do projeto aberto.
                if let Some(t) = action.strip_prefix("proj_tab:") {
                    ctx.set("proj_tab", t);
                    return;
                }
                // `ns_kind:<application|database|compose|template>` — passo 1
                // do wizard: escolher o tipo do novo serviço.
                if let Some(kind) = action.strip_prefix("ns_kind:") {
                    ctx.set("ns_msg", "");
                    match kind {
                        "application" => ctx.set("ns_step", "app_form"),
                        "compose" => ctx.set("ns_step", "compose_form"),
                        "database" => ctx.set("ns_step", "pick_db"),
                        "broker" => ctx.set("ns_step", "pick_broker"),
                        "template" => {
                            ctx.set("ns_tsearch", "");
                            Self::ns_templates_refresh(ctx);
                            ctx.set("ns_step", "pick_template");
                        }
                        _ => {}
                    }
                    return;
                }
                // `ns_db:<kind_id>` — escolher o banco (pré-preenche o form).
                if let Some(id) = action.strip_prefix("ns_db:") {
                    if let Some(db) = super::wizard::DbKind::from_str(id) {
                        Self::ns_pick_db(ctx, db);
                    }
                    return;
                }
                // `ns_broker:<kind_id>` — escolher o broker (reusa o db_form).
                if let Some(id) = action.strip_prefix("ns_broker:") {
                    if let Some(b) = super::wizard::BrokerKind::from_str(id) {
                        Self::ns_pick_broker(ctx, b);
                    }
                    return;
                }
                // `ns_pick_template:<id>` — escolher o template do catálogo.
                if let Some(id) = action.strip_prefix("ns_pick_template:") {
                    Self::ns_pick_template(ctx, id);
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
                    ctx.show_dialog(confirm_dialog(
                        "Sobrescrever variáveis de ambiente?",
                        "O conteúdo do editor vai substituir TODAS as variáveis atuais deste serviço.",
                        "Sobrescrever",
                        "do_env_import",
                    ));
                    return;
                }
                if action == "do_env_import" {
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
                        Ok(_) => {
                            ctx.set("svc_action_msg", format!("exportado para {path}"));
                            ctx.show_toast(ToastSpec::success(format!("Variáveis exportadas para {path}")));
                        }
                        Err(e) => {
                            ctx.set("svc_action_msg", format!("erro ao exportar: {e}"));
                            ctx.show_toast(ToastSpec::error(format!("Falha ao exportar: {e}")));
                        }
                    }
                }
            }
        }
    }

    /// Enter-triggered submit of any `<Form>` (a `formControl`-bound
    /// `TextInput`'s Enter key always dispatches its enclosing `Form`'s
    /// `onSubmit`, routed here instead of `update()` — see
    /// `glacier_ui::Component::on_form_submit`). Each arm mirrors the
    /// corresponding Save/Connect/Criar button's `onClick` in `update()`, so
    /// pressing Enter and clicking the button behave identically.
    fn on_form_submit(&mut self, action: &str, ctx: &mut Context) {
        match action {
            "connect" => self.submit_login(ctx),
            "save_project_edit" => self.submit_edit_project(ctx),
            "env_add" => self.submit_env_add(ctx),
            "penv_add" => self.submit_penv_add(ctx),
            "ns_create" => self.submit_ns_create(ctx),
            "settings_save" => self.submit_settings(ctx),
            "gp_connect" => self.submit_git_provider(ctx),
            "create_project" => self.submit_new_project(ctx),
            "dom_add" => self.submit_domain_add(ctx),
            "dom_hostport_save" => self.submit_host_port(ctx),
            "hc_save" => self.submit_healthcheck(ctx),
            "adv_save" => self.submit_advanced(ctx),
            "gen_save" => self.submit_general(ctx),
            "compose_save" => self.submit_compose(ctx),
            "compose_cancel" => {
                let orig = ctx.get("svc_compose_orig").cloned().unwrap_or_default();
                ctx.set("svc_compose", orig);
            }
            _ => {}
        }
    }

    fn subscription(&self) -> iced::Subscription<EngineMessage> {
        if self.active && !self.addr.is_empty() {
            let key = PollKey {
                seq: self.seq,
                client: self.client.clone(),
                selected: self.selected_shared.clone(),
                selected_deploy: self.selected_deploy_shared.clone(),
                deploy_track: self.deploy_shared.clone(),
                search: self.search_shared.clone(),
                selected_project: self.selected_project_shared.clone(),
                search_cache: self.search_cache.clone(),
            };
            iced::Subscription::run_with(key, |k: &PollKey| {
                super::net::poll_stream(
                    k.client.clone(),
                    k.selected.clone(),
                    k.selected_deploy.clone(),
                    k.deploy_track.clone(),
                    k.search.clone(),
                    k.selected_project.clone(),
                    k.search_cache.clone(),
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

/// Builds a confirmation dialog for a destructive/disruptive action:
/// "Cancelar" (no-op, matches nothing in `Root::update`) plus a labeled
/// confirm button whose `action` is only reached if the user actually
/// clicks it — dismissing (backdrop click) or "Cancelar" never fires it.
fn confirm_dialog(
    title: &str,
    message: &str,
    confirm_label: &str,
    confirm_action: impl Into<String>,
) -> DialogSpec {
    DialogSpec::new(DialogIcon::Warning, title, message)
        .dismissible(true)
        .with_button(DialogButton::new("Cancelar", "noop", ButtonRole::Neutral))
        .with_button(DialogButton::new(confirm_label, confirm_action, ButtonRole::Destructive))
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
