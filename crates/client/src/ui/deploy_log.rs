use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Frame,
};

pub fn render_progress(f: &mut Frame, app: &App, area: Rect, deployment_id: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let prog = app.deploy_progress.get(deployment_id);

    let state_label = prog
        .map(|p| p.current_state.label())
        .unwrap_or("Unknown");
    let percent = prog.map(|p| p.percent).unwrap_or(0);
    let _description = prog.map(|p| p.description.as_str()).unwrap_or("");

    let title = Paragraph::new(format!(" Deploy em andamento — {state_label} "))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).title(" DEPLOY "));
    f.render_widget(title, chunks[0]);

    let gauge = Gauge::default()
        .block(Block::default().borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Green))
        .percent(percent as u16)
        .label(format!("{percent}%  {state_label}"));
    f.render_widget(gauge, chunks[1]);

    let events: Vec<ListItem> = prog
        .map(|p| {
            p.states_seen
                .iter()
                .map(|s| {
                    ListItem::new(Line::from(vec![
                        Span::styled("✓ ", Style::default().fg(Color::Green)),
                        Span::raw(s.label()),
                    ]))
                })
                .collect()
        })
        .unwrap_or_default();

    let event_list = List::new(events)
        .block(Block::default().borders(Borders::ALL).title(" Eventos "));
    f.render_widget(event_list, chunks[2]);

    let help = Paragraph::new(" [a]bortar  [q]uit / [Esc] voltar")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[3]);
}

pub fn render_logs(f: &mut Frame, app: &App, area: Rect, service_id: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let svc_name = app
        .services
        .iter()
        .find(|s| s.id == service_id)
        .map(|s| s.spec.name.as_str())
        .unwrap_or(service_id);

    let title_line = Paragraph::new(format!(" Logs: {svc_name}"))
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    f.render_widget(title_line, chunks[0]);

    let log_items: Vec<ListItem> = app
        .logs
        .get(service_id)
        .into_iter()
        .flatten()
        .map(|line| {
            let ts = line.timestamp.format("%H:%M:%S%.3f");
            let color = if line.is_stderr { Color::Red } else { Color::White };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{ts} "), Style::default().fg(Color::DarkGray)),
                Span::styled(line.text.clone(), Style::default().fg(color)),
            ]))
        })
        .collect();

    let log_widget = List::new(log_items)
        .block(Block::default().borders(Borders::ALL).title(" Logs (streaming...) "));
    f.render_widget(log_widget, chunks[1]);

    let help = Paragraph::new(" [q]uit / [Esc] voltar")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[2]);
}
