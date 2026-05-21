use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use shared::EnvVarValue;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let Some(svc) = app.current_service() else {
        f.render_widget(Paragraph::new("No service selected"), area);
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    // Service info
    let source_desc = match &svc.spec.source {
        shared::ServiceSource::Registry { image } => format!("registry: {image}"),
        shared::ServiceSource::Git(g) => format!("git: {} @ {}", g.url, g.branch),
    };

    let info_lines = vec![
        Line::from(format!(" Nome:      {}", svc.spec.name)),
        Line::from(format!(" Domínio:   {}", svc.spec.domain)),
        Line::from(format!(" Porta:     {}", svc.spec.port)),
        Line::from(format!(" Status:    {}", svc.status)),
        Line::from(format!(" Fonte:     {source_desc}")),
        Line::from(format!(
            " Container: {}",
            svc.live_container_id.as_deref().unwrap_or("none")
        )),
    ];

    let info = Paragraph::new(info_lines)
        .block(Block::default().borders(Borders::ALL).title(format!(" Serviço: {} ", svc.spec.name)));
    f.render_widget(info, chunks[0]);

    // Env vars (masked)
    let env_items: Vec<ListItem> = svc
        .spec
        .env_vars
        .iter()
        .map(|e| {
            let val = match &e.value {
                EnvVarValue::Plain(v) => {
                    if v.len() > 20 {
                        format!("{}...", &v[..20])
                    } else {
                        v.clone()
                    }
                }
                EnvVarValue::Secret(name) => format!("<secret:{name}>"),
            };
            ListItem::new(format!("  {}={}", e.key, val))
        })
        .collect();

    let env_list = List::new(env_items)
        .block(Block::default().borders(Borders::ALL).title(" Variáveis de Ambiente "));
    f.render_widget(env_list, chunks[1]);

    let help = Paragraph::new(
        " [d]eploy  [l]ogs  [m]étricas  [r]ollback  [Esc] voltar",
    )
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(help, chunks[2]);
}
