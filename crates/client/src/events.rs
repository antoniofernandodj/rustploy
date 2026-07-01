use crate::app::{
    AdvancedField, App, CmdContext, ConfirmAction, DbKind, DockerPruneButton, EnvEditField,
    EnvTabState, Focus, GeneralTabField, HcField, NewServiceState, NewServiceStep, PendingCommand,
    ProjectDetailTab, ProjectSettingsField, PruneSlot, SecretEditField, SecretsTabState,
    ServerSettingsField, ServiceTab, View,
};
use crossterm::event::KeyModifiers;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use shared::ServiceSource;
use shared::{self, Command, EnvVar, EnvVarValue};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    if app.creating_project {
        handle_new_project(app, key);
        return;
    }

    if app.new_service.is_some() {
        handle_new_service(app, key);
        return;
    }

    if let View::Confirm { .. } = &app.view.clone() {
        handle_confirm(app, key);
        return;
    }

    if app.project_env_text.editing {
        handle_project_env_textarea(app, key);
        return;
    }

    match key.code {
        KeyCode::Tab => {
            // Cede o Tab quando algum formulário de edição está aberto
            // (env vars de serviço ou de projeto precisam de Tab para KEY→VALUE)
            let editing = app.env_tab.editing || app.project_env_tab.editing || app.project_env_text.editing || app.secrets_tab.adding;
            if !editing {
                app.focus = match app.focus {
                    Focus::Sidebar => Focus::Content,
                    Focus::Content => Focus::Sidebar,
                };
                return;
            }
            // cai no dispatch de view abaixo
        }
        KeyCode::Esc => {
            match &app.view {
                View::ServiceDetail => {
                    app.view = View::ProjectDetail;
                    app.active_service_id = None;
                }
                View::ProjectDetail => {
                    app.view = View::Projects;
                    app.active_project_id = None;
                    app.service_filtering = false;
                }
                _ => {
                    app.focus = Focus::Sidebar;
                    app.service_filtering = false;
                }
            }
            return;
        }
        _ => {}
    }

    match app.focus {
        Focus::Sidebar => handle_sidebar(app, key),
        Focus::Content => handle_content(app, key),
    }
}

fn handle_sidebar(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => app.sidebar_move_up(),
        KeyCode::Down => app.sidebar_move_down(),
        KeyCode::Enter => app.sidebar_select(),
        _ => {}
    }
}

fn handle_content(app: &mut App, key: KeyEvent) {
    match app.view.clone() {
        View::Projects => handle_projects_list(app, key),
        View::ProjectDetail => handle_project_detail(app, key),
        View::ServiceDetail => handle_service_detail(app, key),
        View::SettingsWebServer => handle_settings_web_server(app, key),
        View::HomeDeployEngine => handle_home_deploy_engine(app, key),
        View::HomeDocker => handle_docker_cleanup(app, key),
        View::HomeMonitoring
        | View::HomeDeployments
        | View::HomeSchedules
        | View::HomeIngress
        | View::HomeRequests => handle_home(app, key),
        _ => {}
    }
}

fn handle_home(_app: &mut App, _key: KeyEvent) {}

fn handle_home_deploy_engine(app: &mut App, key: KeyEvent) {
    if key.code == KeyCode::Char('r') {
        app.deploy_engine = None;
        app.pending_commands.push(PendingCommand {
            command: Command::DeployEngineStatus,
            context: CmdContext::LoadDeployEngine,
        });
    }
}

fn handle_projects_list(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Tab | KeyCode::Esc => {
            app.focus = Focus::Sidebar;
        }
        KeyCode::Up => {
            if app.projects_cursor > 0 {
                app.projects_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.projects_cursor + 1 < app.projects.len() {
                app.projects_cursor += 1;
            }
        }
        KeyCode::Enter => {
            app.open_project(app.projects_cursor);
        }
        KeyCode::Char('n') => {
            app.creating_project = true;
            app.new_proj_name = String::new();
            app.new_proj_desc = String::new();
            app.new_proj_field = 0;
        }
        KeyCode::Char('D') => {
            if let Some(project) = app.projects.get(app.projects_cursor) {
                let has_services = app
                    .services
                    .iter()
                    .any(|s| s.spec.project_id == project.id);
                if !has_services {
                    let pid = project.id.clone();
                    let name = project.name.clone();
                    app.view = View::Confirm {
                        message: format!(
                            "Remover projeto '{name}'? Esta ação não pode ser desfeita."
                        ),
                        action: ConfirmAction::DeleteProject(pid),
                    };
                }
            }
        }
        _ => {}
    }
}

fn handle_project_detail(app: &mut App, key: KeyEvent) {
    let is_editing = app.service_filtering
        || app.project_env_tab.editing
        || app.project_env_text.editing
        || (app.project_detail_tab == ProjectDetailTab::Settings
            && app.project_settings.focused.clone().is_text())
        || (app.project_detail_tab == ProjectDetailTab::Secrets && app.secrets_tab.adding);

    if !is_editing {
        match key.code {
            KeyCode::Left => {
                app.project_detail_tab = app.project_detail_tab.prev();
                on_project_tab_change(app);
                return;
            }
            KeyCode::Right => {
                app.project_detail_tab = app.project_detail_tab.next();
                on_project_tab_change(app);
                return;
            }
            KeyCode::Char('1') => {
                app.project_detail_tab = ProjectDetailTab::Services;
                on_project_tab_change(app);
                return;
            }
            KeyCode::Char('2') => {
                app.project_detail_tab = ProjectDetailTab::Environment;
                on_project_tab_change(app);
                return;
            }
            KeyCode::Char('3') => {
                app.project_detail_tab = ProjectDetailTab::Secrets;
                on_project_tab_change(app);
                return;
            }
            KeyCode::Char('4') => {
                app.project_detail_tab = ProjectDetailTab::Settings;
                on_project_tab_change(app);
                return;
            }
            _ => {}
        }
    }

    match app.project_detail_tab.clone() {
        ProjectDetailTab::Services => handle_project_services_tab(app, key),
        ProjectDetailTab::Environment => handle_project_env_tab(app, key),
        ProjectDetailTab::Settings => handle_project_settings_tab(app, key),
        ProjectDetailTab::Secrets => handle_project_secrets_tab(app, key),
    }
}

fn on_project_tab_change(app: &mut App) {
    match app.project_detail_tab {
        ProjectDetailTab::Settings => {
            let data = app
                .current_project()
                .map(|p| (p.name.clone(), p.description.clone().unwrap_or_default()));
            if let Some((name, description)) = data {
                app.project_settings.name = name;
                app.project_settings.description = description;
                app.project_settings.focused = ProjectSettingsField::default();
            }
        }
        ProjectDetailTab::Secrets => {
            if let Some(pid) = app.active_project_id.clone() {
                app.project_secrets.clear();
                app.pending_commands.push(PendingCommand {
                    command: shared::Command::SecretList { project_id: pid },
                    context: CmdContext::LoadSecrets,
                });
            }
        }
        _ => {}
    }
}

fn handle_project_services_tab(app: &mut App, key: KeyEvent) {
    if app.service_filtering {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                app.service_filtering = false;
            }
            KeyCode::Char(c) => {
                app.service_filter.push(c);
                app.service_cursor = 0;
            }
            KeyCode::Backspace => {
                app.service_filter.pop();
                app.service_cursor = 0;
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Up => {
            if app.service_cursor > 0 {
                app.service_cursor -= 1;
            }
        }
        KeyCode::Down => {
            let max = app.filtered_services().len().saturating_sub(1);
            if app.service_cursor < max {
                app.service_cursor += 1;
            }
        }
        KeyCode::Enter => {
            if let Some(svc) = app.current_service().cloned() {
                app.open_service(&svc);
            }
        }
        KeyCode::Char('/') => {
            app.service_filtering = true;
            app.service_filter = String::new();
            app.service_cursor = 0;
        }
        KeyCode::Char('n') => {
            if let Some(pid) = app.active_project_id.clone() {
                app.new_service = Some(NewServiceState::new(pid));
            }
        }
        KeyCode::Char('D') => {
            if let Some(svc) = app.current_service() {
                let id = svc.id.clone();
                let name = svc.spec.name.clone();
                app.view = View::Confirm {
                    message: format!("Remover serviço '{name}'?"),
                    action: ConfirmAction::DeleteService(id),
                };
            }
        }
        _ => {}
    }
}

fn handle_project_env_tab(app: &mut App, key: KeyEvent) {
    // ── Modo edição ───────────────────────────────────────────────────────────
    if app.project_env_tab.editing {
        match key.code {
            KeyCode::Tab => {
                app.project_env_tab.edit_field = match app.project_env_tab.edit_field {
                    EnvEditField::Key => EnvEditField::Value,
                    EnvEditField::Value => EnvEditField::Key,
                };
            }
            KeyCode::Esc => {
                app.project_env_tab.editing = false;
            }
            KeyCode::Enter => {
                let k = app.project_env_tab.edit_key.clone();
                let v = app.project_env_tab.edit_value.clone();
                if !k.is_empty() {
                    if let Some(pid) = app.active_project_id.clone() {
                        if let Some(project) = app.projects.iter().find(|p| p.id == pid) {
                            let mut env_vars = project.env_vars.clone();
                            env_vars.retain(|e| e.key != k);
                            env_vars.push(shared::EnvVar {
                                key: k,
                                value: if let Some(n) = v.strip_prefix("secret:") {
                                            shared::EnvVarValue::Secret(n.to_string())
                                        } else {
                                            shared::EnvVarValue::Plain(v)
                                        },
                            });
                            app.pending_commands.push(PendingCommand {
                                command: shared::Command::ProjectEnvSet {
                                    project_id: pid,
                                    env_vars,
                                },
                                context: CmdContext::UpdateProjectEnv,
                            });
                        }
                    }
                }
                app.project_env_tab.editing = false;
            }
            KeyCode::Char(c) => match app.project_env_tab.edit_field {
                EnvEditField::Key => app.project_env_tab.edit_key.push(c),
                EnvEditField::Value => app.project_env_tab.edit_value.push(c),
            },
            KeyCode::Backspace => match app.project_env_tab.edit_field {
                EnvEditField::Key => {
                    app.project_env_tab.edit_key.pop();
                }
                EnvEditField::Value => {
                    app.project_env_tab.edit_value.pop();
                }
            },
            _ => {}
        }
        return;
    }

    // ── Navegação ─────────────────────────────────────────────────────────────
    let env_len = app
        .active_project_id
        .as_deref()
        .and_then(|pid| app.projects.iter().find(|p| p.id == pid))
        .map(|p| p.env_vars.len())
        .unwrap_or(0);

    match key.code {
        KeyCode::Up => {
            if app.project_env_tab.cursor > 0 {
                app.project_env_tab.cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.project_env_tab.cursor + 1 < env_len {
                app.project_env_tab.cursor += 1;
            }
        }
        KeyCode::Char('n') => {
            app.project_env_tab = EnvTabState {
                cursor: app.project_env_tab.cursor,
                editing: true,
                edit_key: String::new(),
                edit_value: String::new(),
                edit_field: EnvEditField::Key,
            };
        }
        KeyCode::Char('e') => {
            if let Some(project) = app
                .active_project_id
                .as_deref()
                .and_then(|pid| app.projects.iter().find(|p| p.id == pid))
            {
                if let Some(ev) = project.env_vars.get(app.project_env_tab.cursor) {
                    let edit_value = match &ev.value {
                        shared::EnvVarValue::Plain(v) => v.clone(),
                        shared::EnvVarValue::Secret(s) => format!("<secret:{s}>"),
                    };
                    app.project_env_tab = EnvTabState {
                        cursor: app.project_env_tab.cursor,
                        editing: true,
                        edit_key: ev.key.clone(),
                        edit_value,
                        edit_field: EnvEditField::Key,
                    };
                }
            }
        }
        KeyCode::Char('D') => {
            if let Some(pid) = app.active_project_id.clone() {
                if let Some(project) = app.projects.iter().find(|p| p.id == pid) {
                    if let Some(ev) = project.env_vars.get(app.project_env_tab.cursor) {
                        let key = ev.key.clone();
                        let mut env_vars = project.env_vars.clone();
                        env_vars.retain(|e| e.key != key);
                        app.pending_commands.push(PendingCommand {
                            command: shared::Command::ProjectEnvSet {
                                project_id: pid,
                                env_vars,
                            },
                            context: CmdContext::UpdateProjectEnv,
                        });
                        if app.project_env_tab.cursor > 0 {
                            app.project_env_tab.cursor -= 1;
                        }
                    }
                }
            }
        }
        KeyCode::Char('t') => {
            if let Some(pid) = &app.active_project_id {
                if let Some(project) = app.projects.iter().find(|p| &p.id == pid) {
                    app.project_env_text =
                        crate::models::EnvTextTabState::from_env_vars(&project.env_vars);
                    app.project_env_text.set_editing(true);
                }
            }
        }
        KeyCode::Char('c') => {
            if let Some(pid) = &app.active_project_id {
                if let Some(project) = app.projects.iter().find(|p| &p.id == pid) {
                    let text: String = project
                        .env_vars
                        .iter()
                        .map(|ev| {
                            let v = match &ev.value {
                                shared::EnvVarValue::Plain(v) => v.clone(),
                                shared::EnvVarValue::Secret(s) => format!("secret:{s}"),
                            };
                            format!("{}={}\n", ev.key, v)
                        })
                        .collect();
                    copy_to_clipboard(app, &text);
                }
            }
        }
        _ => {}
    }
}

fn handle_project_env_textarea(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.project_env_text.set_editing(false);
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            save_project_env_text(app);
        }
        _ => {
            app.project_env_text.textarea.input(key);
        }
    }
}

fn save_project_env_text(app: &mut App) {
    let pid = match app.active_project_id.clone() {
        Some(p) => p,
        None => return,
    };
    let env_vars = app.project_env_text.parse_env_vars();
    app.project_env_text.set_editing(false);
    app.pending_commands.push(PendingCommand {
        command: shared::Command::ProjectEnvSet {
            project_id: pid,
            env_vars,
        },
        context: CmdContext::UpdateProjectEnv,
    });
}

fn handle_project_settings_tab(app: &mut App, key: KeyEvent) {
    let focused = app.project_settings.focused.clone();

    if focused.clone().is_text() {
        match key.code {
            KeyCode::Esc => {
                app.project_settings.focused = ProjectSettingsField::default();
            }
            KeyCode::Tab | KeyCode::Down => {
                app.project_settings.focused = focused.next();
            }
            KeyCode::Up => {
                app.project_settings.focused = focused.prev();
            }
            KeyCode::Char(c) => {
                match focused {
                    ProjectSettingsField::Name => app.project_settings.name.push(c),
                    ProjectSettingsField::Description => app.project_settings.description.push(c),
                    _ => {}
                }
            }
            KeyCode::Backspace => {
                match focused {
                    ProjectSettingsField::Name => {
                        app.project_settings.name.pop();
                    }
                    ProjectSettingsField::Description => {
                        app.project_settings.description.pop();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Up => {
            app.project_settings.focused = focused.prev();
        }
        KeyCode::Down | KeyCode::Tab => {
            app.project_settings.focused = focused.next();
        }
        KeyCode::Enter | KeyCode::Char(' ') => match focused {
            ProjectSettingsField::Name | ProjectSettingsField::Description => {
                // entra em modo texto
            }
            ProjectSettingsField::Save => save_project_settings(app),
            ProjectSettingsField::Delete => request_delete_project(app),
        },
        _ => {}
    }
}

fn handle_project_secrets_tab(app: &mut App, key: KeyEvent) {
    if app.secrets_tab.adding {
        match key.code {
            KeyCode::Tab => {
                app.secrets_tab.edit_field = match app.secrets_tab.edit_field {
                    SecretEditField::Name => SecretEditField::Value,
                    SecretEditField::Value => SecretEditField::Name,
                };
            }
            KeyCode::Esc => {
                app.secrets_tab.adding = false;
            }
            KeyCode::Enter => {
                let name = app.secrets_tab.edit_name.clone();
                let value = app.secrets_tab.edit_value.clone();
                if !name.is_empty() && !value.is_empty() {
                    if let Some(pid) = app.active_project_id.clone() {
                        app.pending_commands.push(PendingCommand {
                            command: Command::SecretSet {
                                project_id: pid,
                                name,
                                value,
                            },
                            context: CmdContext::SetSecret,
                        });
                    }
                } else {
                    app.secrets_tab.adding = false;
                }
            }
            KeyCode::Char(c) => match app.secrets_tab.edit_field {
                SecretEditField::Name => app.secrets_tab.edit_name.push(c),
                SecretEditField::Value => app.secrets_tab.edit_value.push(c),
            },
            KeyCode::Backspace => match app.secrets_tab.edit_field {
                SecretEditField::Name => {
                    app.secrets_tab.edit_name.pop();
                }
                SecretEditField::Value => {
                    app.secrets_tab.edit_value.pop();
                }
            },
            _ => {}
        }
        return;
    }

    let secrets_len = app.project_secrets.len();
    match key.code {
        KeyCode::Up => {
            if app.secrets_tab.cursor > 0 {
                app.secrets_tab.cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.secrets_tab.cursor + 1 < secrets_len {
                app.secrets_tab.cursor += 1;
            }
        }
        KeyCode::Char('n') => {
            app.secrets_tab = SecretsTabState {
                cursor: app.secrets_tab.cursor,
                adding: true,
                edit_name: String::new(),
                edit_value: String::new(),
                edit_field: SecretEditField::Name,
            };
        }
        KeyCode::Char('D') => {
            if let Some(name) = app.project_secrets.get(app.secrets_tab.cursor).cloned() {
                if let Some(pid) = app.active_project_id.clone() {
                    app.pending_commands.push(PendingCommand {
                        command: Command::SecretDelete {
                            project_id: pid,
                            name,
                        },
                        context: CmdContext::DeleteSecret,
                    });
                    if app.secrets_tab.cursor > 0 {
                        app.secrets_tab.cursor -= 1;
                    }
                }
            }
        }
        _ => {}
    }
}

fn save_project_settings(app: &mut App) {
    let pid = match app.active_project_id.clone() {
        Some(p) => p,
        None => return,
    };
    let name = app.project_settings.name.trim().to_string();
    if name.is_empty() {
        app.set_notification("Nome do projeto não pode ser vazio", true);
        return;
    }
    let description = {
        let d = app.project_settings.description.trim().to_string();
        if d.is_empty() { None } else { Some(d) }
    };
    app.pending_commands.push(PendingCommand {
        command: Command::ProjectUpdate {
            id: pid,
            name,
            description,
        },
        context: CmdContext::UpdateProject,
    });
}

fn request_delete_project(app: &mut App) {
    let service_count = {
        let pid = match app.active_project_id.as_deref() {
            Some(p) => p,
            None => return,
        };
        app.services
            .iter()
            .filter(|s| s.spec.project_id == pid)
            .count()
    };
    if service_count > 0 {
        app.set_notification(
            "Remova todos os serviços antes de deletar o projeto",
            true,
        );
        return;
    }
    let pid = match app.active_project_id.clone() {
        Some(p) => p,
        None => return,
    };
    let name = app.current_project().map(|p| p.name.clone()).unwrap_or_default();
    app.view = View::Confirm {
        message: format!("Remover projeto '{name}'? Esta ação não pode ser desfeita."),
        action: ConfirmAction::DeleteProject(pid),
    };
}

fn on_tab_change(app: &mut App) {
    match app.service_tab {
        ServiceTab::General => {
            if let Some(svc) = app.current_active_service() {
                app.general_tab = crate::app::GeneralTabState::from_service(svc);
            }
        }
        ServiceTab::Logs => {
            app.log_refresh_ticks = 0;
            if let Some(sid) = app.active_service_id.clone() {
                app.logs.remove(&sid);
                app.log_cursor = 0;
                app.pending_commands.push(PendingCommand {
                    command: Command::LogsGet {
                        service_id: sid,
                        tail: 500,
                    },
                    context: CmdContext::LoadLogs,
                });
            }
        }
        _ => {}
    }
}

fn handle_service_detail(app: &mut App, key: KeyEvent) {
    // Quando a compose textarea está em modo edição, todas as teclas vão para ela
    let compose_editing = app.compose_tab.editing
        && app.service_tab == ServiceTab::General
        && app
            .current_active_service()
            .map(|s| matches!(s.spec.source, ServiceSource::Compose(_)))
            .unwrap_or(false);

    if compose_editing {
        handle_compose_textarea(app, key);
        return;
    }

    match key.code {
        KeyCode::Left => {
            app.service_tab = app.prev_service_tab();
            on_tab_change(app);
        }
        KeyCode::Right => {
            app.service_tab = app.next_service_tab();
            on_tab_change(app);
        }
        _ => match app.service_tab.clone() {
            crate::app::ServiceTab::General => handle_general_tab(app, key),
            crate::app::ServiceTab::Environment => handle_env_tab(app, key),
            crate::app::ServiceTab::Deployments => handle_deployments_tab(app, key),
            crate::app::ServiceTab::Healthcheck => handle_healthcheck_tab(app, key),
            crate::app::ServiceTab::Domains => handle_domains_tab(app, key),
            crate::app::ServiceTab::Logs => handle_logs_tab(app, key),
            crate::app::ServiceTab::Advanced => handle_advanced_tab(app, key),
            crate::app::ServiceTab::Connection
            | crate::app::ServiceTab::Metrics
            | crate::app::ServiceTab::Patches => {}
        },
    }
}

fn handle_compose_textarea(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.compose_tab.set_editing(false);
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            save_compose_content(app);
        }
        _ => {
            app.compose_tab.textarea.input(key);
        }
    }
}

fn save_compose_content(app: &mut App) {
    let sid = match app.active_service_id.clone() {
        Some(s) => s,
        None => return,
    };
    let svc = match app.services.iter().find(|s| s.id == sid) {
        Some(s) => s.clone(),
        None => return,
    };
    let content = app.compose_tab.content();
    let new_spec = shared::ServiceSpec {
        source: shared::ServiceSource::Compose(shared::ComposeSource { content }),
        ..svc.spec.clone()
    };
    app.pending_commands.push(PendingCommand {
        command: Command::ServiceUpdate {
            id: sid,
            spec: new_spec,
        },
        context: CmdContext::UpdateService,
    });
    app.compose_tab.set_editing(false);
    app.set_notification("Compose salvo.", false);
}

fn handle_general_tab(app: &mut App, key: KeyEvent) {
    let is_compose = app
        .current_active_service()
        .map(|s| matches!(s.spec.source, ServiceSource::Compose(_)))
        .unwrap_or(false);

    if is_compose {
        handle_compose_general_nav(app, key);
        return;
    }

    match key.code {
        KeyCode::Up => {
            app.general_tab.focused_field = app.general_tab.focused_field.prev();
        }
        KeyCode::Down => {
            app.general_tab.focused_field = app.general_tab.focused_field.next();
        }
        KeyCode::Enter => {
            let field = app.general_tab.focused_field;
            if field.is_button() {
                activate_general_btn(app, field);
            } else {
                app.general_tab.focused_field = field.next();
            }
        }
        KeyCode::Char(' ') => {
            let field = app.general_tab.focused_field;
            if field == GeneralTabField::Submodules {
                app.general_tab.submodules = !app.general_tab.submodules;
            } else if field.is_button() {
                activate_general_btn(app, field);
            }
        }
        KeyCode::Char(c) => {
            if app.general_tab.focused_field.is_text_field() {
                if let Some(field) = app.general_tab.focused_text_mut() {
                    field.push(c);
                }
            }
        }
        KeyCode::Backspace => {
            if app.general_tab.focused_field.is_text_field() {
                if let Some(field) = app.general_tab.focused_text_mut() {
                    field.pop();
                }
            }
        }
        _ => {}
    }
}

fn handle_compose_general_nav(app: &mut App, key: KeyEvent) {
    let field = app.general_tab.focused_field;
    match key.code {
        KeyCode::Up => {
            app.general_tab.focused_field = compose_general_prev(field);
        }
        KeyCode::Down => {
            app.general_tab.focused_field = compose_general_next(field);
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if field.is_button() {
                activate_general_btn(app, field);
            } else if field == GeneralTabField::RepoUrl {
                // RepoUrl é reaproveitado como "entrar no editor" para compose
                app.compose_tab.set_editing(true);
            }
        }
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            save_compose_content(app);
        }
        _ => {}
    }
}

fn compose_general_next(field: GeneralTabField) -> GeneralTabField {
    match field {
        GeneralTabField::BtnDeploy => GeneralTabField::BtnReload,
        GeneralTabField::BtnReload => GeneralTabField::BtnRebuild,
        GeneralTabField::BtnRebuild => GeneralTabField::BtnStop,
        GeneralTabField::BtnStop => GeneralTabField::RepoUrl,
        GeneralTabField::RepoUrl => GeneralTabField::Port,
        GeneralTabField::Port => GeneralTabField::ProviderSave,
        GeneralTabField::ProviderSave => GeneralTabField::BtnDeploy,
        _ => GeneralTabField::BtnDeploy,
    }
}

fn compose_general_prev(field: GeneralTabField) -> GeneralTabField {
    match field {
        GeneralTabField::BtnDeploy => GeneralTabField::ProviderSave,
        GeneralTabField::BtnReload => GeneralTabField::BtnDeploy,
        GeneralTabField::BtnRebuild => GeneralTabField::BtnReload,
        GeneralTabField::BtnStop => GeneralTabField::BtnRebuild,
        GeneralTabField::RepoUrl => GeneralTabField::BtnStop,
        GeneralTabField::Port => GeneralTabField::RepoUrl,
        GeneralTabField::ProviderSave => GeneralTabField::Port,
        _ => GeneralTabField::BtnDeploy,
    }
}

fn activate_general_btn(app: &mut App, field: GeneralTabField) {
    match field {
        GeneralTabField::BtnDeploy | GeneralTabField::BtnRebuild => {
            if let Some(sid) = app.active_service_id.clone() {
                app.pending_commands.push(PendingCommand {
                    command: Command::DeployStart { service_id: sid },
                    context: CmdContext::Deploy,
                });
                app.set_notification("Iniciando deploy...", false);
            }
        }
        GeneralTabField::BtnStop => {
            if let Some(sid) = app.active_service_id.clone() {
                // Atualização otimista: mostra Stopping imediatamente.
                if let Some(svc) = app.services.iter_mut().find(|s| s.id == sid) {
                    svc.status = shared::ServiceStatus::Stopping;
                }
                app.pending_commands.push(PendingCommand {
                    command: Command::ServiceStop { service_id: sid },
                    context: CmdContext::ServiceStop,
                });
                app.set_notification("Parando serviço...", false);
            }
        }
        GeneralTabField::BtnReload => {
            if let Some(sid) = app.active_service_id.clone() {
                app.pending_commands.push(PendingCommand {
                    command: Command::ServiceReload { service_id: sid },
                    context: CmdContext::ServiceReload,
                });
                app.set_notification("Reiniciando container...", false);
            }
        }
        GeneralTabField::ProviderSave | GeneralTabField::BuildSave => {
            save_service_general(app);
        }
        _ => {}
    }
}

fn save_service_general(app: &mut App) {
    let sid = match app.active_service_id.clone() {
        Some(s) => s,
        None => return,
    };
    let svc = match app.services.iter().find(|s| s.id == sid) {
        Some(s) => s.clone(),
        None => return,
    };

    let port = app.general_tab.port.parse::<u16>().unwrap_or(svc.spec.port);

    let new_source = match &svc.spec.source {
        ServiceSource::Compose(_) => return, // compose content é salvo via Ctrl+S no editor
        ServiceSource::Git(_) => ServiceSource::Git(app.general_tab.to_git_source()),
        ServiceSource::Registry { .. } => {
            let url = &app.general_tab.repo_url;
            if shared::looks_like_git_url(url) {
                ServiceSource::Git(app.general_tab.to_git_source())
            } else {
                ServiceSource::Registry { image: url.clone() }
            }
        }
    };

    let new_spec = shared::ServiceSpec {
        source: new_source,
        port,
        ..svc.spec.clone()
    };

    app.pending_commands.push(PendingCommand {
        command: Command::ServiceUpdate {
            id: sid,
            spec: new_spec,
        },
        context: CmdContext::UpdateService,
    });
}

fn handle_healthcheck_tab(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            app.healthcheck_tab.focused = app.healthcheck_tab.focused.prev();
        }
        KeyCode::Down | KeyCode::Enter => {
            if app.healthcheck_tab.focused == HcField::Save {
                save_healthcheck(app);
            } else {
                app.healthcheck_tab.focused = app.healthcheck_tab.focused.next();
            }
        }
        KeyCode::Char(' ') => {
            if app.healthcheck_tab.focused == HcField::Kind {
                app.healthcheck_tab.cycle_kind();
            } else if app.healthcheck_tab.focused == HcField::Save {
                save_healthcheck(app);
            }
        }
        KeyCode::Char(c) => {
            if app.healthcheck_tab.focused.is_text() {
                if let Some(field) = app.healthcheck_tab.focused_text_mut() {
                    field.push(c);
                }
            }
        }
        KeyCode::Backspace => {
            if app.healthcheck_tab.focused.is_text() {
                if let Some(field) = app.healthcheck_tab.focused_text_mut() {
                    field.pop();
                }
            }
        }
        _ => {}
    }
}

fn handle_domains_tab(app: &mut App, key: KeyEvent) {
    use crate::app::DomainsField;
    match key.code {
        KeyCode::Up => {
            app.domains_tab.focused = app.domains_tab.focused.clone().prev();
        }
        KeyCode::Down | KeyCode::Enter => {
            if app.domains_tab.focused == DomainsField::Save {
                save_domains(app);
            } else {
                app.domains_tab.focused = app.domains_tab.focused.clone().next();
            }
        }
        KeyCode::Char(' ') => {
            match app.domains_tab.focused {
                DomainsField::TlsEnabled => {
                    app.domains_tab.tls_enabled = !app.domains_tab.tls_enabled;
                }
                DomainsField::Save => save_domains(app),
                _ => {
                    if let Some(field) = app.domains_tab.focused_text_mut() {
                        field.push(' ');
                    }
                }
            }
        }
        KeyCode::Char(c) => {
            if app.domains_tab.focused.clone().is_text() {
                if let Some(field) = app.domains_tab.focused_text_mut() {
                    field.push(c);
                }
            }
        }
        KeyCode::Backspace => {
            if app.domains_tab.focused.clone().is_text() {
                if let Some(field) = app.domains_tab.focused_text_mut() {
                    field.pop();
                }
            }
        }
        _ => {}
    }
}

fn save_domains(app: &mut App) {
    let sid = match app.active_service_id.clone() {
        Some(s) => s,
        None => return,
    };
    let svc = match app.services.iter().find(|s| s.id == sid) {
        Some(s) => s.clone(),
        None => return,
    };
    let domain = if app.domains_tab.domain.trim().is_empty() {
        None
    } else {
        Some(app.domains_tab.domain.trim().to_string())
    };
    let host_port = app.domains_tab.host_port.trim().parse::<u16>().ok();
    let tls_enabled = app.domains_tab.tls_enabled;
    let new_spec = shared::ServiceSpec {
        domain,
        host_port,
        tls_enabled,
        ..svc.spec.clone()
    };
    app.pending_commands.push(PendingCommand {
        command: Command::ServiceUpdate {
            id: sid,
            spec: new_spec,
        },
        context: CmdContext::UpdateService,
    });
    app.set_notification("Domínio atualizado.", false);
}

fn save_healthcheck(app: &mut App) {
    let sid = match app.active_service_id.clone() {
        Some(s) => s,
        None => return,
    };
    let svc = match app.services.iter().find(|s| s.id == sid) {
        Some(s) => s.clone(),
        None => return,
    };

    let new_spec = shared::ServiceSpec {
        healthcheck: app.healthcheck_tab.to_healthcheck(),
        ..svc.spec.clone()
    };

    app.pending_commands.push(PendingCommand {
        command: Command::ServiceUpdate {
            id: sid,
            spec: new_spec,
        },
        context: CmdContext::UpdateService,
    });
    app.set_notification("Healthcheck atualizado.", false);
}

fn handle_env_tab(app: &mut App, key: KeyEvent) {
    if app.env_tab.editing {
        match key.code {
            KeyCode::Tab => {
                app.env_tab.edit_field = match app.env_tab.edit_field {
                    EnvEditField::Key => EnvEditField::Value,
                    EnvEditField::Value => EnvEditField::Key,
                };
            }
            KeyCode::Esc => {
                app.env_tab.editing = false;
            }
            KeyCode::Enter => {
                let k = app.env_tab.edit_key.clone();
                let v = app.env_tab.edit_value.clone();
                if !k.is_empty() {
                    if let Some(sid) = app.active_service_id.clone() {
                        if let Some(svc) = app.services.iter().find(|s| s.id == sid) {
                            let mut spec = svc.spec.clone();
                            spec.env_vars.retain(|e| e.key != k);
                            spec.env_vars.push(EnvVar {
                                key: k,
                                value: if let Some(n) = v.strip_prefix("secret:") {
                                            EnvVarValue::Secret(n.to_string())
                                        } else {
                                            EnvVarValue::Plain(v)
                                        },
                            });
                            app.pending_commands.push(PendingCommand {
                                command: Command::ServiceUpdate { id: sid, spec },
                                context: CmdContext::UpdateService,
                            });
                        }
                    }
                }
                app.env_tab.editing = false;
            }
            KeyCode::Char(c) => match app.env_tab.edit_field {
                EnvEditField::Key => app.env_tab.edit_key.push(c),
                EnvEditField::Value => app.env_tab.edit_value.push(c),
            },
            KeyCode::Backspace => match app.env_tab.edit_field {
                EnvEditField::Key => {
                    app.env_tab.edit_key.pop();
                }
                EnvEditField::Value => {
                    app.env_tab.edit_value.pop();
                }
            },
            _ => {}
        }
        return;
    }

    let svc_env_len = app
        .active_service_id
        .as_deref()
        .and_then(|sid| app.services.iter().find(|s| s.id == sid))
        .map(|s| s.spec.env_vars.len())
        .unwrap_or(0);

    match key.code {
        KeyCode::Up => {
            if app.env_tab.cursor > 0 {
                app.env_tab.cursor -= 1;
            }
        }
        KeyCode::Down => {
            if app.env_tab.cursor + 1 < svc_env_len {
                app.env_tab.cursor += 1;
            }
        }
        KeyCode::Char('n') => {
            app.env_tab.edit_key = String::new();
            app.env_tab.edit_value = String::new();
            app.env_tab.edit_field = EnvEditField::Key;
            app.env_tab.editing = true;
        }
        KeyCode::Char('e') => {
            if let Some(svc) = app
                .active_service_id
                .as_deref()
                .and_then(|sid| app.services.iter().find(|s| s.id == sid))
            {
                if let Some(ev) = svc.spec.env_vars.get(app.env_tab.cursor) {
                    app.env_tab.edit_key = ev.key.clone();
                    app.env_tab.edit_value = match &ev.value {
                        EnvVarValue::Plain(v) => v.clone(),
                        EnvVarValue::Secret(s) => format!("<secret:{s}>"),
                    };
                    app.env_tab.edit_field = EnvEditField::Key;
                    app.env_tab.editing = true;
                }
            }
        }
        KeyCode::Char('D') => {
            if let Some(sid) = app.active_service_id.clone() {
                if let Some(svc) = app.services.iter().find(|s| s.id == sid) {
                    if let Some(ev) = svc.spec.env_vars.get(app.env_tab.cursor) {
                        let key = ev.key.clone();
                        let mut spec = svc.spec.clone();
                        spec.env_vars.retain(|e| e.key != key);
                        app.pending_commands.push(PendingCommand {
                            command: Command::ServiceUpdate { id: sid, spec },
                            context: CmdContext::UpdateService,
                        });
                    }
                }
            }
        }
        _ => {}
    }
}

fn request_build_logs(app: &mut App) {
    let cursor = app
        .deployment_cursor
        .min(app.service_deployments.len().saturating_sub(1));
    if let Some(dep) = app.service_deployments.get(cursor) {
        let dep_id = dep.id.clone();
        // Skip if already cached.
        if !app.build_logs.contains_key(&dep_id) {
            app.pending_commands.push(PendingCommand {
                command: Command::GetBuildLogs {
                    deployment_id: dep_id,
                },
                context: CmdContext::LoadBuildLogs,
            });
        }
    }
}

fn handle_deployments_tab(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            if app.deployment_cursor > 0 {
                app.deployment_cursor -= 1;
                app.build_log_scroll = usize::MAX; // volta ao tail no novo deployment
                request_build_logs(app);
            }
        }
        KeyCode::Down => {
            let max = app.service_deployments.len().saturating_sub(1);
            if app.deployment_cursor < max {
                app.deployment_cursor += 1;
                app.build_log_scroll = usize::MAX;
                request_build_logs(app);
            }
        }
        // Scroll no build log
        KeyCode::Char('[') => {
            app.build_log_scroll = app.build_log_scroll.saturating_sub(5);
        }
        KeyCode::Char(']') => {
            app.build_log_scroll = app.build_log_scroll.saturating_add(5);
        }
        KeyCode::Char('g') => {
            app.build_log_scroll = 0;
        }
        KeyCode::Char('G') => {
            app.build_log_scroll = usize::MAX;
        }
        KeyCode::Char('r') => {
            if let Some(sid) = app.active_service_id.clone() {
                app.pending_commands.push(PendingCommand {
                    command: Command::DeployRollback { service_id: sid },
                    context: CmdContext::Deploy,
                });
                app.set_notification("Rollback iniciado", false);
            }
        }
        KeyCode::Char('x') => {
            let cursor = app.deployment_cursor
                .min(app.service_deployments.len().saturating_sub(1));
            if let Some(dep) = app.service_deployments.get(cursor) {
                let did = dep.id.clone();
                app.pending_commands.push(PendingCommand {
                    command: Command::DeployDelete { deployment_id: did.clone() },
                    context: CmdContext::DeleteDeployment(did),
                });
            }
        }
        KeyCode::Char('c') => {
            if let Some(url) = app.webhook_url.clone() {
                copy_to_clipboard(app, &url);
            }
        }
        KeyCode::Char('w') => {
            if let Some(sid) = app.active_service_id.clone() {
                app.pending_commands.push(PendingCommand {
                    command: Command::RegenerateWebhookToken { service_id: sid },
                    context: CmdContext::RegenerateWebhook,
                });
            }
        }
        _ => {}
    }
}

fn handle_logs_tab(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            if app.log_cursor > 0 {
                app.log_cursor -= 1;
            }
        }
        KeyCode::Down => {
            if let Some(sid) = &app.active_service_id {
                let len = app.logs.get(sid).map(|l| l.len()).unwrap_or(0);
                if app.log_cursor + 1 < len {
                    app.log_cursor += 1;
                }
            }
        }
        KeyCode::Char('f') => {
            if let Some(sid) = &app.active_service_id {
                let len = app.logs.get(sid).map(|l| l.len()).unwrap_or(0);
                app.log_cursor = len.saturating_sub(1);
            }
        }
        KeyCode::Char('r') => {
            if let Some(sid) = app.active_service_id.clone() {
                app.logs.remove(&sid);
                app.log_cursor = 0;
                app.pending_commands.push(PendingCommand {
                    command: Command::LogsGet {
                        service_id: sid,
                        tail: 500,
                    },
                    context: CmdContext::LoadLogs,
                });
            }
        }
        _ => {}
    }
}

fn handle_advanced_tab(app: &mut App, key: KeyEvent) {
    // Se estiver editando um arg, captura tudo primeiro.
    if app.advanced_tab.focused == AdvancedField::RunArgs && app.advanced_tab.args_editing {
        match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                app.advanced_tab.args_editing = false;
            }
            KeyCode::Char(c) => {
                let cur = app.advanced_tab.args_cursor;
                if cur < app.advanced_tab.run_args.len() {
                    app.advanced_tab.run_args[cur].push(c);
                }
            }
            KeyCode::Backspace => {
                let cur = app.advanced_tab.args_cursor;
                if cur < app.advanced_tab.run_args.len() {
                    app.advanced_tab.run_args[cur].pop();
                }
            }
            _ => {}
        }
        return;
    }

    // Navegação e ações no contexto de RunArgs (sem edição ativa).
    if app.advanced_tab.focused == AdvancedField::RunArgs {
        match key.code {
            KeyCode::Up => {
                if app.advanced_tab.args_cursor > 0 {
                    app.advanced_tab.args_cursor -= 1;
                } else {
                    // Sai do bloco de args para o campo acima.
                    app.advanced_tab.focused = AdvancedField::RunCommand;
                }
                return;
            }
            KeyCode::Down => {
                let last = app.advanced_tab.run_args.len().saturating_sub(1);
                if app.advanced_tab.run_args.is_empty() || app.advanced_tab.args_cursor >= last {
                    // Sai do bloco de args para o Save.
                    app.advanced_tab.focused = AdvancedField::Save;
                } else {
                    app.advanced_tab.args_cursor += 1;
                }
                return;
            }
            KeyCode::Enter => {
                if !app.advanced_tab.run_args.is_empty() {
                    app.advanced_tab.args_editing = true;
                }
                return;
            }
            KeyCode::Char('a') => {
                app.advanced_tab.args_add();
                return;
            }
            KeyCode::Char('D') => {
                app.advanced_tab.args_delete();
                return;
            }
            _ => return,
        }
    }

    // Navegação global entre campos.
    match key.code {
        KeyCode::Up => {
            app.advanced_tab.focused = app.advanced_tab.focused.prev();
        }
        KeyCode::Down | KeyCode::Enter => {
            if app.advanced_tab.focused == AdvancedField::Save {
                save_advanced(app);
            } else {
                app.advanced_tab.focused = app.advanced_tab.focused.next();
                // Ao entrar em RunArgs, garante cursor válido.
                if app.advanced_tab.focused == AdvancedField::RunArgs {
                    let len = app.advanced_tab.run_args.len();
                    if app.advanced_tab.args_cursor >= len && len > 0 {
                        app.advanced_tab.args_cursor = len - 1;
                    }
                }
            }
        }
        KeyCode::Char(' ') => {
            if app.advanced_tab.focused == AdvancedField::Save {
                save_advanced(app);
            }
        }
        KeyCode::Char(c) => {
            if app.advanced_tab.focused.is_simple_text() {
                if let Some(field) = app.advanced_tab.focused_text_mut() {
                    field.push(c);
                }
            }
        }
        KeyCode::Backspace => {
            if app.advanced_tab.focused.is_simple_text() {
                if let Some(field) = app.advanced_tab.focused_text_mut() {
                    field.pop();
                }
            }
        }
        _ => {}
    }
}

fn save_advanced(app: &mut App) {
    let sid = match app.active_service_id.clone() {
        Some(s) => s,
        None => return,
    };
    let svc = match app.services.iter().find(|s| s.id == sid) {
        Some(s) => s.clone(),
        None => return,
    };
    let replicas = app.advanced_tab.replicas.parse::<u32>().unwrap_or(svc.spec.replicas).max(1);
    let run_command = if app.advanced_tab.run_command.trim().is_empty() {
        None
    } else {
        Some(app.advanced_tab.run_command.trim().to_string())
    };
    let run_args: Vec<String> = app.advanced_tab.run_args
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let new_spec = shared::ServiceSpec { replicas, run_command, run_args, ..svc.spec.clone() };
    app.pending_commands.push(PendingCommand {
        command: Command::ServiceUpdate { id: sid, spec: new_spec },
        context: CmdContext::UpdateService,
    });
    app.set_notification("Configurações avançadas salvas.", false);
}

fn handle_new_service(app: &mut App, key: KeyEvent) {
    let step = match &app.new_service {
        Some(s) => s.step.clone(),
        None => return,
    };
    match step {
        NewServiceStep::PickType => handle_ns_pick_type(app, key),
        NewServiceStep::PickDbType => handle_ns_pick_db(app, key),
        NewServiceStep::ApplicationForm
        | NewServiceStep::DatabaseForm
        | NewServiceStep::ComposeForm => handle_ns_form(app, key),
        NewServiceStep::PickTemplate => handle_ns_pick_template(app, key),
        NewServiceStep::TemplateVarForm => handle_ns_template_vars(app, key),
    }
}

fn handle_ns_pick_type(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.new_service = None;
        }
        KeyCode::Left => {
            if let Some(s) = &mut app.new_service {
                if s.type_cursor % 2 == 1 {
                    s.type_cursor -= 1;
                }
            }
        }
        KeyCode::Right => {
            if let Some(s) = &mut app.new_service {
                if s.type_cursor % 2 == 0 {
                    s.type_cursor += 1;
                }
            }
        }
        KeyCode::Up => {
            if let Some(s) = &mut app.new_service {
                if s.type_cursor >= 2 {
                    s.type_cursor -= 2;
                }
            }
        }
        KeyCode::Down => {
            if let Some(s) = &mut app.new_service {
                if s.type_cursor < 2 {
                    s.type_cursor += 2;
                }
            }
        }
        KeyCode::Enter => {
            if let Some(s) = &mut app.new_service {
                match s.type_cursor {
                    0 => {
                        s.step = NewServiceStep::ApplicationForm;
                        s.focused_field = 0;
                    }
                    1 => {
                        s.step = NewServiceStep::PickDbType;
                        s.db_cursor = 0;
                    }
                    2 => {
                        s.step = NewServiceStep::ComposeForm;
                        s.focused_field = 0;
                    }
                    _ => {
                        s.step = NewServiceStep::PickTemplate;
                        s.template_cat_cursor = 0;
                        s.template_cursor = 0;
                        s.template_search.clear();
                    }
                }
            }
        }
        _ => {}
    }
}

fn handle_ns_pick_db(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            if let Some(s) = &mut app.new_service {
                s.step = NewServiceStep::PickType;
            }
        }
        KeyCode::Up => {
            if let Some(s) = &mut app.new_service {
                if s.db_cursor > 0 {
                    s.db_cursor -= 1;
                }
            }
        }
        KeyCode::Down => {
            if let Some(s) = &mut app.new_service {
                if s.db_cursor + 1 < DbKind::ALL.len() {
                    s.db_cursor += 1;
                }
            }
        }
        KeyCode::Enter => {
            if let Some(s) = &mut app.new_service {
                s.select_db_kind();
            }
        }
        _ => {}
    }
}

fn handle_ns_form(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            if let Some(s) = &mut app.new_service {
                match s.step {
                    NewServiceStep::ApplicationForm | NewServiceStep::ComposeForm => {
                        app.new_service = None;
                    }
                    NewServiceStep::DatabaseForm => {
                        s.step = NewServiceStep::PickDbType;
                        s.focused_field = 0;
                        s.form_scroll = 0;
                    }
                    _ => {}
                }
            }
        }
        KeyCode::Up => {
            if let Some(s) = &mut app.new_service {
                s.prev_field();
            }
        }
        KeyCode::Down | KeyCode::Tab => {
            if let Some(s) = &mut app.new_service {
                s.next_field();
            }
        }
        KeyCode::Char(' ') => {
            if let Some(s) = &mut app.new_service {
                if s.is_checkbox() {
                    s.use_replica_sets = !s.use_replica_sets;
                } else if !s.is_button() {
                    if let Some(f) = s.focused_text_mut() {
                        f.push(' ');
                    }
                }
            }
        }
        KeyCode::Char(c) => {
            if let Some(s) = &mut app.new_service {
                if !s.is_button() && !s.is_checkbox() {
                    if let Some(f) = s.focused_text_mut() {
                        f.push(c);
                    }
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(s) = &mut app.new_service {
                if let Some(f) = s.focused_text_mut() {
                    f.pop();
                }
            }
        }
        KeyCode::Enter => {
            let is_btn = app
                .new_service
                .as_ref()
                .map(|s| s.is_button())
                .unwrap_or(false);
            if is_btn {
                let spec = app.new_service.as_ref().unwrap().to_service_spec();
                if spec.name.is_empty() {
                    app.set_notification("Nome é obrigatório", true);
                    return;
                }
                app.pending_commands.push(PendingCommand {
                    command: Command::ServiceCreate(spec),
                    context: CmdContext::CreateService,
                });
                app.new_service = None;
            } else if let Some(s) = &mut app.new_service {
                s.next_field();
            }
        }
        _ => {}
    }
}

fn handle_ns_pick_template(app: &mut App, key: KeyEvent) {
    use shared::templates::{self, TemplateCategory};

    let s = match app.new_service.as_mut() {
        Some(s) => s,
        None => return,
    };

    if s.template_searching {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                s.template_searching = false;
                s.template_cursor = 0;
            }
            KeyCode::Char(c) => {
                s.template_search.push(c);
                s.template_cursor = 0;
            }
            KeyCode::Backspace => {
                s.template_search.pop();
                s.template_cursor = 0;
            }
            _ => {}
        }
        return;
    }

    match key.code {
        KeyCode::Esc => {
            s.step = NewServiceStep::PickType;
            s.template_search.clear();
            s.template_cursor = 0;
        }
        KeyCode::Left => {
            if s.template_cat_cursor > 0 {
                s.template_cat_cursor -= 1;
                s.template_cursor = 0;
            }
        }
        KeyCode::Right => {
            if s.template_cat_cursor + 1 < TemplateCategory::FILTERS.len() {
                s.template_cat_cursor += 1;
                s.template_cursor = 0;
            }
        }
        KeyCode::Up => {
            if s.template_cursor > 0 {
                s.template_cursor -= 1;
            }
        }
        KeyCode::Down => {
            let cat = TemplateCategory::FILTERS[s.template_cat_cursor];
            let count = templates::filtered(cat, &s.template_search.clone()).len();
            if s.template_cursor + 1 < count {
                s.template_cursor += 1;
            }
        }
        KeyCode::Char('/') => {
            s.template_searching = true;
            s.template_search.clear();
        }
        KeyCode::Enter => {
            let cat = TemplateCategory::FILTERS[s.template_cat_cursor];
            let search = s.template_search.clone();
            let list = templates::filtered(cat, &search);
            if let Some(&t) = list.get(s.template_cursor) {
                let s2 = app.new_service.as_mut().unwrap();
                s2.select_template(t);
            }
        }
        _ => {}
    }
}

fn handle_ns_template_vars(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            if let Some(s) = app.new_service.as_mut() {
                s.step = NewServiceStep::PickTemplate;
                s.focused_field = 0;
            }
        }
        KeyCode::Up => {
            if let Some(s) = app.new_service.as_mut() {
                s.prev_field();
            }
        }
        KeyCode::Down | KeyCode::Tab => {
            if let Some(s) = app.new_service.as_mut() {
                s.next_field();
            }
        }
        KeyCode::Char(c) => {
            if let Some(s) = app.new_service.as_mut() {
                if !s.is_button() {
                    if let Some(f) = s.focused_text_mut() {
                        f.push(c);
                    }
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(s) = app.new_service.as_mut() {
                if let Some(f) = s.focused_text_mut() {
                    f.pop();
                }
            }
        }
        KeyCode::Enter => {
            let is_btn = app
                .new_service
                .as_ref()
                .map(|s| s.is_button())
                .unwrap_or(false);
            if is_btn {
                let spec = app.new_service.as_ref().unwrap().to_service_spec();
                if spec.name.is_empty() {
                    app.set_notification("Nome é obrigatório", true);
                    return;
                }
                app.pending_commands.push(PendingCommand {
                    command: Command::ServiceCreate(spec),
                    context: CmdContext::CreateService,
                });
                app.new_service = None;
            } else if let Some(s) = app.new_service.as_mut() {
                s.next_field();
            }
        }
        _ => {}
    }
}

fn handle_new_project(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.creating_project = false;
        }
        KeyCode::Tab => {
            app.new_proj_field = (app.new_proj_field + 1) % 2;
        }
        KeyCode::Enter => {
            if app.new_proj_name.is_empty() {
                app.set_notification("Nome do projeto é obrigatório", true);
                return;
            }
            let name = app.new_proj_name.clone();
            let desc = if app.new_proj_desc.is_empty() {
                None
            } else {
                Some(app.new_proj_desc.clone())
            };
            app.pending_commands.push(PendingCommand {
                command: Command::ProjectCreate {
                    name,
                    description: desc,
                },
                context: CmdContext::CreateProject,
            });
            app.creating_project = false;
        }
        KeyCode::Char(c) => {
            if app.new_proj_field == 0 {
                app.new_proj_name.push(c);
            } else {
                app.new_proj_desc.push(c);
            }
        }
        KeyCode::Backspace => {
            if app.new_proj_field == 0 {
                app.new_proj_name.pop();
            } else {
                app.new_proj_desc.pop();
            }
        }
        _ => {}
    }
}

fn handle_confirm(app: &mut App, key: KeyEvent) {
    let View::Confirm { action, .. } = app.view.clone() else {
        return;
    };

    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            match action {
                ConfirmAction::DeleteProject(id) => {
                    app.pending_commands.push(PendingCommand {
                        command: Command::ProjectDelete { id: id.clone() },
                        context: CmdContext::DeleteProject(id),
                    });
                }
                ConfirmAction::DeleteService(id) => {
                    app.pending_commands.push(PendingCommand {
                        command: Command::ServiceDelete { id: id.clone() },
                        context: CmdContext::DeleteService(id),
                    });
                }
                ConfirmAction::AbortDeploy(id) => {
                    app.pending_commands.push(PendingCommand {
                        command: Command::DeployAbort { deployment_id: id },
                        context: CmdContext::None,
                    });
                }
            }
            let back = if app.active_project_id.is_some() {
                View::ProjectDetail
            } else {
                View::Projects
            };
            app.view = back;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            let back = if app.active_project_id.is_some() {
                View::ProjectDetail
            } else {
                View::Projects
            };
            app.view = back;
        }
        _ => {}
    }
}

fn handle_settings_web_server(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            app.server_settings.focused = app.server_settings.focused.clone().prev();
        }
        KeyCode::Down | KeyCode::Enter => {
            if app.server_settings.focused == ServerSettingsField::Save {
                save_server_settings(app);
            } else {
                app.server_settings.focused = app.server_settings.focused.clone().next();
            }
        }
        KeyCode::Char(' ') => {
            if app.server_settings.focused == ServerSettingsField::Save {
                save_server_settings(app);
            }
        }
        KeyCode::Char(c) => match app.server_settings.focused {
            ServerSettingsField::ServerDomain => app.server_settings.server_domain.push(c),
            ServerSettingsField::AcmeEmail => app.server_settings.acme_email.push(c),
            _ => {}
        },
        KeyCode::Backspace => match app.server_settings.focused {
            ServerSettingsField::ServerDomain => { app.server_settings.server_domain.pop(); }
            ServerSettingsField::AcmeEmail => { app.server_settings.acme_email.pop(); }
            _ => {}
        },
        _ => {}
    }
}

fn save_server_settings(app: &mut App) {
    let domain = app.server_settings.server_domain.trim().to_string();
    let webhook_base_url = if domain.is_empty() { None } else { Some(domain) };
    let email = app.server_settings.acme_email.trim().to_string();
    let acme_email = if email.is_empty() { None } else { Some(email) };
    app.pending_commands.push(PendingCommand {
        command: Command::SetDaemonSettings { webhook_base_url, acme_email },
        context: CmdContext::SaveServerSettings,
    });
}

fn copy_to_clipboard(app: &mut App, text: &str) {
    // wl-copy (Wayland) — fica em background como dono do clipboard
    if std::process::Command::new("wl-copy")
        .arg("--")
        .arg(text)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
    {
        app.set_notification("URL copiada para a área de transferência", false);
        return;
    }

    // xclip (X11)
    if let Ok(mut child) = std::process::Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        app.set_notification("URL copiada para a área de transferência", false);
        return;
    }

    // xsel (X11 alternativo)
    if let Ok(mut child) = std::process::Command::new("xsel")
        .args(["--clipboard", "--input"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        app.set_notification("URL copiada para a área de transferência", false);
        return;
    }

    app.set_notification(
        "Instale wl-copy (Wayland) ou xclip/xsel (X11) para copiar",
        true,
    );
}

fn handle_docker_cleanup(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            app.docker_prune.focused = app.docker_prune.focused.prev();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.docker_prune.focused = app.docker_prune.focused.next();
        }
        KeyCode::Enter => {
            let (cmd, ctx, slot) = match app.docker_prune.focused {
                DockerPruneButton::Containers => (
                    Command::PruneContainers,
                    CmdContext::PruneContainers,
                    &mut app.docker_prune.containers,
                ),
                DockerPruneButton::Volumes => (
                    Command::PruneVolumes { all: false },
                    CmdContext::PruneVolumes,
                    &mut app.docker_prune.volumes,
                ),
                DockerPruneButton::Images => (
                    Command::PruneImages { all: false },
                    CmdContext::PruneImages,
                    &mut app.docker_prune.images,
                ),
                DockerPruneButton::BuildCache => (
                    Command::PruneBuildCache,
                    CmdContext::PruneBuildCache,
                    &mut app.docker_prune.build_cache,
                ),
            };
            if *slot == PruneSlot::Running {
                return;
            }
            *slot = PruneSlot::Running;
            app.pending_commands.push(PendingCommand { command: cmd, context: ctx });
        }
        _ => {}
    }
}
