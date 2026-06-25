use crate::app::{App, DockerPruneButton, PruneSlot};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

const GRAY: Color = Color::DarkGray;
const CYAN: Color = Color::Cyan;
const GREEN: Color = Color::Green;
const RED: Color = Color::Red;
const YELLOW: Color = Color::Yellow;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Docker — Limpeza ")
        .border_style(Style::default().fg(GRAY));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // espaço
            Constraint::Length(1), // header
            Constraint::Length(1), // espaço
            Constraint::Length(2), // containers
            Constraint::Length(1), // espaço
            Constraint::Length(2), // volumes
            Constraint::Length(1), // espaço
            Constraint::Length(2), // images
            Constraint::Length(1), // espaço
            Constraint::Length(2), // build cache
            Constraint::Length(1), // espaço
            Constraint::Length(1), // dica teclas
            Constraint::Min(0),
        ])
        .split(inner);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  Selecione uma categoria e pressione Enter para limpar.",
            Style::default().fg(GRAY),
        ))),
        rows[1],
    );

    let p = &app.docker_prune;
    render_row(f, rows[3],  "Containers parados",  &DockerPruneButton::Containers, &p.containers, &p.focused);
    render_row(f, rows[5],  "Volumes sem uso",     &DockerPruneButton::Volumes,    &p.volumes,    &p.focused);
    render_row(f, rows[7],  "Imagens sem uso",     &DockerPruneButton::Images,     &p.images,     &p.focused);
    render_row(f, rows[9],  "Cache de build",      &DockerPruneButton::BuildCache, &p.build_cache,&p.focused);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  ↑↓ navegar   Enter limpar",
            Style::default().fg(GRAY),
        ))),
        rows[11],
    );
}

fn render_row(
    f: &mut Frame,
    area: Rect,
    label: &str,
    btn: &DockerPruneButton,
    slot: &PruneSlot,
    focused: &DockerPruneButton,
) {
    let is_focused = btn == focused;
    let btn_style = if is_focused {
        Style::default().fg(Color::Black).bg(CYAN).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(GRAY)
    };

    let status_span = match slot {
        PruneSlot::Idle => Span::raw(""),
        PruneSlot::Running => Span::styled("  aguardando…", Style::default().fg(YELLOW)),
        PruneSlot::Done { count, reclaimed_bytes } => {
            let mb = *reclaimed_bytes / 1_000_000;
            Span::styled(
                format!("  ✓ {} removido(s), {} MB liberados", count, mb),
                Style::default().fg(GREEN),
            )
        }
        PruneSlot::Error(msg) => {
            Span::styled(format!("  ✗ {msg}"), Style::default().fg(RED))
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  [ Limpar ] ", btn_style),
            Span::styled(format!("{label:<24}"), Style::default().fg(if is_focused { CYAN } else { Color::White })),
            status_span,
        ])),
        chunks[0],
    );
}
