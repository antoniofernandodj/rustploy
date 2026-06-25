//! Message handling: mirrors the TUI's command dispatch, response handling and
//! event application.

use crate::model::*;
use iced::Task;
use shared::{Command, Event, Response, Service, ServiceSource, ServiceSpec, ServiceStatus};

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // ── Connection ────────────────────────────────────────────────
            Message::UrlChanged(v) => self.url = v,
            Message::TokenChanged(v) => self.token = v,
            Message::RememberUrlToggled(v) => {
                self.remember_url = v;
                self.persist_prefs();
            }
            Message::RememberTokenToggled(v) => {
                self.remember_token = v;
                self.persist_prefs();
            }
            Message::Connect => {
                // Canonicalize to an `rwp://…` URL, keeping the authority verbatim
                // (no port baked in); resolve the TCP target separately. Prefs are
                // only persisted once the connection actually succeeds.
                self.url = crate::model::normalize_url(&self.url);
                let target = match crate::model::connect_target(&self.url) {
                    Ok(t) => t,
                    Err(e) => {
                        self.session = None;
                        self.connected = false;
                        self.error = Some(e.to_string());
                        self.status_msg = "URL inválida".into();
                        return Task::none();
                    }
                };
                self.connect_seq += 1;
                self.connected = false;
                self.worker_tx = None;
                self.error = None;
                self.reset_data();
                self.status_msg = "Conectando…".into();
                let token = if self.token.trim().is_empty() {
                    None
                } else {
                    Some(self.token.clone())
                };
                self.session = Some(Session { addr: target, token });
            }
            Message::Disconnect => {
                self.session = None;
                self.worker_tx = None;
                self.connected = false;
                self.status_msg = "Desconectado".into();
                self.reset_data();
            }
            Message::Worker(ev) => self.handle_worker(ev),
            Message::Tick => self.on_tick(),
            Message::DismissNotification => self.notification = None,

            // ── Navigation ────────────────────────────────────────────────
            Message::Sidebar(item) => self.select_sidebar(item),
            Message::OpenProject(id) => self.open_project(id),
            Message::BackToProjects => self.view = View::Projects,
            Message::OpenService(id) => self.open_service(id),
            Message::BackToProject => self.view = View::ProjectDetail,
            Message::ProjectTab(tab) => {
                self.project_tab = tab;
                if let Some(pid) = self.active_project_id.clone() {
                    match tab {
                        ProjectTab::Services | ProjectTab::Settings => {
                            self.send(Ctx::Services, Command::ServiceList { project_id: pid });
                        }
                        ProjectTab::Secrets => {
                            self.send(Ctx::Secrets, Command::SecretList { project_id: pid });
                        }
                        ProjectTab::Environment => {}
                    }
                }
            }
            Message::ServiceTab(tab) => {
                self.service_tab = tab;
                if tab == ServiceTab::Logs {
                    if let Some(sid) = self.active_service_id.clone() {
                        self.send(Ctx::Logs, Command::LogsGet { service_id: sid, tail: 500 });
                    }
                    self.rebuild_log_editor();
                } else if tab == ServiceTab::Deployments {
                    self.rebuild_build_log_editor();
                }
            }

            // ── New project ───────────────────────────────────────────────
            Message::NewProjectOpen => {
                self.new_project_open = true;
                self.np_name.clear();
                self.np_desc.clear();
            }
            Message::NpName(v) => self.np_name = v,
            Message::NpDesc(v) => self.np_desc = v,
            Message::NpCancel => self.new_project_open = false,
            Message::NpSubmit => {
                if !self.np_name.trim().is_empty() {
                    self.send(
                        Ctx::CreateProject,
                        Command::ProjectCreate {
                            name: self.np_name.trim().to_string(),
                            description: opt(&self.np_desc),
                        },
                    );
                }
            }

            // ── Project settings / delete ─────────────────────────────────
            Message::PsName(v) => self.ps_name = v,
            Message::PsDesc(v) => self.ps_desc = v,
            Message::PsSave => {
                if let Some(id) = self.active_project_id.clone() {
                    self.send(
                        Ctx::UpdateProject,
                        Command::ProjectUpdate {
                            id,
                            name: self.ps_name.trim().to_string(),
                            description: opt(&self.ps_desc),
                        },
                    );
                }
            }
            Message::AskDelete(action) => self.confirm = Some(action),
            Message::ConfirmNo => self.confirm = None,
            Message::ConfirmYes => {
                if let Some(action) = self.confirm.take() {
                    match action {
                        ConfirmAction::DeleteProject(id) => {
                            self.send(Ctx::DeleteProject(id.clone()), Command::ProjectDelete { id });
                        }
                        ConfirmAction::DeleteService(id) => {
                            self.send(Ctx::DeleteService(id.clone()), Command::ServiceDelete { id });
                        }
                    }
                }
            }

            // ── Project env ───────────────────────────────────────────────
            Message::PEnvOpen => {
                self.p_env_editor = KvEditor { open: true, ..Default::default() };
            }
            Message::PEnvKey(v) => self.p_env_editor.key = v,
            Message::PEnvVal(v) => self.p_env_editor.value = v,
            Message::PEnvCancel => self.p_env_editor.open = false,
            Message::PEnvSubmit => self.project_env_add(),
            Message::PEnvDelete(i) => self.project_env_delete(i),

            // ── Secrets ───────────────────────────────────────────────────
            Message::SecretOpen => {
                self.secret_editor = KvEditor { open: true, ..Default::default() };
            }
            Message::SecretName(v) => self.secret_editor.key = v,
            Message::SecretVal(v) => self.secret_editor.value = v,
            Message::SecretCancel => self.secret_editor.open = false,
            Message::SecretSubmit => {
                if let Some(pid) = self.active_project_id.clone() {
                    if !self.secret_editor.key.trim().is_empty() {
                        self.send(
                            Ctx::Action("Secret salvo".into()),
                            Command::SecretSet {
                                project_id: pid,
                                name: self.secret_editor.key.trim().to_string(),
                                value: self.secret_editor.value.clone(),
                            },
                        );
                        self.secret_editor.open = false;
                    }
                }
            }
            Message::SecretDelete(name) => {
                if let Some(pid) = self.active_project_id.clone() {
                    self.send(
                        Ctx::Action("Secret removido".into()),
                        Command::SecretDelete { project_id: pid, name },
                    );
                }
            }

            // ── Service actions ───────────────────────────────────────────
            Message::SvcDeploy => self.service_action(Ctx::Deploy, |id| Command::DeployStart { service_id: id }, "Deploy"),
            Message::SvcReload => self.service_action(Ctx::Action("Reload".into()), |id| Command::ServiceReload { service_id: id }, "Reload"),
            Message::SvcStop => self.service_action(Ctx::Action("Stop".into()), |id| Command::ServiceStop { service_id: id }, "Stop"),
            Message::SvcRollback => self.service_action(Ctx::Action("Rollback".into()), |id| Command::DeployRollback { service_id: id }, "Rollback"),

            // ── General / git form ────────────────────────────────────────
            Message::GenField(f, v) => {
                let g = &mut self.general;
                match f {
                    GenField::RepoUrl => g.repo_url = v,
                    GenField::Branch => g.branch = v,
                    GenField::Username => g.username = v,
                    GenField::Credentials => g.credentials = v,
                    GenField::BuildPath => g.build_path = v,
                    GenField::WatchPaths => g.watch_paths = v,
                    GenField::Port => g.port = v,
                    GenField::Dockerfile => g.dockerfile = v,
                    GenField::ContextPath => g.context_path = v,
                    GenField::BuildStage => g.build_stage = v,
                }
            }
            Message::GenSubmodules(b) => self.general.submodules = b,
            Message::GenSave => self.general_save(),
            Message::ComposeAction(action) => self.compose_editor.perform(action),
            Message::ComposeSave => self.compose_save(),
            // Logs são read-only: aplicamos apenas ações de seleção/cursor/scroll
            // (cópia via Ctrl+C é tratada internamente pelo widget); ignoramos
            // edições para que o texto continue espelhando o buffer.
            Message::BuildLogModal(open) => self.build_log_modal_open = open,
            Message::BuildLogAction(action) => {
                if !action.is_edit() {
                    self.build_log_editor.perform(action);
                }
            }
            Message::LogAction(action) => {
                if !action.is_edit() {
                    self.log_editor.perform(action);
                }
            }

            // ── Service env ───────────────────────────────────────────────
            Message::SEnvOpen => {
                self.s_env_editor = KvEditor { open: true, ..Default::default() };
                self.s_env_text_open = false;
            }
            Message::SEnvKey(v) => self.s_env_editor.key = v,
            Message::SEnvVal(v) => self.s_env_editor.value = v,
            Message::SEnvCancel => self.s_env_editor.open = false,
            Message::SEnvSubmit => self.service_env_add(),
            Message::SEnvDelete(i) => self.service_env_delete(i),
            Message::SEnvTextOpen => {
                self.s_env_text_open = !self.s_env_text_open;
                self.s_env_editor.open = false;
                if self.s_env_text_open {
                    let text = env_vars_to_dotenv(
                        self.current_service().map(|s| s.spec.env_vars.as_slice()).unwrap_or(&[])
                    );
                    self.s_env_text_editor = iced::widget::text_editor::Content::with_text(&text);
                }
            }
            Message::SEnvTextAction(action) => self.s_env_text_editor.perform(action),
            Message::SEnvImport => {
                let text = self.s_env_text_editor.text();
                let raw = parse_dotenv(&text);
                // Keep only the last occurrence of each key (last-wins, same as resolve_env)
                let mut last_idx: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
                for (i, ev) in raw.iter().enumerate() {
                    last_idx.insert(ev.key.clone(), i);
                }
                let parsed: Vec<shared::EnvVar> = raw.into_iter().enumerate()
                    .filter(|(i, ev)| last_idx.get(&ev.key) == Some(i))
                    .map(|(_, ev)| ev)
                    .collect();
                self.s_env_text_open = false;
                self.update_spec(move |s| s.env_vars = parsed);
            }
            Message::SEnvExport => {
                let text = env_vars_to_dotenv(
                    self.current_service().map(|s| s.spec.env_vars.as_slice()).unwrap_or(&[])
                );
                return iced::clipboard::write(text);
            }

            // ── Domains ───────────────────────────────────────────────────
            Message::DomDomain(v) => self.domains.domain = v,
            Message::DomHostPort(v) => self.domains.host_port = v,
            Message::DomTls(b) => self.domains.tls_enabled = b,
            Message::DomSave => self.domains_save(),

            // ── Healthcheck ───────────────────────────────────────────────
            Message::HcKind(v) => self.health.kind = v,
            Message::HcField(f, v) => {
                let h = &mut self.health;
                match f {
                    HcField::HttpPath => h.http_path = v,
                    HcField::ExpectedStatus => h.expected_status = v,
                    HcField::Interval => h.interval = v,
                    HcField::Timeout => h.timeout = v,
                    HcField::Retries => h.retries = v,
                    HcField::StartPeriod => h.start_period = v,
                }
            }
            Message::HcSave => {
                let hc = self.health.to_healthcheck();
                self.update_spec(|s| s.healthcheck = hc);
            }

            // ── Advanced ──────────────────────────────────────────────────
            Message::AdvReplicas(v) => self.advanced.replicas = v,
            Message::AdvCommand(v) => self.advanced.run_command = v,
            Message::AdvArgAdd => self.advanced.run_args.push(String::new()),
            Message::AdvArg(i, v) => {
                if let Some(a) = self.advanced.run_args.get_mut(i) {
                    *a = v;
                }
            }
            Message::AdvArgDelete(i) => {
                if i < self.advanced.run_args.len() {
                    self.advanced.run_args.remove(i);
                }
            }
            Message::AdvSave => {
                let replicas = self.advanced.replicas.parse().unwrap_or(1);
                let cmd = opt(&self.advanced.run_command);
                let args: Vec<String> = self
                    .advanced
                    .run_args
                    .iter()
                    .filter(|a| !a.trim().is_empty())
                    .cloned()
                    .collect();
                self.update_spec(|s| {
                    s.replicas = replicas;
                    s.run_command = cmd;
                    s.run_args = args;
                });
            }

            // ── Deployments ───────────────────────────────────────────────
            Message::DeploySelect(i) => {
                self.selected_deployment = i;
                if let Some(dep) = self.service_deployments.get(i) {
                    self.send(Ctx::BuildLogs, Command::GetBuildLogs { deployment_id: dep.id.clone() });
                }
                self.rebuild_build_log_editor();
            }
            Message::WebhookRegen => {
                if let Some(sid) = self.active_service_id.clone() {
                    self.send(Ctx::WebhookUrl, Command::RegenerateWebhookToken { service_id: sid });
                }
            }

            // ── New service wizard ────────────────────────────────────────
            Message::NewServiceOpen => {
                if let Some(pid) = self.active_project_id.clone() {
                    self.ns = Some(NsForm::new(pid));
                }
            }
            Message::NsCancel => self.ns = None,
            Message::NsBack => self.ns_back(),
            Message::NsPickType(kind) => {
                if let Some(ns) = &mut self.ns {
                    ns.step = match kind {
                        ServiceKind::Application => NsStep::AppForm,
                        ServiceKind::Database => NsStep::PickDb,
                        ServiceKind::Compose => NsStep::ComposeForm,
                        ServiceKind::Template => NsStep::PickTemplate,
                    };
                }
            }
            Message::NsPickDb(db) => {
                if let Some(ns) = &mut self.ns {
                    ns.select_db(db);
                }
            }
            Message::NsField(f, v) => {
                if let Some(ns) = &mut self.ns {
                    match f {
                        NsField::Name => ns.name = v,
                        NsField::AppName => ns.app_name = v,
                        NsField::Description => ns.description = v,
                        NsField::DbName => ns.db_name = v,
                        NsField::DbUser => ns.db_user = v,
                        NsField::DbPassword => ns.db_password = v,
                        NsField::DbRootPassword => ns.db_root_password = v,
                        NsField::Image => ns.docker_image = v,
                    }
                }
            }
            Message::NsReplica(b) => {
                if let Some(ns) = &mut self.ns {
                    ns.use_replica_sets = b;
                }
            }
            Message::NsTemplateCat(i) => {
                if let Some(ns) = &mut self.ns {
                    ns.template_cat = i;
                }
            }
            Message::NsTemplateSearch(v) => {
                if let Some(ns) = &mut self.ns {
                    ns.template_search = v;
                }
            }
            Message::NsTemplateSelect(id) => {
                if let Some(t) = shared::templates::all().iter().find(|t| t.id == id) {
                    if let Some(ns) = &mut self.ns {
                        ns.select_template(t);
                    }
                }
            }
            Message::NsTemplateVar(i, v) => {
                if let Some(ns) = &mut self.ns {
                    if let Some(slot) = ns.template_var_values.get_mut(i) {
                        *slot = v;
                    }
                }
            }
            Message::NsCreate => {
                if let Some(spec) = self.ns.as_ref().and_then(|ns| ns.to_spec()) {
                    self.send(Ctx::CreateService, Command::ServiceCreate(spec));
                }
            }

            // ── Server settings ───────────────────────────────────────────
            Message::SsDomain(v) => self.ss_domain = v,
            Message::SsEmail(v) => self.ss_email = v,
            Message::SsSave => {
                self.send(
                    Ctx::Action("Configurações salvas".into()),
                    Command::SetDaemonSettings {
                        webhook_base_url: opt(&self.ss_domain),
                        acme_email: opt(&self.ss_email),
                    },
                );
            }
            // ── Git providers (Settings → Git) ────────────────────────────
            Message::GpName(v) => self.gp_form.name = v,
            Message::GpBaseUrl(v) => self.gp_form.base_url = v,
            Message::GpMode(m) => self.gp_form.mode = m,
            Message::GpClientId(v) => self.gp_form.client_id = v,
            Message::GpClientSecret(v) => self.gp_form.client_secret = v,
            Message::GpPat(v) => self.gp_form.pat = v,
            Message::GpRefresh => self.send(Ctx::GitProviders, Command::GitProviderList),
            Message::GpDelete(id) => {
                self.send(Ctx::GitProviderDeleted, Command::GitProviderDelete { id });
            }
            Message::GpConnect => self.gp_connect(),

            // ── Provider sub-tab (General) ────────────────────────────────
            Message::ProviderTabChanged(tab) => {
                self.provider_tab = tab;
                // Ao abrir a sub-aba Gitea com uma conta já escolhida, garante
                // que os repositórios estejam carregados.
                if tab == ProviderTab::Gitea && self.git_repos.is_empty() {
                    if let Some(pid) = self.gitea.provider_id.clone() {
                        self.send(Ctx::GitRepos, Command::GitRepoList { provider_id: pid });
                    }
                }
            }

            // ── Service Gitea form ────────────────────────────────────────
            Message::GiteaProviderPick(choice) => {
                self.gitea.provider_id = Some(choice.id.clone());
                self.gitea.repo_full_name = None;
                self.gitea.branch = None;
                self.git_repos.clear();
                self.git_branches.clear();
                self.send(Ctx::GitRepos, Command::GitRepoList { provider_id: choice.id });
            }
            Message::GiteaRepoPick(repo) => {
                self.gitea.repo_full_name = Some(repo.full_name.clone());
                self.gitea.clone_url = repo.clone_url.clone();
                self.gitea.branch = Some(repo.default_branch.clone());
                self.git_branches.clear();
                if let Some(pid) = self.gitea.provider_id.clone() {
                    self.send(
                        Ctx::GitBranches,
                        Command::GitBranchList { provider_id: pid, repo_full_name: repo.full_name },
                    );
                }
            }
            Message::GiteaBranchPick(b) => self.gitea.branch = Some(b),
            Message::GiteaBuildPath(v) => self.gitea.build_path = v,
            Message::GiteaDockerfile(v) => self.gitea.dockerfile = v,
            Message::GiteaSubmodules(b) => self.gitea.submodules = b,
            Message::GiteaPort(v) => self.gitea.port = v,
            Message::GiteaWatchAdd => self.gitea.watch_paths.push(String::new()),
            Message::GiteaWatch(i, v) => {
                if let Some(p) = self.gitea.watch_paths.get_mut(i) {
                    *p = v;
                }
            }
            Message::GiteaWatchDelete(i) => {
                if i < self.gitea.watch_paths.len() {
                    self.gitea.watch_paths.remove(i);
                }
            }
            Message::GiteaSave => self.gitea_save(),

            Message::Copy(s) => return iced::clipboard::write(s),
            Message::Ignore => {}
        }
        Task::none()
    }

    // ── Navigation helpers ────────────────────────────────────────────────

    fn select_sidebar(&mut self, item: SidebarItem) {
        self.sidebar = item;
        self.view = item.to_view();
        match item {
            SidebarItem::Projects => self.send(Ctx::Projects, Command::ProjectList),
            SidebarItem::HomeDeployments => {
                self.send(Ctx::HomeDeployments, Command::RecentDeployments { limit: 30 })
            }
            SidebarItem::HomeDeployEngine => {
                self.engine_ticks = 0;
                self.send(Ctx::DeployEngine, Command::DeployEngineStatus)
            }
            SidebarItem::SettingsWebServer => {
                if !self.ss_loaded {
                    self.send(Ctx::ServerSettings, Command::GetDaemonSettings);
                }
            }
            SidebarItem::SettingsGit => {
                self.send(Ctx::GitProviders, Command::GitProviderList);
                // Precisa do domínio do servidor para montar a redirect URL do OAuth.
                if !self.ss_loaded {
                    self.send(Ctx::ServerSettings, Command::GetDaemonSettings);
                }
            }
            _ => {}
        }
    }

    fn open_project(&mut self, id: String) {
        if let Some(p) = self.projects.iter().find(|p| p.id == id).cloned() {
            self.active_project_id = Some(p.id.clone());
            self.services.clear();
            self.project_secrets.clear();
            self.ps_name = p.name.clone();
            self.ps_desc = p.description.clone().unwrap_or_default();
            self.view = View::ProjectDetail;
            self.project_tab = ProjectTab::Services;
            self.p_env_editor.open = false;
            self.secret_editor.open = false;
            self.send(Ctx::Services, Command::ServiceList { project_id: p.id });
        }
    }

    fn open_service(&mut self, id: String) {
        let Some(svc) = self.services.iter().find(|s| s.id == id).cloned() else {
            return;
        };
        self.active_service_id = Some(svc.id.clone());
        self.view = View::ServiceDetail;
        self.service_tab = ServiceTab::General;
        self.conn_info = ConnInfo::from_service(&svc);
        self.general = GeneralForm::from_service(&svc);
        self.gitea = GiteaForm::from_service(&svc);
        self.health = HealthForm::from_service(&svc);
        self.domains = DomainsForm::from_service(&svc);
        self.advanced = AdvancedForm::from_service(&svc);
        self.git_repos.clear();
        self.git_branches.clear();
        // Abre na sub-aba Gitea quando o serviço já está vinculado a um provider
        // (e pré-carrega seus repositórios); caso contrário, na sub-aba Git.
        if let Some(pid) = self.gitea.provider_id.clone() {
            self.provider_tab = ProviderTab::Gitea;
            self.send(Ctx::GitRepos, Command::GitRepoList { provider_id: pid });
        } else {
            self.provider_tab = ProviderTab::Git;
        }
        let compose = match &svc.spec.source {
            ServiceSource::Compose(c) => c.content.clone(),
            _ => String::new(),
        };
        self.compose_editor = iced::widget::text_editor::Content::with_text(&compose);
        self.s_env_editor.open = false;
        self.selected_deployment = 0;
        self.service_deployments.clear();
        self.build_log_editor = iced::widget::text_editor::Content::new();
        self.log_editor = iced::widget::text_editor::Content::new();
        self.webhook_url = None;
        self.send(Ctx::Deployments, Command::DeployHistory { service_id: svc.id.clone(), limit: 10 });
        self.send(Ctx::Logs, Command::LogsGet { service_id: svc.id.clone(), tail: 500 });
        if !matches!(svc.spec.source, ServiceSource::Compose(_)) {
            self.send(Ctx::WebhookUrl, Command::GetWebhookUrl { service_id: svc.id });
        }
    }

    /// Rebuild the read-only build-log editor from the currently selected
    /// deployment's buffer, so the rendered text matches the live logs.
    fn rebuild_build_log_editor(&mut self) {
        let text = self
            .service_deployments
            .get(self.selected_deployment)
            .and_then(|dep| self.build_logs.get(&dep.id))
            .map(|buf| {
                buf.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n")
            })
            .unwrap_or_default();
        self.build_log_editor = iced::widget::text_editor::Content::with_text(&text);
    }

    /// Rebuild the read-only service-log editor from the active service buffer.
    fn rebuild_log_editor(&mut self) {
        let text = self
            .active_service_id
            .as_ref()
            .and_then(|sid| self.logs.get(sid))
            .map(|buf| {
                buf.iter()
                    .map(|l| format!("{} {}", l.timestamp.format("%H:%M:%S%.3f"), l.text))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        self.log_editor = iced::widget::text_editor::Content::with_text(&text);
    }

    // ── Worker / responses / events ───────────────────────────────────────

    fn handle_worker(&mut self, ev: WorkerEvent) {
        match ev {
            WorkerEvent::Ready(tx) => self.worker_tx = Some(tx),
            WorkerEvent::Connected => {
                self.connected = true;
                self.error = None;
                // Only persist the connection prefs after a successful connect.
                self.persist_prefs();
                self.status_msg = format!("Conectado a {}", self.url);
                self.send(Ctx::DaemonStatus, Command::DaemonStatus);
                self.send(Ctx::Projects, Command::ProjectList);
                self.send(Ctx::HomeDeployments, Command::RecentDeployments { limit: 30 });
                self.send(Ctx::GitProviders, Command::GitProviderList);
                if let Some(pid) = self.active_project_id.clone() {
                    self.send(Ctx::Services, Command::ServiceList { project_id: pid.clone() });
                    self.send(Ctx::Secrets, Command::SecretList { project_id: pid });
                }
            }
            WorkerEvent::Reply(ctx, resp) => self.handle_reply(ctx, resp),
            WorkerEvent::Event(e) => self.apply_event(e),
            WorkerEvent::Error(e) => {
                self.connected = false;
                self.error = Some(e.clone());
                self.status_msg = "Erro de conexão".into();
            }
            WorkerEvent::Disconnected => {
                self.connected = false;
                self.status_msg = "Conexão encerrada".into();
            }
        }
    }

    fn handle_reply(&mut self, ctx: Ctx, resp: Response) {
        match (ctx, resp) {
            (Ctx::Projects, Response::Projects(p)) => self.projects = p,
            (Ctx::Services, Response::Services(s)) => self.services = s,
            (Ctx::HomeDeployments, Response::DeploymentSummaries(s)) => self.home_deployments = s,
            (Ctx::DeployEngine, Response::DeployEngineStatus(s)) => self.deploy_engine = Some(s),
            (Ctx::Deployments, Response::Deployments(d)) => {
                if let Some(first) = d.first() {
                    self.send(Ctx::BuildLogs, Command::GetBuildLogs { deployment_id: first.id.clone() });
                }
                self.service_deployments = d;
                self.selected_deployment = 0;
            }
            (Ctx::BuildLogs, Response::BuildLogs(entries)) => {
                if let Some(dep) = self.service_deployments.get(self.selected_deployment) {
                    let buf = self.build_logs.entry(dep.id.clone()).or_default();
                    buf.clear();
                    for e in entries {
                        buf.push(LogLine { timestamp: e.timestamp, text: e.line, is_stderr: false });
                    }
                }
                self.rebuild_build_log_editor();
            }
            (Ctx::Logs, Response::Logs(entries)) => {
                // O daemon recarimba todas as linhas com `now()` a cada poll, então
                // não dá pra confiar nos timestamps para detectar novidade. Fazemos
                // um merge incremental por conteúdo: preservamos as linhas já vistas
                // (e seus timestamps) e anexamos só as realmente novas. Quando nada
                // muda, não reconstruímos o editor — preservando a seleção do usuário.
                if !entries.is_empty() {
                    if let Some(sid) = self.active_service_id.clone() {
                        let incoming: Vec<LogLine> = entries
                            .into_iter()
                            .map(|e| LogLine {
                                timestamp: e.timestamp,
                                text: e.line,
                                is_stderr: e.stream == shared::protocol::LogStream::Stderr,
                            })
                            .collect();
                        let buf = self.logs.entry(sid).or_default();
                        if merge_logs(buf, incoming) {
                            self.rebuild_log_editor();
                        }
                    }
                }
            }
            (Ctx::Secrets, Response::SecretNames(n)) => self.project_secrets = n,
            (Ctx::WebhookUrl, Response::WebhookUrl(u)) => self.webhook_url = u,
            (Ctx::ServerSettings, Response::DaemonSettings { webhook_base_url, acme_email }) => {
                self.ss_domain = webhook_base_url.unwrap_or_default();
                self.ss_email = acme_email.unwrap_or_default();
                self.ss_loaded = true;
            }
            (Ctx::DaemonStatus, Response::DaemonStatus(d)) => self.daemon_status = Some(d),
            (Ctx::CreateProject, Response::Project(p)) => {
                self.projects.push(p);
                self.new_project_open = false;
                self.notify("Projeto criado", false);
            }
            (Ctx::UpdateProject, Response::Project(p)) => {
                if let Some(e) = self.projects.iter_mut().find(|x| x.id == p.id) {
                    *e = p.clone();
                }
                self.ps_name = p.name;
                self.ps_desc = p.description.unwrap_or_default();
                self.notify("Projeto atualizado", false);
            }
            (Ctx::UpdateProjectEnv, Response::Project(p)) => {
                if let Some(e) = self.projects.iter_mut().find(|x| x.id == p.id) {
                    *e = p;
                }
                self.p_env_editor.open = false;
                self.notify("Env vars atualizadas", false);
            }
            (Ctx::DeleteProject(id), Response::Ok) => {
                self.projects.retain(|p| p.id != id);
                self.view = View::Projects;
                self.notify("Projeto removido", false);
            }
            (Ctx::CreateService, Response::Service(s)) => {
                self.services.push(s);
                self.ns = None;
                self.view = View::ProjectDetail;
                self.notify("Serviço criado", false);
            }
            (Ctx::UpdateService, Response::Service(s)) => {
                if let Some(e) = self.services.iter_mut().find(|x| x.id == s.id) {
                    *e = s.clone();
                }
                if self.active_service_id.as_deref() == Some(&s.id) {
                    self.general = GeneralForm::from_service(&s);
                    self.gitea = GiteaForm::from_service(&s);
                    self.health = HealthForm::from_service(&s);
                    self.domains = DomainsForm::from_service(&s);
                    self.advanced = AdvancedForm::from_service(&s);
                    self.s_env_editor.open = false;
                }
                self.notify("Serviço atualizado", false);
            }
            (Ctx::DeleteService(id), Response::Ok) => {
                self.services.retain(|s| s.id != id);
                self.view = View::ProjectDetail;
                self.notify("Serviço removido", false);
            }
            (Ctx::Deploy, Response::Deployment(dep)) => {
                if let Some(pos) = self.service_deployments.iter().position(|d| d.id == dep.id) {
                    self.service_deployments[pos] = dep;
                } else {
                    self.service_deployments.insert(0, dep);
                }
                self.notify("Deploy iniciado ✓", false);
            }
            // ── Git providers ─────────────────────────────────────────────
            (Ctx::GitProviders, Response::GitProviders(ps)) => self.git_providers = ps,
            (Ctx::CreateGitProvider, Response::GitProviderInfo(p)) => {
                let is_oauth = matches!(p.auth_mode, shared::GitAuthMode::OAuth);
                let pid = p.id.clone();
                if let Some(e) = self.git_providers.iter_mut().find(|x| x.id == p.id) {
                    *e = p;
                } else {
                    self.git_providers.push(p);
                }
                self.gp_form = GpForm::default();
                if is_oauth {
                    self.notify("Abrindo navegador para autorizar…", false);
                    self.send(Ctx::OAuthStart, Command::GitOAuthStart { provider_id: pid });
                } else {
                    self.notify("Conta Gitea conectada ✓", false);
                }
            }
            (Ctx::OAuthStart, Response::OAuthUrl(url)) => {
                if let Err(e) = open::that(&url) {
                    self.notify(format!("Abra manualmente: {url} ({e})"), true);
                } else {
                    self.notify("Autorize no navegador e clique em Atualizar", false);
                }
            }
            (Ctx::GitProviderDeleted, Response::Ok) => {
                self.notify("Provider removido", false);
                self.send(Ctx::GitProviders, Command::GitProviderList);
            }
            (Ctx::GitRepos, Response::GitRepos(r)) => self.git_repos = r,
            (Ctx::GitBranches, Response::GitBranches(b)) => self.git_branches = b,

            (Ctx::Action(label), Response::Err { message, .. }) => {
                self.notify(format!("{label}: {message}"), true);
            }
            (Ctx::Action(label), _) => {
                self.notify(label, false);
                // refresh services and, if relevant, secrets
                if let Some(pid) = self.active_project_id.clone() {
                    self.send(Ctx::Services, Command::ServiceList { project_id: pid.clone() });
                    self.send(Ctx::Secrets, Command::SecretList { project_id: pid });
                }
            }
            (_, Response::Err { message, .. }) => self.notify(message, true),
            _ => {}
        }
    }

    fn apply_event(&mut self, event: Event) {
        match event {
            Event::ServiceStatusChanged { service_id, status } => {
                if let Some(svc) = self.services.iter_mut().find(|s| s.id == service_id) {
                    svc.status = status.clone();
                }
                if matches!(status, ServiceStatus::Running)
                    && self.active_service_id.as_deref() == Some(&service_id)
                {
                    self.logs.remove(&service_id);
                    self.send(Ctx::Logs, Command::LogsGet { service_id, tail: 500 });
                }
            }
            Event::DeployStateChanged { deployment_id, service_id, state, message, .. } => {
                if matches!(state, shared::DeployState::RollingBack) {
                    let reason = message.as_deref().unwrap_or("motivo desconhecido");
                    self.notify(format!("Deploy falhou: {reason}"), true);
                }
                if let Some(s) = self.home_deployments.iter_mut().find(|s| s.deployment.id == deployment_id) {
                    s.deployment.state = state.clone();
                }
                if let Some(dep) = self.service_deployments.iter_mut().find(|d| d.id == deployment_id) {
                    dep.state = state.clone();
                } else if self.active_service_id.as_deref() == Some(&service_id) {
                    self.service_deployments.insert(0, shared::Deployment {
                        id: deployment_id,
                        service_id,
                        image: String::new(),
                        state,
                        states_log: vec![],
                        started_at: chrono::Utc::now(),
                        finished_at: None,
                    });
                }
            }
            Event::DeployProgress { .. } => {}
            Event::BuildLog { deployment_id, line, timestamp, .. } => {
                let displayed = self
                    .service_deployments
                    .get(self.selected_deployment)
                    .is_some_and(|d| d.id == deployment_id);
                let buf = self.build_logs.entry(deployment_id).or_default();
                if buf.len() >= MAX_LOG_LINES {
                    buf.remove(0);
                }
                buf.push(LogLine { timestamp, text: line, is_stderr: false });
                if displayed {
                    self.rebuild_build_log_editor();
                }
            }
            Event::LogLine { service_id, stream, line, timestamp, .. } => {
                let displayed = self.active_service_id.as_deref() == Some(service_id.as_str());
                let buf = self.logs.entry(service_id).or_default();
                if buf.len() >= MAX_LOG_LINES {
                    buf.remove(0);
                }
                buf.push(LogLine {
                    timestamp,
                    text: line,
                    is_stderr: stream == shared::protocol::LogStream::Stderr,
                });
                if displayed {
                    self.rebuild_log_editor();
                }
            }
            Event::ContainerMetrics(m) => {
                let buf = self.metrics.entry(m.service_id.clone()).or_default();
                if buf.len() >= MAX_METRIC_POINTS {
                    buf.remove(0);
                }
                buf.push(m);
            }
            Event::Error { message, .. } => self.notify(message, true),
            Event::DaemonReady { version } => self.notify(format!("daemon {version} ready"), false),
        }
    }

    // ── Periodic ──────────────────────────────────────────────────────────

    fn on_tick(&mut self) {
        if let Some(n) = &self.notification {
            if n.expires_at <= std::time::Instant::now() {
                self.notification = None;
            }
        }
        if !self.connected {
            return;
        }
        if self.view == View::ServiceDetail && self.service_tab == ServiceTab::Logs {
            self.log_ticks += 1;
            if self.log_ticks >= 5 {
                self.log_ticks = 0;
                if let Some(sid) = self.active_service_id.clone() {
                    self.send(Ctx::Logs, Command::LogsGet { service_id: sid, tail: 500 });
                }
            }
        } else {
            self.log_ticks = 0;
        }
        if self.view == View::HomeDeployEngine {
            self.engine_ticks += 1;
            if self.engine_ticks >= 5 {
                self.engine_ticks = 0;
                self.send(Ctx::DeployEngine, Command::DeployEngineStatus);
            }
        } else {
            self.engine_ticks = 0;
        }
    }

    // ── Mutations ─────────────────────────────────────────────────────────

    fn reset_data(&mut self) {
        self.daemon_status = None;
        self.projects.clear();
        self.services.clear();
        self.active_project_id = None;
        self.active_service_id = None;
        self.home_deployments.clear();
        self.deploy_engine = None;
        self.service_deployments.clear();
        self.conn_info = None;
        self.project_secrets.clear();
        self.logs.clear();
        self.build_logs.clear();
        self.metrics.clear();
        self.ns = None;
        self.confirm = None;
        self.view = View::HomeDeployments;
        self.sidebar = SidebarItem::HomeDeployments;
    }

    fn service_action(&mut self, ctx: Ctx, build: impl Fn(String) -> Command, label: &str) {
        if let Some(id) = self.active_service_id.clone() {
            self.notify(format!("{label}…"), false);
            self.send(ctx, build(id));
        }
    }

    /// Clones the active service spec, applies `mutate`, sends `ServiceUpdate`.
    fn update_spec(&mut self, mutate: impl FnOnce(&mut ServiceSpec)) {
        let Some(svc) = self.current_service().cloned() else {
            return;
        };
        let mut spec = svc.spec.clone();
        mutate(&mut spec);
        self.send(Ctx::UpdateService, Command::ServiceUpdate { id: svc.id, spec });
    }

    fn general_save(&mut self) {
        let Some(svc) = self.current_service().cloned() else { return };
        let g = self.general.clone();
        let mut spec = svc.spec.clone();
        spec.port = g.port.parse().unwrap_or(spec.port);
        spec.source = match &svc.spec.source {
            ServiceSource::Git(_) => ServiceSource::Git(g.to_git_source()),
            // A registry-typed Application becomes a Git source when the URL
            // looks like a repository (https/ssh/git@/file:///…).
            ServiceSource::Registry { .. } => {
                if shared::looks_like_git_url(&g.repo_url) {
                    ServiceSource::Git(g.to_git_source())
                } else {
                    ServiceSource::Registry { image: g.repo_url.clone() }
                }
            }
            other => other.clone(),
        };
        self.send(Ctx::UpdateService, Command::ServiceUpdate { id: svc.id, spec });
    }

    fn gp_connect(&mut self) {
        let f = &self.gp_form;
        if f.base_url.trim().is_empty() {
            self.notify("Informe a Base URL do Gitea", true);
            return;
        }
        let name = if f.name.trim().is_empty() {
            "Gitea".to_string()
        } else {
            f.name.trim().to_string()
        };
        let cmd = match f.mode {
            shared::GitAuthMode::OAuth => {
                if f.client_id.trim().is_empty() || f.client_secret.trim().is_empty() {
                    self.notify("Client ID e Client Secret são obrigatórios", true);
                    return;
                }
                Command::GitProviderCreate {
                    kind: shared::GitProviderKind::Gitea,
                    name,
                    base_url: f.base_url.trim().to_string(),
                    auth_mode: shared::GitAuthMode::OAuth,
                    oauth_client_id: Some(f.client_id.trim().to_string()),
                    oauth_client_secret: Some(f.client_secret.clone()),
                    pat: None,
                }
            }
            shared::GitAuthMode::Pat => {
                if f.pat.trim().is_empty() {
                    self.notify("Informe o Personal Access Token", true);
                    return;
                }
                Command::GitProviderCreate {
                    kind: shared::GitProviderKind::Gitea,
                    name,
                    base_url: f.base_url.trim().to_string(),
                    auth_mode: shared::GitAuthMode::Pat,
                    oauth_client_id: None,
                    oauth_client_secret: None,
                    pat: Some(f.pat.clone()),
                }
            }
        };
        self.send(Ctx::CreateGitProvider, cmd);
    }

    fn gitea_save(&mut self) {
        let Some(svc) = self.current_service().cloned() else { return };
        if self.gitea.provider_id.is_none() {
            self.notify("Selecione uma conta Gitea", true);
            return;
        }
        if self.gitea.clone_url.trim().is_empty() {
            self.notify("Selecione um repositório", true);
            return;
        }
        let mut spec = svc.spec.clone();
        spec.port = self.gitea.port.parse().unwrap_or(spec.port);
        spec.source = ServiceSource::Git(self.gitea.to_git_source());
        self.send(Ctx::UpdateService, Command::ServiceUpdate { id: svc.id, spec });
    }

    fn compose_save(&mut self) {
        let content = self.compose_editor.text();
        self.update_spec(|s| s.source = ServiceSource::Compose(shared::ComposeSource { content }));
    }

    fn domains_save(&mut self) {
        let domain = opt(&self.domains.domain);
        let host_port = self.domains.host_port.parse::<u16>().ok();
        let tls = self.domains.tls_enabled;
        self.update_spec(move |s| {
            s.domain = domain;
            s.host_port = host_port;
            s.tls_enabled = tls;
        });
    }

    fn service_env_add(&mut self) {
        let key = self.s_env_editor.key.trim().to_string();
        if key.is_empty() {
            return;
        }
        let value = shared::EnvVarValue::Plain(self.s_env_editor.value.clone());
        self.s_env_editor.open = false;
        self.update_spec(move |s| {
            if let Some(ev) = s.env_vars.iter_mut().find(|e| e.key == key) {
                ev.value = value;
            } else {
                s.env_vars.push(shared::EnvVar { key, value });
            }
        });
    }

    fn service_env_delete(&mut self, i: usize) {
        self.update_spec(move |s| {
            if i < s.env_vars.len() {
                s.env_vars.remove(i);
            }
        });
    }

    fn project_env_add(&mut self) {
        let key = self.p_env_editor.key.trim().to_string();
        if key.is_empty() {
            return;
        }
        let Some(project) = self.current_project().cloned() else { return };
        let mut env = project.env_vars.clone();
        let value = shared::EnvVarValue::Plain(self.p_env_editor.value.clone());
        if let Some(ev) = env.iter_mut().find(|e| e.key == key) {
            ev.value = value;
        } else {
            env.push(shared::EnvVar { key, value });
        }
        self.send(Ctx::UpdateProjectEnv, Command::ProjectEnvSet { project_id: project.id, env_vars: env });
    }

    fn project_env_delete(&mut self, i: usize) {
        let Some(project) = self.current_project().cloned() else { return };
        let mut env = project.env_vars.clone();
        if i < env.len() {
            env.remove(i);
        }
        self.send(Ctx::UpdateProjectEnv, Command::ProjectEnvSet { project_id: project.id, env_vars: env });
    }

    fn ns_back(&mut self) {
        if let Some(ns) = &mut self.ns {
            ns.step = match ns.step {
                NsStep::PickType => {
                    self.ns = None;
                    return;
                }
                NsStep::PickDb | NsStep::AppForm | NsStep::ComposeForm | NsStep::PickTemplate => {
                    NsStep::PickType
                }
                NsStep::DbForm => NsStep::PickDb,
                NsStep::TemplateForm => NsStep::PickTemplate,
            };
        }
    }
}

/// Convenience used by the views to colour a service status.
/// Converte uma lista de env vars para o formato de arquivo `.env`.
pub fn env_vars_to_dotenv(env_vars: &[shared::EnvVar]) -> String {
    env_vars
        .iter()
        .map(|ev| {
            let val = match &ev.value {
                shared::EnvVarValue::Plain(v) => v.clone(),
                shared::EnvVarValue::Secret(s) => format!("<secret:{s}>"),
            };
            format!("{}={}", ev.key, val)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Faz parse de texto no formato `.env` em uma lista de env vars.
/// Ignora linhas vazias e comentários (`#`). Remove aspas simples/duplas do valor.
pub fn parse_dotenv(text: &str) -> Vec<shared::EnvVar> {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let (k, v) = l.split_once('=')?;
            let key = k.trim().to_string();
            if key.is_empty() {
                return None;
            }
            let v = v.trim();
            let val = if v.len() >= 2
                && ((v.starts_with('"') && v.ends_with('"'))
                    || (v.starts_with('\'') && v.ends_with('\'')))
            {
                v[1..v.len() - 1].to_string()
            } else {
                v.to_string()
            };
            Some(shared::EnvVar {
                key,
                value: shared::EnvVarValue::Plain(val),
            })
        })
        .collect()
}

pub fn status_color(status: &ServiceStatus) -> iced::Color {
    match status {
        ServiceStatus::Running => palette::GREEN,
        ServiceStatus::Stopping | ServiceStatus::Deploying => palette::YELLOW,
        ServiceStatus::Stopped => palette::GRAY,
        ServiceStatus::Degraded => palette::MAGENTA,
        ServiceStatus::Error(_) => palette::RED,
    }
}

#[allow(dead_code)]
fn _assert_service(_s: &Service) {}

/// Merge a freshly polled log tail into an existing buffer without duplicating
/// lines that were already seen. Logs are append-only and both slices end at the
/// most recent line, so we find the largest overlap where a suffix of `buf`
/// matches a prefix of `incoming` (comparing text + stream only — timestamps are
/// re-stamped by the daemon on every poll) and append just the new lines past it.
/// Returns `true` if `buf` was modified (i.e. the editor needs rebuilding).
fn merge_logs(buf: &mut Vec<LogLine>, incoming: Vec<LogLine>) -> bool {
    let same = |a: &LogLine, b: &LogLine| a.text == b.text && a.is_stderr == b.is_stderr;

    // Largest k such that the last k lines of `buf` equal the first k of `incoming`.
    let max_k = buf.len().min(incoming.len());
    let mut overlap = 0;
    for k in (1..=max_k).rev() {
        if buf[buf.len() - k..]
            .iter()
            .zip(&incoming[..k])
            .all(|(a, b)| same(a, b))
        {
            overlap = k;
            break;
        }
    }

    if overlap == 0 {
        // No common tail: either first load (empty buf) or full log turnover.
        if buf.is_empty() && incoming.is_empty() {
            return false;
        }
        *buf = incoming;
    } else if overlap == incoming.len() {
        // Everything in `incoming` is already present — nothing new.
        return false;
    } else {
        buf.extend(incoming.into_iter().skip(overlap));
    }

    if buf.len() > MAX_LOG_LINES {
        let excess = buf.len() - MAX_LOG_LINES;
        buf.drain(..excess);
    }
    true
}

#[cfg(test)]
mod merge_tests {
    use super::merge_logs;
    use crate::model::LogLine;

    fn line(text: &str) -> LogLine {
        LogLine { timestamp: chrono::Utc::now(), text: text.into(), is_stderr: false }
    }
    fn lines(texts: &[&str]) -> Vec<LogLine> {
        texts.iter().map(|t| line(t)).collect()
    }
    fn texts(buf: &[LogLine]) -> Vec<String> {
        buf.iter().map(|l| l.text.clone()).collect()
    }

    #[test]
    fn first_load_takes_everything() {
        let mut buf = vec![];
        assert!(merge_logs(&mut buf, lines(&["a", "b", "c"])));
        assert_eq!(texts(&buf), ["a", "b", "c"]);
    }

    #[test]
    fn identical_poll_is_noop() {
        let mut buf = lines(&["a", "b", "c"]);
        assert!(!merge_logs(&mut buf, lines(&["a", "b", "c"])));
        assert_eq!(texts(&buf), ["a", "b", "c"]);
    }

    #[test]
    fn appends_only_new_lines() {
        let mut buf = lines(&["a", "b", "c"]);
        assert!(merge_logs(&mut buf, lines(&["a", "b", "c", "d", "e"])));
        assert_eq!(texts(&buf), ["a", "b", "c", "d", "e"]);
    }

    #[test]
    fn appends_with_truncated_window() {
        // poll window dropped the oldest line but overlaps on the tail
        let mut buf = lines(&["a", "b", "c"]);
        assert!(merge_logs(&mut buf, lines(&["b", "c", "d"])));
        assert_eq!(texts(&buf), ["a", "b", "c", "d"]);
    }

    #[test]
    fn full_turnover_replaces() {
        let mut buf = lines(&["a", "b", "c"]);
        assert!(merge_logs(&mut buf, lines(&["x", "y", "z"])));
        assert_eq!(texts(&buf), ["x", "y", "z"]);
    }

    #[test]
    fn repeated_identical_lines_grow_by_one() {
        let mut buf = lines(&["x", "x"]);
        assert!(merge_logs(&mut buf, lines(&["x", "x", "x"])));
        assert_eq!(texts(&buf), ["x", "x", "x"]);
    }
}
