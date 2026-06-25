use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Row, Table},
};
use shared::ServiceStatus;

// ── Global monitoring (Home > Monitoring) ─────────────────────────────────────

pub fn render_global(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // painel SO
            Constraint::Min(0),     // tabela serviços + charts
        ])
        .split(area);

    render_system_panel(f, app, chunks[0]);
    render_services_panel(f, app, chunks[1]);
}

fn render_system_panel(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Sistema ")
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.system_metrics.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  Aguardando métricas do sistema…",
                Style::default().fg(Color::DarkGray),
            )),
            inner,
        );
        return;
    }

    let latest = app.system_metrics.back().unwrap();

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(inner);

    // CPU
    let cpu_history: Vec<(f64, f64)> = app
        .system_metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (i as f64, m.cpu_percent))
        .collect();
    let cpu_max = cpu_history.iter().map(|(_, v)| *v).fold(10.0f64, f64::max);
    let cpu_ds = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Green))
        .data(&cpu_history);
    let cpu_chart = Chart::new(vec![cpu_ds])
        .block(Block::default().borders(Borders::ALL).title(format!(
            " CPU  {:.1}% ",
            latest.cpu_percent
        )))
        .x_axis(Axis::default().bounds([0.0, app.system_metrics.len() as f64]))
        .y_axis(
            Axis::default()
                .bounds([0.0, cpu_max.max(100.0)])
                .labels(vec![Span::raw("0%"), Span::raw("100%")]),
        );
    f.render_widget(cpu_chart, cols[0]);

    // RAM
    let mem_total_gb = latest.mem_total_bytes as f64 / 1_073_741_824.0;
    let mem_used_gb = latest.mem_used_bytes as f64 / 1_073_741_824.0;
    let mem_history: Vec<(f64, f64)> = app
        .system_metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (i as f64, m.mem_used_bytes as f64 / 1_073_741_824.0))
        .collect();
    let mem_ds = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Blue))
        .data(&mem_history);
    let mem_chart = Chart::new(vec![mem_ds])
        .block(Block::default().borders(Borders::ALL).title(format!(
            " RAM  {:.1}/{:.1} GB ",
            mem_used_gb, mem_total_gb
        )))
        .x_axis(Axis::default().bounds([0.0, app.system_metrics.len() as f64]))
        .y_axis(
            Axis::default()
                .bounds([0.0, mem_total_gb])
                .labels(vec![Span::raw("0"), Span::raw(format!("{mem_total_gb:.0}G"))]),
        );
    f.render_widget(mem_chart, cols[1]);

    // Disco + Load avg (texto simples)
    let disk_used_gb = latest.disk_used_bytes as f64 / 1_073_741_824.0;
    let disk_total_gb = latest.disk_total_bytes as f64 / 1_073_741_824.0;
    let disk_pct = if disk_total_gb > 0.0 {
        disk_used_gb / disk_total_gb * 100.0
    } else {
        0.0
    };

    let info = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Disco  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{:.1}/{:.1} GB  ({:.0}%)", disk_used_gb, disk_total_gb, disk_pct),
                Style::default().fg(if disk_pct > 85.0 { Color::Red } else { Color::White }),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Load   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!(
                    "{:.2}  {:.2}  {:.2}",
                    latest.load_avg_1, latest.load_avg_5, latest.load_avg_15
                ),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled("  (1m / 5m / 15m)", Style::default().fg(Color::DarkGray)),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Info "));
    f.render_widget(info, cols[2]);
}

fn render_services_panel(f: &mut Frame, app: &App, area: Rect) {
    let running: Vec<_> = app
        .services
        .iter()
        .filter(|s| matches!(s.status, ServiceStatus::Running | ServiceStatus::Deploying))
        .collect();

    if running.is_empty() {
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Nenhum container em execução.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Serviços ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(p, area);
        return;
    }

    let rows: Vec<Row> = running
        .iter()
        .map(|svc| {
            let last = app.metrics.get(&svc.id).and_then(|m| m.back());
            let cpu = last.map(|m| format!("{:.1}%", m.cpu_percent)).unwrap_or_else(|| "—".into());
            let mem = last
                .map(|m| {
                    let used_mb = m.mem_used_bytes / 1_048_576;
                    let limit_mb = m.mem_limit_bytes / 1_048_576;
                    if limit_mb > 0 {
                        format!("{used_mb} / {limit_mb} MB")
                    } else {
                        format!("{used_mb} MB")
                    }
                })
                .unwrap_or_else(|| "—".into());
            let net = last
                .map(|m| {
                    format!(
                        "↓{} ↑{}",
                        humanize_bytes(m.net_rx_bytes),
                        humanize_bytes(m.net_tx_bytes)
                    )
                })
                .unwrap_or_else(|| "—".into());

            Row::new(vec![svc.spec.name.clone(), cpu, mem, net])
        })
        .collect();

    let widths = [
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(22),
        Constraint::Length(20),
    ];
    let header = Row::new(vec!["Serviço", "CPU", "Memória", "Rede (total)"])
        .style(Style::default().fg(Color::DarkGray));

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Serviços ")
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .highlight_style(Style::default().fg(Color::Cyan));

    f.render_widget(table, area);
}

fn humanize_bytes(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;
    const KB: u64 = 1_024;
    if bytes >= GB {
        format!("{:.1}G", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}M", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}K", bytes as f64 / KB as f64)
    } else {
        format!("{bytes}B")
    }
}

// ── Per-service charts (reused by service detail Metrics tab) ─────────────────

pub fn render_service_charts(
    f: &mut Frame,
    app: &App,
    area: Rect,
    service_id: &str,
    svc_name: &str,
) {
    let metrics: Vec<_> = app
        .metrics
        .get(service_id)
        .into_iter()
        .flatten()
        .cloned()
        .collect();

    if metrics.is_empty() {
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Aguardando métricas do container…",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Métricas: {svc_name} "))
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(p, area);
        return;
    }

    let latest = metrics.last().unwrap();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    // CPU
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
        .block(Block::default().borders(Borders::ALL).title(format!(
            " CPU  {:.1}% ",
            latest.cpu_percent
        )))
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, metrics.len() as f64]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds([0.0, cpu_max.max(5.0)])
                .labels(vec![Span::raw("0%"), Span::raw(format!("{cpu_max:.0}%"))]),
        );
    f.render_widget(cpu_chart, chunks[0]);

    // RAM
    let mem_used_mb = latest.mem_used_bytes as f64 / 1_048_576.0;
    let mem_limit_mb = latest.mem_limit_bytes as f64 / 1_048_576.0;
    let mem_data: Vec<(f64, f64)> = metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (i as f64, m.mem_used_bytes as f64 / 1_048_576.0))
        .collect();
    let mem_max = mem_data.iter().map(|(_, v)| *v).fold(64.0f64, f64::max);
    let mem_ds = Dataset::default()
        .name("RAM")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Blue))
        .data(&mem_data);
    let mem_title = if mem_limit_mb > 0.0 {
        format!(" RAM  {mem_used_mb:.0}/{mem_limit_mb:.0} MB ")
    } else {
        format!(" RAM  {mem_used_mb:.0} MB ")
    };
    let mem_chart = Chart::new(vec![mem_ds])
        .block(Block::default().borders(Borders::ALL).title(mem_title))
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
    f.render_widget(mem_chart, chunks[1]);
}
