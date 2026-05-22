use crate::app::{App, Focus};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use shared::ServiceStatus;

pub fn render_project_detail(f: &mut Frame, app: &App, area: Rect) {
    let project_name = app
        .current_project()
        .map(|p| p.name.as_str())
        .unwrap_or("Projeto");

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filter_display = if app.service_filtering {
        format!(" Filtro: {}▌", app.service_filter)
    } else if !app.service_filter.is_empty() {
        format!(" Filtro: {} ", app.service_filter)
    } else {
        String::new()
    };

    let header_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let header_text = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {project_name}"),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(filter_display, Style::default().fg(Color::Yellow)),
    ]))
    .block(header_block);
    f.render_widget(header_text, chunks[0]);

    let focused = app.focus == Focus::Content;
    let filtered = app.filtered_services();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, svc)| {
            let selected = focused && i == app.service_cursor;
            let status_color = match &svc.status {
                ServiceStatus::Running => Color::Green,
                ServiceStatus::Stopped => Color::DarkGray,
                ServiceStatus::Deploying => Color::Yellow,
                ServiceStatus::Degraded => Color::Magenta,
                ServiceStatus::Error(_) => Color::Red,
            };
            let status_label = match &svc.status {
                ServiceStatus::Running => "RUNNING",
                ServiceStatus::Stopped => "STOPPED",
                ServiceStatus::Deploying => "DEPLOYING",
                ServiceStatus::Degraded => "DEGRADED",
                ServiceStatus::Error(_) => "ERROR",
            };

            let metrics_str = app
                .metrics
                .get(&svc.id)
                .and_then(|m| m.back())
                .map(|m| {
                    format!(
                        "  ↑{:.0}M {:.0}%",
                        m.mem_used_bytes as f64 / 1_048_576.0,
                        m.cpu_percent
                    )
                })
                .unwrap_or_default();

            let name_style = if selected {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {:<28}", svc.spec.name), name_style),
                Span::styled(
                    format!("[{:<10}]", status_label),
                    Style::default().fg(status_color),
                ),
                Span::styled(metrics_str, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default());

    let list = List::new(items).block(list_block).highlight_style(
        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    if focused {
        state.select(Some(app.service_cursor));
    }
    f.render_stateful_widget(list, chunks[1], &mut state);

    if filtered.is_empty() {
        let msg = if app.service_filter.is_empty() {
            "Nenhum serviço. Pressione [n] para criar."
        } else {
            "Nenhum serviço corresponde ao filtro."
        };
        let p = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(p, chunks[1]);
    }
}

