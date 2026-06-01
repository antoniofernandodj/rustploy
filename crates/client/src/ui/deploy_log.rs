use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

/// Standalone deploy progress view (accessible via Home > Deployments).
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
    let state_label = prog.map(|p| p.current_state.label()).unwrap_or("Unknown");
    let percent = prog.map(|p| p.percent).unwrap_or(0);

    let title = Paragraph::new(format!(" Deploy em andamento — {state_label} "))
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
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

    let event_list =
        List::new(events).block(Block::default().borders(Borders::ALL).title(" Eventos "));
    f.render_widget(event_list, chunks[2]);

    let help = Paragraph::new(" [Esc] voltar").style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[3]);
}
