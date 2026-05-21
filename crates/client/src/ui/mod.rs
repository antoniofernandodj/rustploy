pub mod dashboard;
pub mod deploy_log;
pub mod metrics;
pub mod service_detail;

use crate::app::{App, Screen};
use ratatui::{Frame, layout::{Constraint, Direction, Layout, Rect}, style::{Color, Style}, text::{Line, Span}, widgets::{Block, Borders, Clear, Paragraph}};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    match &app.screen.clone() {
        Screen::Dashboard => dashboard::render(f, app, area),
        Screen::ServiceDetail => service_detail::render(f, app, area),
        Screen::DeployProgress(dep_id) => deploy_log::render_progress(f, app, area, dep_id),
        Screen::Logs(svc_id) => deploy_log::render_logs(f, app, area, svc_id),
        Screen::Metrics(svc_id) => metrics::render(f, app, area, svc_id),
        Screen::Confirm { message, .. } => render_confirm(f, area, message),
    }

    if let Some(notif) = &app.notification {
        render_notification(f, area, &notif.message, notif.is_error);
    }
}

fn render_notification(f: &mut Frame, area: Rect, message: &str, is_error: bool) {
    let width = (message.len() as u16 + 4).min(area.width - 2);
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
    let text = Paragraph::new(message)
        .block(block)
        .style(Style::default().fg(color));
    f.render_widget(Clear, notif_area);
    f.render_widget(text, notif_area);
}

fn render_confirm(f: &mut Frame, area: Rect, message: &str) {
    let popup = centered_rect(60, 20, area);
    let block = Block::default()
        .title(" Confirm ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let text = Paragraph::new(vec![
        Line::from(message),
        Line::from(""),
        Line::from(vec![
            Span::styled("[y] Yes  ", Style::default().fg(Color::Green)),
            Span::styled("[n] No", Style::default().fg(Color::Red)),
        ]),
    ])
    .block(block);
    f.render_widget(Clear, popup);
    f.render_widget(text, popup);
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
