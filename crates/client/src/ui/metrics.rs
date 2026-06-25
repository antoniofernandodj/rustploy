use crate::app::App;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
};

// ── Global monitoring (Home > Monitoring) ─────────────────────────────────────

pub fn render_global(f: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // SO: CPU | RAM
            Constraint::Percentage(50), // Containers: CPU total | RAM total
        ])
        .split(area);

    render_os_row(f, app, rows[0]);
    render_aggregate_row(f, app, rows[1]);
}

// ── Linha 1: métricas de SO ────────────────────────────────────────────────

fn render_os_row(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    if app.system_metrics.is_empty() {
        let waiting = Paragraph::new(Span::styled(
            "  Aguardando métricas do sistema…",
            Style::default().fg(Color::DarkGray),
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Sistema — CPU% ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(waiting, cols[0]);
        f.render_widget(
            Paragraph::new("").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Sistema — RAM ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            cols[1],
        );
        return;
    }

    let latest = app.system_metrics.back().unwrap();
    let n = app.system_metrics.len() as f64;

    // CPU
    let cpu_data: Vec<(f64, f64)> = app
        .system_metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (i as f64, m.cpu_percent))
        .collect();
    let cpu_max = cpu_data.iter().map(|(_, v)| *v).fold(5.0f64, f64::max);
    render_chart(
        f,
        cols[0],
        &cpu_data,
        Color::Green,
        format!(" SO — CPU  {:.1}% ", latest.cpu_percent),
        [0.0, n],
        [0.0, cpu_max.max(100.0)],
        "0%",
        "100%",
    );

    // RAM
    let mem_total_gb = latest.mem_total_bytes as f64 / 1_073_741_824.0;
    let mem_used_gb = latest.mem_used_bytes as f64 / 1_073_741_824.0;
    let mem_data: Vec<(f64, f64)> = app
        .system_metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (i as f64, m.mem_used_bytes as f64 / 1_073_741_824.0))
        .collect();
    render_chart(
        f,
        cols[1],
        &mem_data,
        Color::Blue,
        format!(" SO — RAM  {:.1}/{:.1} GB ", mem_used_gb, mem_total_gb),
        [0.0, n],
        [0.0, mem_total_gb.max(1.0)],
        "0",
        &format!("{mem_total_gb:.0}G"),
    );
}

// ── Linha 2: agregado de todos os containers ───────────────────────────────

fn render_aggregate_row(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Calcular série temporal agregada:
    // Para cada ponto de tempo (índice), somar todos os serviços.
    // Usamos o comprimento mínimo entre todos os serviços para ter séries alinhadas.
    let service_bufs: Vec<_> = app.metrics.values().collect();

    if service_bufs.is_empty() {
        let msg = Paragraph::new(Span::styled(
            "  Nenhum container com dados de métricas.",
            Style::default().fg(Color::DarkGray),
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Containers — CPU total ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(msg, cols[0]);
        f.render_widget(
            Paragraph::new("").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Containers — RAM total ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            cols[1],
        );
        return;
    }

    let min_len = service_bufs.iter().map(|b| b.len()).min().unwrap_or(0);
    if min_len == 0 {
        let msg = Paragraph::new(Span::styled(
            "  Aguardando métricas dos containers…",
            Style::default().fg(Color::DarkGray),
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Containers — CPU total ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );
        f.render_widget(msg, cols[0]);
        f.render_widget(
            Paragraph::new("").block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Containers — RAM total ")
                    .border_style(Style::default().fg(Color::DarkGray)),
            ),
            cols[1],
        );
        return;
    }

    // Somar por índice (alinhado pelo tail)
    let mut agg_cpu: Vec<(f64, f64)> = Vec::with_capacity(min_len);
    let mut agg_mem: Vec<(f64, f64)> = Vec::with_capacity(min_len);

    for idx in 0..min_len {
        let offset_from_end = min_len - 1 - idx;
        let cpu_sum: f64 = service_bufs
            .iter()
            .map(|b| {
                let rev_idx = b.len() - 1 - offset_from_end;
                b[rev_idx].cpu_percent
            })
            .sum();
        let mem_sum: u64 = service_bufs
            .iter()
            .map(|b| {
                let rev_idx = b.len() - 1 - offset_from_end;
                b[rev_idx].mem_used_bytes
            })
            .sum();
        agg_cpu.push((idx as f64, cpu_sum));
        agg_mem.push((idx as f64, mem_sum as f64 / 1_073_741_824.0));
    }

    let n = min_len as f64;
    let latest_cpu: f64 = agg_cpu.last().map(|(_, v)| *v).unwrap_or(0.0);
    let latest_mem: f64 = agg_mem.last().map(|(_, v)| *v).unwrap_or(0.0);
    let cpu_max = agg_cpu.iter().map(|(_, v)| *v).fold(5.0f64, f64::max);
    let mem_max = agg_mem.iter().map(|(_, v)| *v).fold(0.1f64, f64::max);

    render_chart(
        f,
        cols[0],
        &agg_cpu,
        Color::Yellow,
        format!(" Containers — CPU total  {:.1}% ", latest_cpu),
        [0.0, n],
        [0.0, cpu_max * 1.1],
        "0%",
        &format!("{cpu_max:.0}%"),
    );

    render_chart(
        f,
        cols[1],
        &agg_mem,
        Color::Magenta,
        format!(" Containers — RAM total  {:.1} GB ", latest_mem),
        [0.0, n],
        [0.0, mem_max * 1.1],
        "0",
        &format!("{mem_max:.1}G"),
    );
}

// ── Helper genérico de gráfico ────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn render_chart(
    f: &mut Frame,
    area: Rect,
    data: &[(f64, f64)],
    color: Color,
    title: String,
    x_bounds: [f64; 2],
    y_bounds: [f64; 2],
    y_label_lo: &str,
    y_label_hi: &str,
) {
    let ds = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(color))
        .data(data);

    let chart = Chart::new(vec![ds])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds(x_bounds),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(Color::DarkGray))
                .bounds(y_bounds)
                .labels(vec![Span::raw(y_label_lo), Span::raw(y_label_hi)]),
        );

    f.render_widget(chart, area);
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
    let n = metrics.len() as f64;

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // CPU
    let cpu_data: Vec<(f64, f64)> = metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (i as f64, m.cpu_percent))
        .collect();
    let cpu_max = cpu_data.iter().map(|(_, v)| *v).fold(5.0f64, f64::max);
    render_chart(
        f,
        rows[0],
        &cpu_data,
        Color::Green,
        format!(" {} — CPU  {:.1}% ", svc_name, latest.cpu_percent),
        [0.0, n],
        [0.0, cpu_max.max(5.0)],
        "0%",
        &format!("{cpu_max:.0}%"),
    );

    // RAM
    let mem_used_mb = latest.mem_used_bytes as f64 / 1_048_576.0;
    let mem_limit_mb = latest.mem_limit_bytes as f64 / 1_048_576.0;
    let mem_data: Vec<(f64, f64)> = metrics
        .iter()
        .enumerate()
        .map(|(i, m)| (i as f64, m.mem_used_bytes as f64 / 1_048_576.0))
        .collect();
    let mem_max = mem_data.iter().map(|(_, v)| *v).fold(1.0f64, f64::max);
    let mem_title = if mem_limit_mb > 0.0 {
        format!(" {} — RAM  {:.0}/{:.0} MB ", svc_name, mem_used_mb, mem_limit_mb)
    } else {
        format!(" {} — RAM  {:.0} MB ", svc_name, mem_used_mb)
    };
    render_chart(
        f,
        rows[1],
        &mem_data,
        Color::Blue,
        mem_title,
        [0.0, n],
        [0.0, mem_max * 1.1],
        "0",
        &format!("{mem_max:.0}M"),
    );

}
