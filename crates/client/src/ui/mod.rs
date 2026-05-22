pub mod deploy_log;
pub mod metrics;
pub mod projects;
pub mod service_detail;
pub mod settings;
pub mod sidebar;

use crate::app::{App, Focus, View};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

const SIDEBAR_WIDTH: u16 = 26;

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    render_titlebar(f, main_chunks[0], app);
    render_body(f, main_chunks[1], app);
    render_statusbar(f, main_chunks[2], app);

    if app.creating_project {
        render_new_project_popup(f, area, app);
    }

    if let Some(notif) = &app.notification {
        render_notification(f, area, &notif.message, notif.is_error);
    }
}

fn render_titlebar(f: &mut Frame, area: Rect, _app: &App) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" Rustploy v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw("  "),
        Span::styled("PaaS Engine", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(title, area);
}

fn render_body(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(0)])
        .split(area);

    sidebar::render(f, app, chunks[0]);
    render_content(f, app, chunks[1]);
}

fn render_content(f: &mut Frame, app: &App, area: Rect) {
    match &app.view {
        View::ProjectDetail => projects::render_project_detail(f, app, area),
        View::ServiceDetail => service_detail::render(f, app, area),
        View::ServiceForm => projects::render_service_form(f, app, area),
        View::HomeDeployments => render_home_placeholder(f, area, "Deployments", "Ver todos os deploys ativos em todos os projetos."),
        View::HomeMonitoring => metrics::render_global(f, app, area),
        View::HomeSchedules => render_home_placeholder(f, area, "Schedules", "Agendamentos de auto-deploy (v2)."),
        View::HomePingoraFs => render_home_placeholder(f, area, "Pingora File System", "Tabela de rotas ativa no Pingora."),
        View::HomeDocker => render_home_placeholder(f, area, "Docker", "Containers, redes e imagens gerenciadas."),
        View::HomeDeployEngine => render_home_placeholder(f, area, "Deploy Engine", "Estado interno do motor de deploy."),
        View::HomeRequests => render_home_placeholder(f, area, "Requests", "Log de requisições recebidas pelo Pingora."),
        View::SettingsWebServer
        | View::SettingsProfile
        | View::SettingsUsers
        | View::SettingsAuditLogs
        | View::SettingsSshKeys
        | View::SettingsTags
        | View::SettingsGit
        | View::SettingsRegistry
        | View::SettingsS3
        | View::SettingsCerts
        | View::SettingsSso => settings::render(f, app, area),
        View::Account => settings::render_account(f, app, area),
        View::Confirm { message, .. } => render_confirm_overlay(f, area, message),
    }
}

fn render_home_placeholder(f: &mut Frame, area: Rect, title: &str, desc: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .border_style(Style::default().fg(Color::DarkGray));
    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(desc, Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(Span::styled("Em construção.", Style::default().fg(Color::Yellow))),
    ])
    .block(block);
    f.render_widget(text, area);
}

fn render_statusbar(f: &mut Frame, area: Rect, app: &App) {
    let hints = match (&app.focus, &app.view) {
        (Focus::Sidebar, _) => " [Tab] conteúdo  [↑↓] nav  [Enter] abrir  [q] quit",
        (Focus::Content, View::ProjectDetail) => {
            if app.service_filtering {
                " [Enter/Esc] sair do filtro  [Backspace] apagar"
            } else {
                " [/] filtrar  [n] novo  [Enter] abrir  [D] deletar  [Tab] sidebar"
            }
        }
        (Focus::Content, View::ServiceDetail) => {
            " [←→] abas  [1-6] aba direta  [↑↓] nav campo  [Esc] voltar  [Tab] sidebar"
        }
        (Focus::Content, View::ServiceForm) => {
            " [↑↓] nav campo  [Enter] próximo/confirmar  [Esc] cancelar"
        }
        _ => " [Tab] sidebar  [Esc] voltar",
    };

    let bar = Paragraph::new(hints).style(Style::default().fg(Color::DarkGray));
    f.render_widget(bar, area);
}

fn render_new_project_popup(f: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect_h(56, 14, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Novo Projeto ")
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // padding
            Constraint::Length(1), // label Nome
            Constraint::Length(3), // input Nome
            Constraint::Length(1), // label Descrição
            Constraint::Length(3), // input Descrição
            Constraint::Length(1), // padding
            Constraint::Length(1), // hints
        ])
        .split(inner);

    // Name
    let name_focused = app.new_proj_field == 0;
    f.render_widget(
        Paragraph::new(Span::styled(
            "  Nome",
            Style::default().fg(if name_focused { Color::Cyan } else { Color::DarkGray }),
        )),
        chunks[1],
    );
    let name_box = Block::default()
        .borders(Borders::ALL)
        .border_style(if name_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });
    let name_inner = name_box.inner(chunks[2]);
    f.render_widget(name_box, chunks[2]);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                if name_focused {
                    format!("{}▌", app.new_proj_name)
                } else {
                    app.new_proj_name.clone()
                },
                Style::default().fg(Color::White),
            ),
        ])),
        name_inner,
    );

    // Description
    let desc_focused = app.new_proj_field == 1;
    f.render_widget(
        Paragraph::new(Span::styled(
            "  Descrição  (opcional)",
            Style::default().fg(if desc_focused { Color::Cyan } else { Color::DarkGray }),
        )),
        chunks[3],
    );
    let desc_box = Block::default()
        .borders(Borders::ALL)
        .border_style(if desc_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });
    let desc_inner = desc_box.inner(chunks[4]);
    f.render_widget(desc_box, chunks[4]);
    let desc_content = if desc_focused {
        format!(" {}▌", app.new_proj_desc)
    } else if app.new_proj_desc.is_empty() {
        " opcional...".to_string()
    } else {
        format!(" {}", app.new_proj_desc)
    };
    f.render_widget(
        Paragraph::new(Span::styled(
            desc_content,
            if desc_focused {
                Style::default().fg(Color::White)
            } else if app.new_proj_desc.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            },
        )),
        desc_inner,
    );

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Tab", Style::default().fg(Color::Cyan)),
            Span::styled(" alternar  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::styled(" criar  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::styled(" cancelar", Style::default().fg(Color::DarkGray)),
        ])),
        chunks[6],
    );
}

fn render_confirm_overlay(f: &mut Frame, area: Rect, message: &str) {
    let popup = centered_rect(60, 20, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Confirmar ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(message),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [y] Sim  ", Style::default().fg(Color::Green)),
            Span::styled("[n] Não", Style::default().fg(Color::Red)),
        ]),
    ])
    .block(block);
    f.render_widget(text, popup);
}

fn render_notification(f: &mut Frame, area: Rect, message: &str, is_error: bool) {
    let width = (message.len() as u16 + 4).min(area.width.saturating_sub(2));
    let notif_area = Rect {
        x: area.width.saturating_sub(width + 1),
        y: area.height.saturating_sub(3),
        width,
        height: 3,
    };
    let color = if is_error { Color::Red } else { Color::Green };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color));
    let text = Paragraph::new(message).block(block).style(Style::default().fg(color));
    f.render_widget(Clear, notif_area);
    f.render_widget(text, notif_area);
}

/// Centers a popup with a fixed height and percentage width.
fn centered_rect_h(percent_x: u16, height: u16, r: Rect) -> Rect {
    let y_offset = r.height.saturating_sub(height) / 2;
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(y_offset),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
