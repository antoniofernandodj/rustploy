pub use crate::models::*;

use shared::{
    Command, ContainerMetricsPoint, DeployEngineSummary, DeployState, Deployment,
    DeploymentSummary, Event, Project, Response, Service, ServiceStatus,
};

use std::collections::{HashMap, VecDeque};

pub struct App {
    pub focus: Focus,
    pub sidebar_cursor: usize,
    pub view: View,

    pub projects: Vec<Project>,
    pub projects_cursor: usize,
    pub services: Vec<Service>,
    pub active_project_id: Option<String>,
    pub active_service_id: Option<String>,

    pub creating_project: bool,
    pub new_proj_name: String,
    pub new_proj_desc: String,
    pub new_proj_field: usize,

    pub service_cursor: usize,
    pub service_filter: String,
    pub service_filtering: bool,

    pub project_detail_tab: ProjectDetailTab,
    pub project_env_tab: EnvTabState,
    pub project_settings: ProjectSettingsState,

    pub service_tab: ServiceTab,
    pub general_tab: GeneralTabState,
    pub healthcheck_tab: HealthcheckTabState,
    pub domains_tab: DomainsTabState,
    pub advanced_tab: AdvancedTabState,
    pub env_tab: EnvTabState,
    pub compose_tab: ComposeTabState,
    pub deployment_cursor: usize,
    pub build_log_scroll: usize, // offset from top; usize::MAX = follow tail
    pub log_cursor: usize,

    pub new_service: Option<NewServiceState>,

    pub home_deployments: Vec<DeploymentSummary>,
    pub service_deployments: Vec<Deployment>,
    pub deploy_progress: HashMap<String, DeployProgressState>,
    /// Build output per deployment_id — populated from Event::BuildLog.
    pub build_logs: HashMap<String, VecDeque<LogLine>>,
    /// Container stdout/stderr per service_id — populated from Event::LogLine.
    pub logs: HashMap<String, VecDeque<LogLine>>,
    pub metrics: HashMap<String, VecDeque<ContainerMetricsPoint>>,

    pub notification: Option<Notification>,
    pub last_deployment: Option<Deployment>,

    pub pending_commands: Vec<PendingCommand>,

    pub log_refresh_ticks: u32,
    pub deploy_engine_refresh_ticks: u32,
    pub deploy_engine: Option<DeployEngineSummary>,

    pub webhook_url: Option<String>,
    pub server_settings: ServerSettingsState,
    pub project_secrets: Vec<String>,
    pub secrets_tab: SecretsTabState,
}

impl App {
    pub fn new() -> Self {
        Self {
            focus: Focus::Sidebar,
            sidebar_cursor: 0,
            view: View::HomeDeployments,

            projects: vec![],
            projects_cursor: 0,
            services: vec![],
            active_project_id: None,
            active_service_id: None,

            creating_project: false,
            new_proj_name: String::new(),
            new_proj_desc: String::new(),
            new_proj_field: 0,

            service_cursor: 0,
            service_filter: String::new(),
            service_filtering: false,

            project_detail_tab: ProjectDetailTab::default(),
            project_env_tab: EnvTabState::default(),
            project_settings: ProjectSettingsState::default(),

            service_tab: ServiceTab::General,
            general_tab: GeneralTabState::default(),
            healthcheck_tab: HealthcheckTabState::default(),
            domains_tab: DomainsTabState::default(),
            advanced_tab: AdvancedTabState::default(),
            env_tab: EnvTabState::default(),
            compose_tab: ComposeTabState::default(),
            deployment_cursor: 0,
            build_log_scroll: usize::MAX,
            log_cursor: 0,

            new_service: None,

            home_deployments: vec![],
            service_deployments: vec![],
            deploy_progress: HashMap::new(),
            build_logs: HashMap::new(),
            logs: HashMap::new(),
            metrics: HashMap::new(),

            notification: None,
            last_deployment: None,

            pending_commands: vec![],

            log_refresh_ticks: 0,
            deploy_engine_refresh_ticks: 0,
            deploy_engine: None,

            webhook_url: None,
            server_settings: ServerSettingsState::default(),
            project_secrets: vec![],
            secrets_tab: SecretsTabState::default(),
        }
    }

    pub fn selectable_sidebar_items(&self) -> Vec<SidebarItem> {
        let mut items = vec![
            SidebarItem::HomeDeployments,
            SidebarItem::HomeMonitoring,
            SidebarItem::HomeSchedules,
            SidebarItem::HomeIngress,
            SidebarItem::HomeDocker,
            SidebarItem::HomeDeployEngine,
            SidebarItem::HomeRequests,
            SidebarItem::Projects,
        ];
        items.extend([
            SidebarItem::SettingsWebServer,
            SidebarItem::SettingsProfile,
            SidebarItem::SettingsUsers,
            SidebarItem::SettingsAuditLogs,
            SidebarItem::SettingsSshKeys,
            SidebarItem::SettingsTags,
            SidebarItem::SettingsGit,
            SidebarItem::SettingsRegistry,
            SidebarItem::SettingsS3,
            SidebarItem::SettingsCerts,
            SidebarItem::SettingsSso,
            SidebarItem::Account,
        ]);
        items
    }

    pub fn current_sidebar_item(&self) -> Option<SidebarItem> {
        self.selectable_sidebar_items()
            .into_iter()
            .nth(self.sidebar_cursor)
    }

    pub fn sidebar_move_up(&mut self) {
        if self.sidebar_cursor > 0 {
            self.sidebar_cursor -= 1;
        }
    }

    pub fn sidebar_move_down(&mut self) {
        let max = self.selectable_sidebar_items().len().saturating_sub(1);
        if self.sidebar_cursor < max {
            self.sidebar_cursor += 1;
        }
    }

    pub fn sidebar_select(&mut self) {
        let item = match self.current_sidebar_item() {
            Some(i) => i,
            None => return,
        };

        match &item {
            SidebarItem::HomeDeployments => {
                self.view = View::HomeDeployments;
                self.focus = Focus::Content;
                self.home_deployments.clear();
                self.pending_commands.push(PendingCommand {
                    command: Command::RecentDeployments { limit: 30 },
                    context: CmdContext::LoadHomeDeployments,
                });
            }
            SidebarItem::Projects => {
                self.view = View::Projects;
                self.focus = Focus::Content;
                self.projects_cursor = 0;
            }
            SidebarItem::HomeDeployEngine => {
                self.view = View::HomeDeployEngine;
                self.focus = Focus::Content;
                self.deploy_engine = None;
                self.deploy_engine_refresh_ticks = 0;
                self.pending_commands.push(PendingCommand {
                    command: Command::DeployEngineStatus,
                    context: CmdContext::LoadDeployEngine,
                });
            }
            SidebarItem::SettingsWebServer => {
                self.view = View::SettingsWebServer;
                self.focus = Focus::Content;
                if !self.server_settings.loaded {
                    self.pending_commands.push(PendingCommand {
                        command: Command::GetDaemonSettings,
                        context: CmdContext::LoadServerSettings,
                    });
                }
            }
            other => {
                if let Some(view) = other.to_view() {
                    self.view = view;
                }
                self.focus = Focus::Content;
            }
        }
    }

    pub fn filtered_services(&self) -> Vec<&Service> {
        if self.service_filter.is_empty() {
            self.services.iter().collect()
        } else {
            let f = self.service_filter.to_lowercase();
            self.services
                .iter()
                .filter(|s| s.spec.name.to_lowercase().contains(&f))
                .collect()
        }
    }

    pub fn current_service(&self) -> Option<&Service> {
        self.filtered_services()
            .into_iter()
            .nth(self.service_cursor)
    }

    pub fn current_project(&self) -> Option<&Project> {
        let pid = self.active_project_id.as_deref()?;
        self.projects.iter().find(|p| p.id == pid)
    }

    pub fn current_active_service(&self) -> Option<&Service> {
        let sid = self.active_service_id.as_deref()?;
        self.services.iter().find(|s| s.id == sid)
    }

    pub fn open_project(&mut self, idx: usize) {
        if let Some(project) = self.projects.get(idx) {
            let pid = project.id.clone();
            self.project_settings = ProjectSettingsState {
                focused: ProjectSettingsField::default(),
                name: project.name.clone(),
                description: project.description.clone().unwrap_or_default(),
            };
            self.active_project_id = Some(pid.clone());
            self.view = View::ProjectDetail;
            self.project_detail_tab = ProjectDetailTab::Services;
            self.project_env_tab = EnvTabState::default();
            self.service_cursor = 0;
            self.service_filter = String::new();
            self.service_filtering = false;
            self.focus = Focus::Content;
            self.pending_commands.push(PendingCommand {
                command: Command::ServiceList { project_id: pid },
                context: CmdContext::LoadServices,
            });
        }
    }

    pub fn open_service(&mut self, svc: &Service) {
        self.active_service_id = Some(svc.id.clone());
        self.service_tab = ServiceTab::General;
        self.general_tab = GeneralTabState::from_service(svc);
        self.healthcheck_tab = HealthcheckTabState::from_service(svc);
        self.domains_tab = DomainsTabState::from_service(svc);
        self.advanced_tab = AdvancedTabState::from_service(svc);
        self.env_tab = EnvTabState::default();
        self.compose_tab = if let shared::ServiceSource::Compose(c) = &svc.spec.source {
            ComposeTabState::new(&c.content)
        } else {
            ComposeTabState::default()
        };
        self.deployment_cursor = 0;
        self.build_log_scroll = usize::MAX;
        self.log_cursor = 0;
        self.service_deployments = vec![];
        self.view = View::ServiceDetail;
        self.focus = Focus::Content;
        self.pending_commands.push(PendingCommand {
            command: Command::DeployHistory {
                service_id: svc.id.clone(),
                limit: 10,
            },
            context: CmdContext::LoadDeployments,
        });
        self.pending_commands.push(PendingCommand {
            command: Command::LogsGet {
                service_id: svc.id.clone(),
                tail: 500,
            },
            context: CmdContext::LoadLogs,
        });
        // Busca webhook URL somente para serviços Application (não Compose)
        if !matches!(svc.spec.source, shared::ServiceSource::Compose(_)) {
            self.webhook_url = None;
            self.pending_commands.push(PendingCommand {
                command: Command::GetWebhookUrl {
                    service_id: svc.id.clone(),
                },
                context: CmdContext::LoadWebhookUrl,
            });
        } else {
            self.webhook_url = None;
        }
    }

    pub fn set_notification(&mut self, msg: impl Into<String>, is_error: bool) {
        self.notification = Some(Notification {
            message: msg.into(),
            is_error,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(4),
        });
    }

    pub fn tick(&mut self) {
        if let Some(n) = &self.notification {
            if n.expires_at <= std::time::Instant::now() {
                self.notification = None;
            }
        }

        // Auto-refresh logs a cada ~5s enquanto a aba Logs estiver ativa
        if self.service_tab == ServiceTab::Logs {
            self.log_refresh_ticks += 1;
            if self.log_refresh_ticks >= 50 {
                self.log_refresh_ticks = 0;
                if let Some(sid) = self.active_service_id.clone() {
                    self.pending_commands.push(PendingCommand {
                        command: Command::LogsGet {
                            service_id: sid,
                            tail: 500,
                        },
                        context: CmdContext::LoadLogs,
                    });
                }
            }
        } else {
            self.log_refresh_ticks = 0;
        }

        // Auto-refresh Deploy Engine a cada ~5s
        if self.view == View::HomeDeployEngine {
            self.deploy_engine_refresh_ticks += 1;
            if self.deploy_engine_refresh_ticks >= 50 {
                self.deploy_engine_refresh_ticks = 0;
                self.pending_commands.push(PendingCommand {
                    command: Command::DeployEngineStatus,
                    context: CmdContext::LoadDeployEngine,
                });
            }
        } else {
            self.deploy_engine_refresh_ticks = 0;
        }
    }

    pub fn apply_event(&mut self, event: Event) {
        match event {
            Event::ServiceStatusChanged { service_id, status } => {
                if let Some(svc) = self.services.iter_mut().find(|s| s.id == service_id) {
                    svc.status = status.clone();
                }
                // Quando o serviço fica Running e está aberto, recarrega logs automaticamente
                if matches!(status, ServiceStatus::Running)
                    && self.active_service_id.as_deref() == Some(&service_id)
                {
                    self.logs.remove(&service_id);
                    self.pending_commands.push(PendingCommand {
                        command: Command::LogsGet {
                            service_id,
                            tail: 500,
                        },
                        context: CmdContext::LoadLogs,
                    });
                }
            }

            Event::DeployStateChanged {
                deployment_id,
                service_id,
                state,
                message,
                ..
            } => {
                if matches!(state, DeployState::RollingBack) {
                    let reason = message.as_deref().unwrap_or("motivo desconhecido");
                    self.set_notification(format!("Deploy falhou: {reason}"), true);
                }

                // Atualiza na home de deployments se estiver carregada.
                if let Some(s) = self
                    .home_deployments
                    .iter_mut()
                    .find(|s| s.deployment.id == deployment_id)
                {
                    s.deployment.state = state.clone();
                }

                if let Some(dep) = self
                    .service_deployments
                    .iter_mut()
                    .find(|d| d.id == deployment_id)
                {
                    dep.state = state.clone();
                } else if self.active_service_id.as_deref() == Some(&service_id) {
                    // Evento chegou antes de Response::Deployment — insere placeholder
                    // que será sobrescrito quando a resposta RPC chegar.
                    self.service_deployments.insert(
                        0,
                        Deployment {
                            id: deployment_id.clone(),
                            service_id: service_id.clone(),
                            image: String::new(),
                            state: state.clone(),
                            states_log: vec![],
                            started_at: chrono::Utc::now(),
                            finished_at: None,
                        },
                    );
                }

                let entry = self
                    .deploy_progress
                    .entry(deployment_id.clone())
                    .or_insert_with(|| DeployProgressState {
                        deployment_id: deployment_id.clone(),
                        service_id: service_id.clone(),
                        current_state: state.clone(),
                        percent: 0,
                        description: String::new(),
                        states_seen: vec![],
                    });
                entry.states_seen.push(entry.current_state.clone());
                entry.current_state = state;
                entry.percent = entry.current_state.to_percent();
            }

            Event::DeployProgress {
                deployment_id,
                percent,
                description,
                ..
            } => {
                if let Some(p) = self.deploy_progress.get_mut(&deployment_id) {
                    p.percent = percent;
                    p.description = description;
                }
            }

            Event::BuildLog {
                deployment_id,
                line,
                timestamp,
                ..
            } => {
                let buf = self.build_logs.entry(deployment_id).or_default();
                if buf.len() >= MAX_LOG_LINES {
                    buf.pop_front();
                }
                buf.push_back(LogLine {
                    timestamp,
                    text: line,
                    is_stderr: false,
                });
            }

            Event::LogLine {
                service_id,
                stream,
                line,
                timestamp,
                ..
            } => {
                let buf = self.logs.entry(service_id).or_default();
                if buf.len() >= MAX_LOG_LINES {
                    buf.pop_front();
                }
                buf.push_back(LogLine {
                    timestamp,
                    text: line,
                    is_stderr: stream == shared::protocol::LogStream::Stderr,
                });
            }

            Event::ContainerMetrics(m) => {
                let buf = self.metrics.entry(m.service_id.clone()).or_default();
                if buf.len() >= MAX_METRIC_POINTS {
                    buf.pop_front();
                }
                buf.push_back(m);
            }

            Event::Error { message, .. } => {
                self.set_notification(message, true);
            }

            Event::DaemonReady { version } => {
                self.set_notification(format!("daemon {version} ready"), false);
            }
        }
    }

    pub fn handle_response(&mut self, resp: Response, ctx: CmdContext) {
        match (resp, ctx) {
            (Response::Projects(projects), CmdContext::LoadProjects) => {
                self.projects = projects;
            }
            (Response::Services(services), CmdContext::LoadServices) => {
                self.services = services;
            }
            (Response::Project(p), CmdContext::CreateProject) => {
                self.projects.push(p);
                self.set_notification("Projeto criado", false);
            }
            (Response::Project(p), CmdContext::UpdateProject) => {
                if let Some(existing) = self.projects.iter_mut().find(|x| x.id == p.id) {
                    *existing = p.clone();
                }
                self.project_settings.name = p.name;
                self.project_settings.description = p.description.unwrap_or_default();
                self.set_notification("Projeto atualizado", false);
            }
            (Response::Project(p), CmdContext::UpdateProjectEnv) => {
                if let Some(existing) = self.projects.iter_mut().find(|x| x.id == p.id) {
                    *existing = p;
                }
                self.set_notification("Env vars do projeto atualizadas", false);
            }
            (Response::Service(s), CmdContext::CreateService) => {
                self.services.push(s);
                self.view = View::ProjectDetail;
                self.set_notification("Serviço criado", false);
            }
            (Response::Service(s), CmdContext::UpdateService) => {
                if let Some(existing) = self.services.iter_mut().find(|x| x.id == s.id) {
                    *existing = s.clone();
                }
                if self.active_service_id.as_deref() == Some(&s.id) {
                    self.general_tab = GeneralTabState::from_service(&s);
                    self.healthcheck_tab = HealthcheckTabState::from_service(&s);
                    self.domains_tab = DomainsTabState::from_service(&s);
                    self.advanced_tab = AdvancedTabState::from_service(&s);
                    if let shared::ServiceSource::Compose(c) = &s.spec.source {
                        self.compose_tab = ComposeTabState::new(&c.content);
                    }
                }
                self.set_notification("Serviço atualizado", false);
            }
            (Response::Ok, CmdContext::DeleteProject(pid)) => {
                self.projects.retain(|p| p.id != pid);
                if self.active_project_id.as_deref() == Some(&pid) {
                    self.services.clear();
                    self.active_project_id = None;
                }
                self.projects_cursor =
                    self.projects_cursor.min(self.projects.len().saturating_sub(1));
                self.view = View::Projects;
                self.set_notification("Projeto removido", false);
            }
            (Response::Ok, CmdContext::DeleteService(sid)) => {
                self.services.retain(|s| s.id != sid);
                if self.active_service_id.as_deref() == Some(&sid) {
                    self.active_service_id = None;
                }
                self.view = View::ProjectDetail;
                self.set_notification("Serviço removido", false);
            }
            (Response::Deployments(deps), CmdContext::LoadDeployments) => {
                if let Some(first) = deps.first() {
                    self.pending_commands.push(PendingCommand {
                        command: Command::GetBuildLogs {
                            deployment_id: first.id.clone(),
                        },
                        context: CmdContext::LoadBuildLogs,
                    });
                }
                self.service_deployments = deps;
            }
            (Response::DeploymentSummaries(summaries), CmdContext::LoadHomeDeployments) => {
                self.home_deployments = summaries;
            }
            (Response::BuildLogs(entries), CmdContext::LoadBuildLogs) => {
                // deposit into whichever deployment is currently selected
                if let Some(dep) = self.service_deployments.get(
                    self.deployment_cursor
                        .min(self.service_deployments.len().saturating_sub(1)),
                ) {
                    let buf = self.build_logs.entry(dep.id.clone()).or_default();
                    buf.clear();
                    for e in entries {
                        buf.push_back(LogLine {
                            timestamp: e.timestamp,
                            text: e.line,
                            is_stderr: false,
                        });
                    }
                }
            }
            (Response::Logs(entries), CmdContext::LoadLogs) => {
                if let Some(sid) = &self.active_service_id.clone() {
                    // Só substitui o buffer se Docker retornou linhas.
                    // Se vier vazio (ex: stdout buffered), mantém os logs
                    // acumulados via Event::LogLine para não perder histórico.
                    if !entries.is_empty() {
                        let buf = self.logs.entry(sid.clone()).or_default();
                        buf.clear();
                        for e in entries {
                            buf.push_back(LogLine {
                                timestamp: e.timestamp,
                                text: e.line,
                                is_stderr: e.stream == shared::protocol::LogStream::Stderr,
                            });
                        }
                    }
                }
            }
            (Response::Ok, CmdContext::ServiceStop) => {
                if let Some(sid) = self.active_service_id.clone() {
                    if let Some(svc) = self.services.iter_mut().find(|s| s.id == sid) {
                        svc.status = ServiceStatus::Stopped;
                    }
                }
                self.set_notification("Serviço parado", false);
            }
            (Response::Err { message, .. }, CmdContext::ServiceStop) => {
                // Reverte para Running se o stop falhou.
                if let Some(sid) = self.active_service_id.clone() {
                    if let Some(svc) = self.services.iter_mut().find(|s| s.id == sid) {
                        svc.status = ServiceStatus::Running;
                    }
                }
                self.set_notification(message, true);
            }
            (Response::Ok, CmdContext::ServiceReload) => {
                if let Some(sid) = self.active_service_id.clone() {
                    if let Some(svc) = self.services.iter_mut().find(|s| s.id == sid) {
                        svc.status = ServiceStatus::Running;
                    }
                }
                self.set_notification("Container reiniciado", false);
            }
            (Response::Deployment(dep), CmdContext::Deploy) => {
                self.logs.remove(&dep.service_id);
                // Aplica estado já conhecido do stream (eventos podem ter chegado antes).
                let mut dep = dep;
                if let Some(progress) = self.deploy_progress.get(&dep.id) {
                    dep.state = progress.current_state.clone();
                }
                // Substitui placeholder inserido pelo evento, ou insere no topo.
                if let Some(pos) = self.service_deployments.iter().position(|d| d.id == dep.id) {
                    self.service_deployments[pos] = dep.clone();
                } else {
                    self.service_deployments.insert(0, dep.clone());
                }
                // Adiciona/atualiza na home com nomes resolvidos do estado local.
                let svc = self.services.iter().find(|s| s.id == dep.service_id);
                let service_name = svc.map(|s| s.spec.name.clone()).unwrap_or_default();
                let project_name = svc
                    .and_then(|s| self.projects.iter().find(|p| p.id == s.spec.project_id))
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                let summary = DeploymentSummary {
                    deployment: dep.clone(),
                    service_name,
                    project_name,
                };
                if let Some(pos) = self
                    .home_deployments
                    .iter()
                    .position(|s| s.deployment.id == dep.id)
                {
                    self.home_deployments[pos] = summary;
                } else {
                    self.home_deployments.insert(0, summary);
                }
                self.last_deployment = Some(dep);
                self.set_notification("Deploy iniciado ✓", false);
            }
            (Response::Ok, CmdContext::Deploy) => {
                self.set_notification("Deploy iniciado", false);
            }
            (Response::WebhookUrl(url), CmdContext::LoadWebhookUrl) => {
                self.webhook_url = url;
            }
            (Response::WebhookUrl(url), CmdContext::RegenerateWebhook) => {
                self.webhook_url = url;
                self.set_notification("Token de webhook regenerado", false);
            }
            (Response::DaemonSettings { webhook_base_url, acme_email }, CmdContext::LoadServerSettings) => {
                self.server_settings.server_domain = webhook_base_url.unwrap_or_default();
                self.server_settings.acme_email = acme_email.unwrap_or_default();
                self.server_settings.loaded = true;
            }
            (Response::Ok, CmdContext::SaveServerSettings) => {
                self.server_settings.loaded = false; // força reload na próxima visita
                self.set_notification("Configurações salvas", false);
                // Recarrega a URL do webhook do serviço atual, se houver
                if let Some(sid) = self.active_service_id.clone() {
                    self.pending_commands.push(PendingCommand {
                        command: Command::GetWebhookUrl { service_id: sid },
                        context: CmdContext::LoadWebhookUrl,
                    });
                }
            }
            (Response::DeployEngineStatus(summary), CmdContext::LoadDeployEngine) => {
                self.deploy_engine = Some(summary);
            }
            (Response::SecretNames(names), CmdContext::LoadSecrets) => {
                self.project_secrets = names;
            }
            (Response::Ok, CmdContext::SetSecret) => {
                if let Some(pid) = self.active_project_id.clone() {
                    self.pending_commands.push(PendingCommand {
                        command: Command::SecretList { project_id: pid },
                        context: CmdContext::LoadSecrets,
                    });
                }
                self.secrets_tab.adding = false;
                self.set_notification("Secret salvo", false);
            }
            (Response::Ok, CmdContext::DeleteSecret) => {
                if let Some(pid) = self.active_project_id.clone() {
                    self.pending_commands.push(PendingCommand {
                        command: Command::SecretList { project_id: pid },
                        context: CmdContext::LoadSecrets,
                    });
                }
                self.set_notification("Secret removido", false);
            }
            (Response::Err { message, .. }, _) => {
                self.set_notification(message, true);
            }
            _ => {}
        }
    }

    pub fn can_quit(&self) -> bool {
        self.focus == Focus::Sidebar && !self.creating_project && self.new_service.is_none()
    }

    pub fn visible_service_tabs(&self) -> &'static [ServiceTab] {
        let is_db = self
            .current_active_service()
            .map(|s| DbKind::detect_from_env(&s.spec.env_vars).is_some())
            .unwrap_or(false);
        if is_db {
            ServiceTab::all_with_connection()
        } else {
            ServiceTab::all()
        }
    }

    pub fn next_service_tab(&self) -> ServiceTab {
        let tabs = self.visible_service_tabs();
        let idx = tabs
            .iter()
            .position(|t| t == &self.service_tab)
            .unwrap_or(0);
        tabs[(idx + 1) % tabs.len()].clone()
    }

    pub fn prev_service_tab(&self) -> ServiceTab {
        let tabs = self.visible_service_tabs();
        let idx = tabs
            .iter()
            .position(|t| t == &self.service_tab)
            .unwrap_or(0);
        let prev = if idx == 0 { tabs.len() - 1 } else { idx - 1 };
        tabs[prev].clone()
    }
}
