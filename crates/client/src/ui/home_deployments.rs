use crate::app::App;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table},
    Frame,
};
use shared::DeployState;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Deployments ")
        .border_style(Style::default().fg(Color::DarkGray));

    let summaries = &app.home_deployments;

    if summaries.is_empty() {
        let msg = if app.projects.is_empty() {
            "  Nenhum projeto cadastrado ainda."
        } else {
            "  Nenhum deployment encontrado."
        };
        f.render_widget(
            ratatui::widgets::Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(msg, Style::default().fg(Color::DarkGray))),
            ])
            .block(block),
            area,
        );
        return;
    }

    let header = Row::new(vec![
        Cell::from("  Serviço").style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        Cell::from("Projeto").style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        Cell::from("Estado").style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        Cell::from("Duração").style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
        Cell::from("Início").style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
    ])
    .height(1)
    .bottom_margin(0);

    let rows: Vec<Row> = summaries
        .iter()
        .map(|s| {
            let dep = &s.deployment;
            let (state_label, state_color) = state_display(&dep.state);
            let duration = dep
                .finished_at
                .map(|fin| fmt_duration((fin - dep.started_at).num_seconds()))
                .unwrap_or_else(|| {
                    let secs = (chrono::Utc::now() - dep.started_at).num_seconds();
                    fmt_duration(secs)
                });
            let started = dep.started_at.format("%H:%M:%S").to_string();

            Row::new(vec![
                Cell::from(format!("  {}", s.service_name))
                    .style(Style::default().fg(Color::Cyan)),
                Cell::from(s.project_name.as_str())
                    .style(Style::default().fg(Color::White)),
                Cell::from(state_label).style(Style::default().fg(state_color)),
                Cell::from(duration).style(Style::default().fg(Color::DarkGray)),
                Cell::from(started).style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(22),
        Constraint::Percentage(18),
        Constraint::Percentage(18),
        Constraint::Percentage(12),
        Constraint::Percentage(12),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block);

    f.render_widget(table, area);
}

fn state_display(state: &DeployState) -> (&'static str, Color) {
    match state {
        DeployState::Live             => ("● Live",             Color::Green),
        DeployState::Stopped          => ("○ Stopped",          Color::DarkGray),
        DeployState::Failed           => ("✕ Failed",           Color::Red),
        DeployState::RollingBack      => ("↩ Rolling back",     Color::Red),
        DeployState::Pending          => ("◌ Pending",          Color::Yellow),
        DeployState::ResolvingDeps    => ("◌ Resolving",        Color::Yellow),
        DeployState::PullingImage     => ("◌ Pulling",          Color::Yellow),
        DeployState::CloningRepo      => ("◌ Cloning",          Color::Yellow),
        DeployState::BuildingImage    => ("◌ Building",         Color::Yellow),
        DeployState::Staging          => ("◌ Staging",          Color::Yellow),
        DeployState::HealthcheckPolling => ("◌ Healthcheck",    Color::Yellow),
        DeployState::SwappingIn       => ("◌ Swapping",         Color::Yellow),
        DeployState::Draining         => ("◌ Draining",         Color::Yellow),
        DeployState::Promoting        => ("◌ Promoting",        Color::Yellow),
        DeployState::Pruning          => ("◌ Pruning",          Color::DarkGray),
        DeployState::ComposingUp      => ("◌ Composing",         Color::Yellow),
    }
}

fn fmt_duration(secs: i64) -> String {
    if secs < 0 {
        return "—".into();
    }
    if secs < 60 {
        format!("{secs}s")
    } else {
        format!("{}m {}s", secs / 60, secs % 60)
    }
}
