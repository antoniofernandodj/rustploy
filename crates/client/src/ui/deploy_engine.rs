use crate::app::App;
use chrono::Utc;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use shared::{ActiveDeployInfo, DeployEngineSummary, DeployState};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    match &app.deploy_engine {
        None => render_loading(f, area),
        Some(summary) => render_dashboard(f, summary, area),
    }
}

fn render_loading(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Deploy Engine ")
        .border_style(Style::default().fg(Color::DarkGray));
    let p = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Carregando...",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(block);
    f.render_widget(p, area);
}

fn render_dashboard(f: &mut Frame, s: &DeployEngineSummary, area: Rect) {
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" Deploy Engine ")
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // padding
            Constraint::Length(3), // stat cards
            Constraint::Length(1), // padding
            Constraint::Min(0),    // main content
            Constraint::Length(1), // hints
        ])
        .split(inner);

    render_stat_cards(f, s, chunks[1]);
    render_main(f, s, chunks[3]);
    render_hints(f, chunks[4]);
}

fn render_stat_cards(f: &mut Frame, s: &DeployEngineSummary, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
            Constraint::Ratio(1, 5),
        ])
        .split(area);

    render_card(
        f,
        cols[0],
        "Ativos",
        &s.active.len().to_string(),
        Color::Yellow,
        "◌",
    );
    render_card(
        f,
        cols[1],
        "Sucesso 24h",
        &s.successful_24h.to_string(),
        Color::Green,
        "✓",
    );
    render_card(
        f,
        cols[2],
        "Falhas 24h",
        &s.failed_24h.to_string(),
        if s.failed_24h > 0 {
            Color::Red
        } else {
            Color::DarkGray
        },
        "✕",
    );
    render_card(
        f,
        cols[3],
        "Total 24h",
        &s.total_24h.to_string(),
        Color::DarkGray,
        "⊙",
    );
    render_card(
        f,
        cols[4],
        "Uptime",
        &fmt_uptime(s.uptime_secs),
        Color::Cyan,
        "↑",
    );
}

fn render_card(f: &mut Frame, area: Rect, label: &str, value: &str, color: Color, icon: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{icon} "), Style::default().fg(color)),
            Span::styled(
                value,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
        ])),
        rows[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled(label, Style::default().fg(Color::DarkGray))),
        rows[1],
    );
}

fn render_main(f: &mut Frame, s: &DeployEngineSummary, area: Rect) {
    let active_height = (s.active.len() + 2).max(4) as u16; // header + rows + padding
    let recent_min = area.height.saturating_sub(active_height + 1);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(active_height),
            Constraint::Min(recent_min),
        ])
        .split(area);

    render_active_section(f, s, chunks[0]);
    render_recent_section(f, s, chunks[1]);
}

fn render_active_section(f: &mut Frame, s: &DeployEngineSummary, area: Rect) {
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(
            "  ── Executando agora ─────────────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    if s.active.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Nenhum deploy em progresso.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for info in &s.active {
            lines.push(active_row(info));
        }
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn active_row(info: &ActiveDeployInfo) -> Line<'static> {
    let bar = progress_bar(info.percent, 18);
    let bar_color = match &info.state {
        DeployState::RollingBack | DeployState::Failed => Color::Red,
        DeployState::HealthcheckPolling | DeployState::Staging => Color::Yellow,
        DeployState::Live => Color::Green,
        _ => Color::Cyan,
    };

    let svc = truncate(&info.service_name, 14);
    let proj = truncate(&info.project_name, 12);
    let state_lbl = truncate(info.state.label(), 20);
    let elapsed = fmt_duration(info.elapsed_secs);
    let state_secs = fmt_duration(info.current_state_secs);

    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{svc:<14}"),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" [{proj:<12}]"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(bar, Style::default().fg(bar_color)),
        Span::raw("  "),
        Span::styled(
            format!("{:>3}%", info.percent),
            Style::default().fg(Color::White),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{state_lbl:<20}"),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("  "),
        Span::styled(
            format!("total {elapsed}"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(
            format!("fase {state_secs}"),
            Style::default().fg(Color::DarkGray),
        ),
    ])
}

fn render_recent_section(f: &mut Frame, s: &DeployEngineSummary, area: Rect) {
    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  ── Histórico 24h ────────────────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    if s.recent.is_empty() {
        lines.push(Line::from(Span::styled(
            "  Nenhum deploy concluído nas últimas 24h.",
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        let now = Utc::now();
        for info in &s.recent {
            lines.push(recent_row(info, now));
        }
    }

    f.render_widget(Paragraph::new(lines), area);
}

fn recent_row(info: &ActiveDeployInfo, now: chrono::DateTime<Utc>) -> Line<'static> {
    let (icon, icon_color) = match &info.state {
        DeployState::Live => ("✓", Color::Green),
        DeployState::Failed => ("✕", Color::Red),
        _ => ("○", Color::DarkGray),
    };

    let svc = truncate(&info.service_name, 14);
    let proj = truncate(&info.project_name, 12);
    let state_lbl = truncate(info.state.label(), 10);
    let ago_secs = (now - info.started_at).num_seconds().max(0) as u64;
    let ago = fmt_ago(ago_secs);
    let duration = fmt_duration(info.elapsed_secs);

    Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("{icon}  "),
            Style::default().fg(icon_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{svc:<14}"), Style::default().fg(Color::White)),
        Span::styled(
            format!(" [{proj:<12}]"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(format!("{state_lbl:<10}"), Style::default().fg(icon_color)),
        Span::raw("  "),
        Span::styled(
            format!("há {ago:<10}"),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw("  "),
        Span::styled(
            format!("duração {duration}"),
            Style::default().fg(Color::DarkGray),
        ),
    ])
}

fn render_hints(f: &mut Frame, area: Rect) {
    let p = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("[r]", Style::default().fg(Color::Yellow)),
        Span::raw(" atualizar"),
    ]));
    f.render_widget(p, area);
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn progress_bar(percent: u8, width: usize) -> String {
    let filled = (width * percent.min(100) as usize) / 100;
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn fmt_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        format!("{days}d {hours}h {mins}m")
    } else if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

fn fmt_duration(secs: u64) -> String {
    let mins = secs / 60;
    let s = secs % 60;
    if mins > 0 {
        format!("{mins}m {s:02}s")
    } else {
        format!("{s}s")
    }
}

fn fmt_ago(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86400 {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m > 0 {
            format!("{h}h {m}m")
        } else {
            format!("{h}h")
        }
    } else {
        format!("{}d", secs / 86400)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{t}…")
    }
}
