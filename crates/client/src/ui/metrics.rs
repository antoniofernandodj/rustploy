use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
};

/// Global monitoring view — shown in Home > Monitoring.
pub fn render_global(f: &mut Frame, app: &App, area: Rect) {
    if app.services.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Monitoring ")
            .border_style(Style::default().fg(Color::DarkGray));
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Nenhum serviço em execução.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(block);
        f.render_widget(p, area);
        return;
    }

    let svc_with_metrics: Vec<_> = app
        .services
        .iter()
        .filter(|s| app.metrics.contains_key(&s.id))
        .collect();

    if svc_with_metrics.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Monitoring ")
            .border_style(Style::default().fg(Color::DarkGray));
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Aguardando métricas dos containers...",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(block);
        f.render_widget(p, area);
        return;
    }

    let svc = svc_with_metrics[0];
    render_service_charts(f, app, area, &svc.id, &svc.spec.name);
}

/// Charts for a specific service — reusable from service detail.
pub fn render_service_charts(
    f: &mut Frame,
    app: &App,
    area: Rect,
    service_id: &str,
    svc_name: &str,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    let title_line =
        Paragraph::new(format!(" Métricas: {svc_name}")).style(Style::default().fg(Color::Cyan));
    f.render_widget(title_line, chunks[0]);

    let metrics: Vec<_> = app
        .metrics
        .get(service_id)
        .into_iter()
        .flatten()
        .cloned()
        .collect();

    let cpu_data: Vec<(f64, f64)> = metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (i as f64, m.cpu_percent))
        .collect();
    let cpu_max = cpu_data.iter().map(|(_, v)| *v).fold(10.0f64, f64::max);

    let cpu_ds = Dataset::default()
        .name("CPU%")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Green))
        .data(&cpu_data);

    let cpu_chart = Chart::new(vec![cpu_ds])
        .block(Block::default().borders(Borders::ALL).title(" CPU% "))
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, metrics.len() as f64]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, cpu_max])
                .labels(vec![Span::raw("0%"), Span::raw(format!("{cpu_max:.0}%"))]),
        );
    f.render_widget(cpu_chart, chunks[1]);

    let mem_data: Vec<(f64, f64)> = metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (i as f64, m.mem_used_bytes as f64 / 1_048_576.0))
        .collect();
    let mem_max = mem_data.iter().map(|(_, v)| *v).fold(64.0f64, f64::max);

    let mem_ds = Dataset::default()
        .name("RAM (MB)")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Blue))
        .data(&mem_data);

    let mem_chart = Chart::new(vec![mem_ds])
        .block(Block::default().borders(Borders::ALL).title(" RAM (MB) "))
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, metrics.len() as f64]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, mem_max])
                .labels(vec![Span::raw("0"), Span::raw(format!("{mem_max:.0}M"))]),
        );
    f.render_widget(mem_chart, chunks[2]);
}
