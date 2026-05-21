use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use shared::ServiceStatus;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(8), Constraint::Length(1)])
        .split(area);

    render_title(f, chunks[0]);

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    render_projects(f, app, body_chunks[0]);
    render_services(f, app, body_chunks[1]);
    render_last_deploy(f, app, chunks[2]);
    render_help(f, chunks[3]);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new(format!(" Rustploy v{}", env!("CARGO_PKG_VERSION")))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(title, area);
}

fn render_projects(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let prefix = if i == app.selected_project { "► " } else { "  " };
            ListItem::new(format!("{prefix}{}", p.name))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" PROJETOS "))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let mut state = ListState::default();
    state.select(Some(app.selected_project));
    f.render_stateful_widget(list, area, &mut state);
}

fn render_services(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .services
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let prefix = if i == app.selected_service { "► " } else { "  " };
            let status_color = status_color(&s.status);
            let status_label = status_label(&s.status);

            let metrics_line = app
                .metrics
                .get(&s.id)
                .and_then(|m| m.back())
                .map(|m| {
                    format!(
                        " ↑{:.0}M {:.0}%",
                        m.mem_used_bytes as f64 / 1_048_576.0,
                        m.cpu_percent
                    )
                })
                .unwrap_or_default();

            ListItem::new(Line::from(vec![
                Span::raw(format!("{prefix}{:<24}", s.spec.name)),
                Span::styled(format!("[{status_label:<10}]"), Style::default().fg(status_color)),
                Span::styled(metrics_line, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(" SERVIÇOS "))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let mut state = ListState::default();
    state.select(Some(app.selected_service));
    f.render_stateful_widget(list, area, &mut state);
}

fn render_last_deploy(f: &mut Frame, app: &App, area: Rect) {
    let content = if let Some(dep) = &app.last_deployment {
        let states: Vec<&str> = dep.states_log.iter().map(|t| t.to.label()).collect();
        let chain = states.join(" → ");
        let duration = dep
            .finished_at
            .map(|f| (f - dep.started_at).num_seconds())
            .map(|s| format!("{s}s"))
            .unwrap_or_else(|| "em andamento".into());
        vec![
            Line::from(format!(" ÚLTIMO DEPLOY: {}", dep.service_id)),
            Line::from(format!("  {chain}")),
            Line::from(format!("  Duração: {duration}")),
        ]
    } else {
        vec![Line::from(" Nenhum deploy recente")]
    };

    let block = Block::default().borders(Borders::ALL).title(" STATUS ");
    let paragraph = Paragraph::new(content).block(block);
    f.render_widget(paragraph, area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let help = Paragraph::new(
        " [d]eploy  [l]ogs  [m]étricas  [r]ollback  [↑↓] navegar  [Tab] alterna painel  [q]uit",
    )
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, area);
}

fn status_color(status: &ServiceStatus) -> Color {
    match status {
        ServiceStatus::Running => Color::Green,
        ServiceStatus::Stopped => Color::DarkGray,
        ServiceStatus::Deploying => Color::Yellow,
        ServiceStatus::Degraded => Color::Magenta,
        ServiceStatus::Error(_) => Color::Red,
    }
}

fn status_label(status: &ServiceStatus) -> &str {
    match status {
        ServiceStatus::Running => "RUNNING",
        ServiceStatus::Stopped => "STOPPED",
        ServiceStatus::Deploying => "DEPLOYING",
        ServiceStatus::Degraded => "DEGRADED",
        ServiceStatus::Error(_) => "ERROR",
    }
}
