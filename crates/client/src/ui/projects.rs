use crate::app::{App, EnvEditField, Focus, ProjectDetailTab, SecretEditField};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use shared::ServiceStatus;

pub fn render_project_detail(f: &mut Frame, app: &App, area: Rect) {
    let project_name = app
        .current_project()
        .map(|p| p.name.as_str())
        .unwrap_or("Projeto");

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header + tabs
            Constraint::Min(0),    // content
        ])
        .split(area);

    // ── Header com tabs ───────────────────────────────────────────────────────
    let header_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));

    let tabs = [
        ProjectDetailTab::Services,
        ProjectDetailTab::Environment,
        ProjectDetailTab::Secrets,
    ];
    let mut tab_spans = vec![
        Span::styled(
            format!(" {project_name} "),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ", Style::default()),
    ];
    for tab in &tabs {
        let active = *tab == app.project_detail_tab;
        let style = if active {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        tab_spans.push(Span::styled(format!(" {} ", tab.label()), style));
        tab_spans.push(Span::raw("  "));
    }

    let header_text = Paragraph::new(Line::from(tab_spans)).block(header_block);
    f.render_widget(header_text, chunks[0]);

    // ── Conteúdo da aba ───────────────────────────────────────────────────────
    match app.project_detail_tab {
        ProjectDetailTab::Services => render_services_tab(f, app, chunks[1]),
        ProjectDetailTab::Environment => render_project_env_tab(f, app, chunks[1]),
        ProjectDetailTab::Secrets => render_project_secrets_tab(f, app, chunks[1]),
    }
}

// ── Aba de serviços ───────────────────────────────────────────────────────────

fn render_services_tab(f: &mut Frame, app: &App, area: Rect) {
    let filter_display = if app.service_filtering {
        format!(" Filtro: {}▌", app.service_filter)
    } else if !app.service_filter.is_empty() {
        format!(" Filtro: {} ", app.service_filter)
    } else {
        String::new()
    };

    // Linha de instruções + filtro
    let hint_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    if !filter_display.is_empty() {
        let filter_p = Paragraph::new(Line::from(Span::styled(
            filter_display,
            Style::default().fg(Color::Yellow),
        )));
        f.render_widget(filter_p, hint_chunks[0]);
    }

    let focused = app.focus == Focus::Content;
    let filtered = app.filtered_services();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, svc)| {
            let selected = focused && i == app.service_cursor;
            let status_color = match &svc.status {
                ServiceStatus::Running => Color::Green,
                ServiceStatus::Stopping => Color::Yellow,
                ServiceStatus::Stopped => Color::DarkGray,
                ServiceStatus::Deploying => Color::Yellow,
                ServiceStatus::Degraded => Color::Magenta,
                ServiceStatus::Error(_) => Color::Red,
            };
            let status_label = match &svc.status {
                ServiceStatus::Running => "RUNNING",
                ServiceStatus::Stopping => "STOPPING",
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
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
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

    let list_block = Block::default().borders(Borders::NONE);
    let list = List::new(items).block(list_block).highlight_style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    if focused {
        state.select(Some(app.service_cursor));
    }
    f.render_stateful_widget(list, hint_chunks[1], &mut state);

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
        f.render_widget(p, hint_chunks[1]);
    }
}

// ── Aba de env vars do projeto ────────────────────────────────────────────────

pub fn render_project_env_tab(f: &mut Frame, app: &App, area: Rect) {
    let project = match app.current_project() {
        Some(p) => p,
        None => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Environment — herdado por todos os serviços ")
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(inner);

    // Lista de env vars
    if project.env_vars.is_empty() && !app.project_env_tab.editing {
        let p = Paragraph::new(Line::from(Span::styled(
            " Nenhuma variável. Pressione [n] para adicionar.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(p, chunks[0]);
    } else {
        let items: Vec<ListItem> = project
            .env_vars
            .iter()
            .enumerate()
            .map(|(i, ev)| {
                let selected = !app.project_env_tab.editing && i == app.project_env_tab.cursor;
                let val_display = match &ev.value {
                    shared::EnvVarValue::Plain(v) => v.clone(),
                    shared::EnvVarValue::Secret(s) => format!("<secret:{s}>"),
                };
                let style = if selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {:<24}", ev.key),
                        style.fg(if selected { Color::Black } else { Color::Cyan }),
                    ),
                    Span::styled(" = ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        val_display,
                        style.fg(if selected { Color::Black } else { Color::White }),
                    ),
                ]))
            })
            .collect();

        let mut list_state = ListState::default();
        if !app.project_env_tab.editing {
            list_state.select(Some(app.project_env_tab.cursor));
        }
        let list = List::new(items).highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        f.render_stateful_widget(list, chunks[0], &mut list_state);
    }

    // Formulário de edição inline
    if app.project_env_tab.editing {
        let form_block = Block::default()
            .borders(Borders::ALL)
            .title(" Nova variável ")
            .border_style(Style::default().fg(Color::Yellow));
        let form_inner = form_block.inner(chunks[1]);
        f.render_widget(form_block, chunks[1]);

        let key_style = if app.project_env_tab.edit_field == EnvEditField::Key {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let val_style = if app.project_env_tab.edit_field == EnvEditField::Value {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let cursor_k = if app.project_env_tab.edit_field == EnvEditField::Key {
            "▌"
        } else {
            ""
        };
        let cursor_v = if app.project_env_tab.edit_field == EnvEditField::Value {
            "▌"
        } else {
            ""
        };

        let form_line = Line::from(vec![
            Span::styled(
                format!(" KEY: {}{}", app.project_env_tab.edit_key, cursor_k),
                key_style,
            ),
            Span::raw("   "),
            Span::styled(
                format!("VALUE: {}{}", app.project_env_tab.edit_value, cursor_v),
                val_style,
            ),
        ]);
        f.render_widget(Paragraph::new(form_line), form_inner);
    } else {
        // Dicas de teclas
        let hint = Line::from(vec![
            Span::styled(" [n]", Style::default().fg(Color::Cyan)),
            Span::styled(" novo  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[e]", Style::default().fg(Color::Cyan)),
            Span::styled(" editar  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[D]", Style::default().fg(Color::Red)),
            Span::styled(" remover  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[←→]", Style::default().fg(Color::DarkGray)),
            Span::styled(" trocar aba", Style::default().fg(Color::DarkGray)),
        ]);
        f.render_widget(Paragraph::new(hint), chunks[1]);
    }
}

// ── Aba de secrets do projeto ─────────────────────────────────────────────────

pub fn render_project_secrets_tab(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Secrets — credenciais criptografadas por projeto ")
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(inner);

    if app.project_secrets.is_empty() && !app.secrets_tab.adding {
        let p = Paragraph::new(Line::from(Span::styled(
            " Nenhum secret. Pressione [n] para adicionar.",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(p, chunks[0]);
    } else {
        let items: Vec<ListItem> = app
            .project_secrets
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let selected = !app.secrets_tab.adding && i == app.secrets_tab.cursor;
                let style = if selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!(" {:<28}", name), style.fg(if selected { Color::Black } else { Color::Cyan })),
                    Span::styled(" ••••••••", style.fg(if selected { Color::Black } else { Color::DarkGray })),
                ]))
            })
            .collect();

        let mut list_state = ListState::default();
        if !app.secrets_tab.adding {
            list_state.select(Some(app.secrets_tab.cursor));
        }
        let list = List::new(items).highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        f.render_stateful_widget(list, chunks[0], &mut list_state);
    }

    if app.secrets_tab.adding {
        let form_block = Block::default()
            .borders(Borders::ALL)
            .title(" Novo secret ")
            .border_style(Style::default().fg(Color::Yellow));
        let form_inner = form_block.inner(chunks[1]);
        f.render_widget(form_block, chunks[1]);

        let name_style = if app.secrets_tab.edit_field == SecretEditField::Name {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let val_style = if app.secrets_tab.edit_field == SecretEditField::Value {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let cursor_n = if app.secrets_tab.edit_field == SecretEditField::Name { "▌" } else { "" };
        let cursor_v = if app.secrets_tab.edit_field == SecretEditField::Value { "▌" } else { "" };
        let masked_value: String = "•".repeat(app.secrets_tab.edit_value.len());

        let form_line = Line::from(vec![
            Span::styled(format!(" NAME: {}{}", app.secrets_tab.edit_name, cursor_n), name_style),
            Span::raw("   "),
            Span::styled(format!("VALUE: {}{}", masked_value, cursor_v), val_style),
        ]);
        f.render_widget(Paragraph::new(form_line), form_inner);
    } else {
        let hint = Line::from(vec![
            Span::styled(" [n]", Style::default().fg(Color::Cyan)),
            Span::styled(" novo  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[D]", Style::default().fg(Color::Red)),
            Span::styled(" remover  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[←→]", Style::default().fg(Color::DarkGray)),
            Span::styled(" trocar aba", Style::default().fg(Color::DarkGray)),
        ]);
        f.render_widget(Paragraph::new(hint), chunks[1]);
    }
}
