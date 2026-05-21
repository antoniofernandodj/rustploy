use crate::{app::{App, ConfirmAction, Screen}, transport::DaemonClient};
use crossterm::event::{Event as TermEvent, KeyCode, KeyEventKind, KeyModifiers};

pub enum AppEvent {
    Term(TermEvent),
    Daemon(shared::Event),
    Tick,
}

pub fn handle_key(app: &mut App, _client: &DaemonClient, key: crossterm::event::KeyEvent) {
    if key.kind != KeyEventKind::Press {
        return;
    }

    // Global keys
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            if matches!(app.screen, Screen::Dashboard) {
                std::process::exit(0);
            } else {
                app.screen = Screen::Dashboard;
            }
            return;
        }
        KeyCode::Esc => {
            app.screen = Screen::Dashboard;
            return;
        }
        _ => {}
    }

    match app.screen.clone() {
        Screen::Dashboard => handle_dashboard(app, key),
        Screen::ServiceDetail => handle_service_detail(app, key),
        Screen::Confirm { action, .. } => handle_confirm(app, key, action),
        Screen::DeployProgress(_) => {
            if let KeyCode::Char('a') = key.code {
                app.set_notification("Aborting deploy...", false);
            }
        }
        _ => {}
    }
}

fn handle_dashboard(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Up => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                if app.selected_project > 0 {
                    app.selected_project -= 1;
                }
            } else if app.selected_service > 0 {
                app.selected_service -= 1;
            }
        }
        KeyCode::Down => {
            if key.modifiers.contains(KeyModifiers::SHIFT) {
                if app.selected_project + 1 < app.projects.len() {
                    app.selected_project += 1;
                }
            } else if app.selected_service + 1 < app.services.len() {
                app.selected_service += 1;
            }
        }
        KeyCode::Enter => {
            if app.current_service().is_some() {
                app.screen = Screen::ServiceDetail;
            }
        }
        KeyCode::Char('d') => {
            if let Some(svc) = app.current_service() {
                let _svc_id = svc.id.clone();
                // Trigger deploy (fire and forget in event loop)
                app.set_notification(format!("Iniciando deploy de {}", svc.spec.name), false);
            }
        }
        KeyCode::Char('l') => {
            if let Some(svc) = app.current_service() {
                app.screen = Screen::Logs(svc.id.clone());
            }
        }
        KeyCode::Char('m') => {
            if let Some(svc) = app.current_service() {
                app.screen = Screen::Metrics(svc.id.clone());
            }
        }
        _ => {}
    }
}

fn handle_service_detail(app: &mut App, key: crossterm::event::KeyEvent) {
    match key.code {
        KeyCode::Char('d') => {
            if let Some(svc) = app.current_service() {
                app.set_notification(format!("Iniciando deploy de {}", svc.spec.name), false);
            }
        }
        KeyCode::Char('l') => {
            if let Some(svc) = app.current_service() {
                app.screen = Screen::Logs(svc.id.clone());
            }
        }
        KeyCode::Char('m') => {
            if let Some(svc) = app.current_service() {
                app.screen = Screen::Metrics(svc.id.clone());
            }
        }
        KeyCode::Char('r') => {
            if let Some(svc) = app.current_service() {
                app.screen = Screen::Confirm {
                    message: format!("Rollback do serviço {}?", svc.spec.name),
                    action: ConfirmAction::DeleteService(svc.id.clone()),
                };
            }
        }
        _ => {}
    }
}

fn handle_confirm(app: &mut App, key: crossterm::event::KeyEvent, action: ConfirmAction) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            match action {
                ConfirmAction::DeleteProject(_id) => {
                    app.set_notification("Deletando projeto...", false);
                }
                ConfirmAction::DeleteService(_id) => {
                    app.set_notification("Deletando serviço...", false);
                }
                ConfirmAction::AbortDeploy(_id) => {
                    app.set_notification("Abortando deploy...", false);
                }
            }
            app.screen = Screen::Dashboard;
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.screen = Screen::Dashboard;
        }
        _ => {}
    }
}
