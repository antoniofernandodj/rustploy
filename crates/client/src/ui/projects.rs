use crate::app::{App, Focus, ServiceFormField};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use shared::ServiceStatus;

pub fn render_project_detail(f: &mut Frame, app: &App, area: Rect) {
    let project_name = app
        .current_project()
        .map(|p| p.name.as_str())
        .unwrap_or("Projeto");

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let filter_display = if app.service_filtering {
        format!(" Filtro: {}▌", app.service_filter)
    } else if !app.service_filter.is_empty() {
        format!(" Filtro: {} ", app.service_filter)
    } else {
        String::new()
    };

    let header_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let header_text = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {project_name}"),
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(filter_display, Style::default().fg(Color::Yellow)),
    ]))
    .block(header_block);
    f.render_widget(header_text, chunks[0]);

    let focused = app.focus == Focus::Content;
    let filtered = app.filtered_services();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, svc)| {
            let selected = focused && i == app.service_cursor;
            let status_color = match &svc.status {
                ServiceStatus::Running => Color::Green,
                ServiceStatus::Stopped => Color::DarkGray,
                ServiceStatus::Deploying => Color::Yellow,
                ServiceStatus::Degraded => Color::Magenta,
                ServiceStatus::Error(_) => Color::Red,
            };
            let status_label = match &svc.status {
                ServiceStatus::Running => "RUNNING",
                ServiceStatus::Stopped => "STOPPED",
                ServiceStatus::Deploying => "DEPLOYING",
                ServiceStatus::Degraded => "DEGRADED",
                ServiceStatus::Error(_) => "ERROR",
            };

            let metrics_str = app
                .metrics
                .get(&svc.id)
                .and_then(|m| m.back())
                .map(|m| {
                    format!(
                        "  ↑{:.0}M {:.0}%",
                        m.mem_used_bytes as f64 / 1_048_576.0,
                        m.cpu_percent
                    )
                })
                .unwrap_or_default();

            let name_style = if selected {
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {:<28}", svc.spec.name), name_style),
                Span::styled(
                    format!("[{:<10}]", status_label),
                    Style::default().fg(status_color),
                ),
                Span::styled(metrics_str, Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::NONE)
        .style(Style::default());

    let list = List::new(items).block(list_block).highlight_style(
        Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    if focused {
        state.select(Some(app.service_cursor));
    }
    f.render_stateful_widget(list, chunks[1], &mut state);

    if filtered.is_empty() {
        let msg = if app.service_filter.is_empty() {
            "Nenhum serviço. Pressione [n] para criar."
        } else {
            "Nenhum serviço corresponde ao filtro."
        };
        let p = Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(p, chunks[1]);
    }
}

pub fn render_service_form(f: &mut Frame, app: &App, area: Rect) {
    let form = match &app.service_form {
        Some(f) => f,
        None => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Novo Serviço ")
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // spacing
            Constraint::Length(1), // Name
            Constraint::Length(1), // Port
            Constraint::Length(1), // Domain
            Constraint::Length(1), // spacing
            Constraint::Length(1), // header Provider
            Constraint::Length(1), // Repo URL
            Constraint::Length(1), // Branch
            Constraint::Length(1), // Build Path
            Constraint::Length(1), // Watch Paths
            Constraint::Length(1), // Submodules
            Constraint::Length(1), // spacing
            Constraint::Length(1), // header Build Type
            Constraint::Length(1), // Docker File
            Constraint::Length(1), // Context Path
            Constraint::Length(1), // Build Stage
            Constraint::Length(1), // spacing
            Constraint::Length(1), // buttons
            Constraint::Min(0),
        ])
        .split(inner);

    let render_field = |f: &mut Frame, rect: Rect, label: &str, value: &str, focused: bool| {
        let style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        };
        let cursor = if focused { "▌" } else { "" };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {:<22}", label), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{value}{cursor}"), style),
            ])),
            rect,
        );
    };

    render_field(f, chunks[1], "Nome", &form.name, form.focused_field == ServiceFormField::Name);
    render_field(f, chunks[2], "Porta", &form.port, form.focused_field == ServiceFormField::Port);
    render_field(f, chunks[3], "Domínio", &form.domain, form.focused_field == ServiceFormField::Domain);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── Provider: Git ──────────────────────────",
            Style::default().fg(Color::Yellow),
        ))),
        chunks[5],
    );

    render_field(f, chunks[6], "Repository URL", &form.repo_url, form.focused_field == ServiceFormField::RepoUrl);
    render_field(f, chunks[7], "Branch", &form.branch, form.focused_field == ServiceFormField::Branch);
    render_field(f, chunks[8], "Build Path", &form.build_path, form.focused_field == ServiceFormField::BuildPath);
    render_field(f, chunks[9], "Watch Paths", &form.watch_paths, form.focused_field == ServiceFormField::WatchPaths);

    let sub_style = if form.focused_field == ServiceFormField::Submodules {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Submodules              ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if form.submodules { "[ Yes ]" } else { "[ No  ]" },
                sub_style,
            ),
        ])),
        chunks[10],
    );

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── Build Type: Dockerfile ─────────────────",
            Style::default().fg(Color::Yellow),
        ))),
        chunks[12],
    );

    render_field(f, chunks[13], "Docker File", &form.dockerfile, form.focused_field == ServiceFormField::DockerFile);
    render_field(f, chunks[14], "Docker Context Path", &form.context_path, form.focused_field == ServiceFormField::DockerContextPath);
    render_field(f, chunks[15], "Docker Build Stage", &form.build_stage, form.focused_field == ServiceFormField::DockerBuildStage);

    let btn_create = button_span("[ Create Service ]", form.focused_field == ServiceFormField::BtnCreate);
    let btn_cancel = button_span("[ Cancel ]", form.focused_field == ServiceFormField::BtnCancel);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            btn_create,
            Span::raw("  "),
            btn_cancel,
        ])),
        chunks[17],
    );
}

fn button_span(label: &str, focused: bool) -> Span<'static> {
    if focused {
        Span::styled(
            label.to_string(),
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(label.to_string(), Style::default().fg(Color::White))
    }
}
