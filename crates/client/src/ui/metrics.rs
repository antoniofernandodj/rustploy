use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect, service_id: &str) {
    let svc_name = app
        .services
        .iter()
        .find(|s| s.id == service_id)
        .map(|s| s.spec.name.as_str())
        .unwrap_or(service_id);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Percentage(50),
            Constraint::Percentage(50),
            Constraint::Length(1),
        ])
        .split(area);

    let title = Paragraph::new(format!(" Métricas: {svc_name}"))
        .style(Style::default().fg(Color::Cyan));
    f.render_widget(title, chunks[0]);

    let metrics: Vec<_> = app
        .metrics
        .get(service_id)
        .into_iter()
        .flatten()
        .cloned()
        .collect();

    // CPU chart
    let cpu_data: Vec<(f64, f64)> = metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (i as f64, m.cpu_percent))
        .collect();

    let cpu_max = cpu_data.iter().map(|(_, v)| *v).fold(10.0f64, f64::max);

    let cpu_dataset = Dataset::default()
        .name("CPU%")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Green))
        .data(&cpu_data);

    let cpu_chart = Chart::new(vec![cpu_dataset])
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
                .labels(vec![
                    ratatui::text::Span::raw("0%"),
                    ratatui::text::Span::raw(format!("{cpu_max:.0}%")),
                ]),
        );
    f.render_widget(cpu_chart, chunks[1]);

    // Memory chart
    let mem_data: Vec<(f64, f64)> = metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (i as f64, m.mem_used_bytes as f64 / 1_048_576.0))
        .collect();

    let mem_max = mem_data.iter().map(|(_, v)| *v).fold(64.0f64, f64::max);

    let mem_dataset = Dataset::default()
        .name("RAM (MB)")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Blue))
        .data(&mem_data);

    let mem_chart = Chart::new(vec![mem_dataset])
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
                .labels(vec![
                    ratatui::text::Span::raw("0"),
                    ratatui::text::Span::raw(format!("{mem_max:.0}M")),
                ]),
        );
    f.render_widget(mem_chart, chunks[2]);

    let help = Paragraph::new(" [Esc] voltar")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[3]);
}
