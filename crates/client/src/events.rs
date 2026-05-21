use crate::app::{
    App, CmdContext, ConfirmAction, EnvEditField, Focus, GeneralTabField, PendingCommand,
    ServiceFormField, View,
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use shared::{Command, EnvVar, EnvVarValue};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    if app.creating_project {
        handle_new_project(app, key);
        return;
    }

    if let View::Confirm { .. } = &app.view.clone() {
        handle_confirm(app, key);
        return;
    }

    match key.code {
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::Sidebar => Focus::Content,
                Focus::Content => Focus::Sidebar,
            };
            return;
        }
        KeyCode::Esc => {
            match &app.view {
                View::ServiceDetail => {
                    app.view = View::ProjectDetail;
                    app.active_service_id = None;
                }
                View::ServiceForm => {
                    app.service_form = None;
                    app.view = View::ProjectDetail;
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
        KeyCode::Char('n') => {
            if let Some(crate::app::SidebarItem::Project(_)) = app.current_sidebar_item() {
                app.sidebar_select();
            }
        }
        _ => {}
    }
}

fn handle_content(app: &mut App, key: KeyEvent) {
    match app.view.clone() {
        View::ProjectDetail => handle_project_detail(app, key),
        View::ServiceDetail => handle_service_detail(app, key),
        View::ServiceForm => handle_service_form(app, key),
        View::HomeMonitoring
        | View::HomeDeployments
        | View::HomeSchedules
        | View::HomePingoraFs
        | View::HomeDocker
        | View::HomeDeployEngine
        | View::HomeRequests => handle_home(app, key),
        _ => {}
    }
}

fn handle_home(app: &mut App, _key: KeyEvent) {}

fn handle_project_detail(app: &mut App, key: KeyEvent) {
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
                app.service_form = Some(crate::app::ServiceFormState::new(pid));
                app.view = View::ServiceForm;
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

fn handle_service_detail(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Left => {
            app.service_tab = app.service_tab.prev();
            if app.service_tab == crate::app::ServiceTab::General {
                if let Some(svc) = app.current_active_service() {
                    app.general_tab = crate::app::GeneralTabState::from_service(svc);
                }
            }
        }
        KeyCode::Right => {
            app.service_tab = app.service_tab.next();
            if app.service_tab == crate::app::ServiceTab::General {
                if let Some(svc) = app.current_active_service() {
                    app.general_tab = crate::app::GeneralTabState::from_service(svc);
                }
            }
        }
        KeyCode::Char('1') => app.service_tab = crate::app::ServiceTab::General,
        KeyCode::Char('2') => app.service_tab = crate::app::ServiceTab::Environment,
        KeyCode::Char('3') => app.service_tab = crate::app::ServiceTab::Domains,
        KeyCode::Char('4') => app.service_tab = crate::app::ServiceTab::Deployments,
        KeyCode::Char('5') => app.service_tab = crate::app::ServiceTab::Logs,
        KeyCode::Char('6') => app.service_tab = crate::app::ServiceTab::Patches,
        _ => match app.service_tab.clone() {
            crate::app::ServiceTab::General => handle_general_tab(app, key),
            crate::app::ServiceTab::Environment => handle_env_tab(app, key),
            crate::app::ServiceTab::Deployments => handle_deployments_tab(app, key),
            crate::app::ServiceTab::Logs => handle_logs_tab(app, key),
            _ => {}
        },
    }
}

fn handle_general_tab(app: &mut App, key: KeyEvent) {
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
            app.set_notification("Stop não implementado via API ainda", true);
        }
        GeneralTabField::BtnReload => {
            app.set_notification("Reload não implementado via API ainda", true);
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

    let new_git = {
        let existing = match &svc.spec.source {
            shared::ServiceSource::Git(g) => g.clone(),
            shared::ServiceSource::Registry { .. } => shared::GitSource::default(),
        };
        app.general_tab.to_git_source(&existing)
    };

    let new_spec = shared::ServiceSpec {
        source: shared::ServiceSource::Git(new_git),
        ..svc.spec.clone()
    };

    app.pending_commands.push(PendingCommand {
        command: Command::ServiceUpdate { id: sid, spec: new_spec },
        context: CmdContext::UpdateService,
    });
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
                            spec.env_vars.push(EnvVar { key: k, value: EnvVarValue::Plain(v) });
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

fn handle_deployments_tab(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Up => {
            if app.deployment_cursor > 0 {
                app.deployment_cursor -= 1;
            }
        }
        KeyCode::Down => {
            app.deployment_cursor += 1;
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
        _ => {}
    }
}

fn handle_service_form(app: &mut App, key: KeyEvent) {
    let form = match &mut app.service_form {
        Some(f) => f,
        None => return,
    };

    match key.code {
        KeyCode::Up => {
            form.focused_field = form.focused_field.prev();
        }
        KeyCode::Down => {
            form.focused_field = form.focused_field.next();
        }
        KeyCode::Char(c) => {
            if c == ' ' && form.focused_field == ServiceFormField::Submodules {
                form.submodules = !form.submodules;
            } else if form.focused_field.is_text_field() {
                if let Some(field) = form.focused_text_mut() {
                    field.push(c);
                }
            }
        }
        KeyCode::Backspace => {
            if form.focused_field.is_text_field() {
                if let Some(field) = form.focused_text_mut() {
                    field.pop();
                }
            }
        }
        KeyCode::Enter => match form.focused_field {
            ServiceFormField::BtnCancel => {
                app.service_form = None;
                app.view = View::ProjectDetail;
            }
            ServiceFormField::BtnCreate => {
                let spec = app.service_form.as_ref().unwrap().to_spec();
                app.pending_commands.push(PendingCommand {
                    command: Command::ServiceCreate(spec),
                    context: CmdContext::CreateService,
                });
                app.service_form = None;
            }
            _ => {
                if let Some(form) = &mut app.service_form {
                    form.focused_field = form.focused_field.next();
                }
            }
        },
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
            let desc = if app.new_proj_desc.is_empty() { None } else { Some(app.new_proj_desc.clone()) };
            app.pending_commands.push(PendingCommand {
                command: Command::ProjectCreate { name, description: desc },
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
    let View::Confirm { action, .. } = app.view.clone() else { return };

    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            match action {
                ConfirmAction::DeleteProject(id) => {
                    app.pending_commands.push(PendingCommand {
                        command: Command::ProjectDelete { id },
                        context: CmdContext::DeleteProject,
                    });
                }
                ConfirmAction::DeleteService(id) => {
                    app.pending_commands.push(PendingCommand {
                        command: Command::ServiceDelete { id },
                        context: CmdContext::DeleteService,
                    });
                }
                ConfirmAction::AbortDeploy(id) => {
                    app.pending_commands.push(PendingCommand {
                        command: Command::DeployAbort { deployment_id: id },
                        context: CmdContext::None,
                    });
                }
            }
            app.view = View::ProjectDetail;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.view = View::ProjectDetail;
        }
        _ => {}
    }
}
