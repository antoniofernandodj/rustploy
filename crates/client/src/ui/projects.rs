use crate::app::{App, EnvEditField, Focus, ProjectDetailTab, ProjectSettingsField};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use shared::ServiceStatus;

pub fn render_projects_list(f: &mut Frame, app: &App, area: Rect) {
    let focused = app.focus == Focus::Content;

    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" Projects ")
        .border_style(Style::default().fg(if focused {
            Color::Cyan
        } else {
            Color::DarkGray
        }));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(inner);

    if app.projects.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "\n  Nenhum projeto criado. Pressione [n] para criar um.",
                Style::default().fg(Color::DarkGray),
            )),
            chunks[0],
        );
    } else {
        let items: Vec<ListItem> = app
            .projects
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let selected = i == app.projects_cursor;
                let svc_count = app
                    .services
                    .iter()
                    .filter(|s| s.spec.project_id == p.id)
                    .count();
                let desc = p
                    .description
                    .as_deref()
                    .unwrap_or("sem descrição");
                let line = Line::from(vec![
                    Span::styled(
                        format!("  {:<30}", p.name),
                        Style::default()
                            .fg(if selected { Color::Black } else { Color::White })
                            .add_modifier(if selected { Modifier::BOLD } else { Modifier::empty() }),
                    ),
                    Span::styled(
                        format!("{:<40}", desc),
                        Style::default().fg(if selected {
                            Color::Black
                        } else {
                            Color::DarkGray
                        }),
                    ),
                    Span::styled(
                        format!("  {} serviço{}", svc_count, if svc_count == 1 { "" } else { "s" }),
                        Style::default().fg(if selected { Color::Black } else { Color::Cyan }),
                    ),
                ]);
                let style = if selected {
                    Style::default().bg(Color::Cyan)
                } else {
                    Style::default()
                };
                ListItem::new(line).style(style)
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(app.projects_cursor));
        f.render_stateful_widget(List::new(items), chunks[0], &mut list_state);
    }

    let can_delete = app
        .projects
        .get(app.projects_cursor)
        .map(|p| {
            app.services
                .iter()
                .filter(|s| s.spec.project_id == p.id)
                .count()
                == 0
        })
        .unwrap_or(false);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" [n]", Style::default().fg(Color::Cyan)),
            Span::styled(" novo  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
            Span::styled(" abrir  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "[D]",
                Style::default().fg(if can_delete {
                    Color::Red
                } else {
                    Color::DarkGray
                }),
            ),
            Span::styled(" remover  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Tab]", Style::default().fg(Color::DarkGray)),
            Span::styled(" sidebar", Style::default().fg(Color::DarkGray)),
        ])),
        chunks[1],
    );
}

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
        ProjectDetailTab::Settings,
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
        ProjectDetailTab::Settings => render_project_settings_tab(f, app, chunks[1]),
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

// ── Aba de configurações do projeto ──────────────────────────────────────────

pub fn render_project_settings_tab(f: &mut Frame, app: &App, area: Rect) {
    let project = match app.current_project() {
        Some(p) => p,
        None => return,
    };

    let outer = Block::default()
        .borders(Borders::ALL)
        .title(" Configurações do Projeto ")
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // padding
            Constraint::Length(1), // label Nome
            Constraint::Length(3), // input Nome
            Constraint::Length(1), // label Descrição
            Constraint::Length(3), // input Descrição
            Constraint::Length(1), // padding
            Constraint::Length(1), // botão Salvar
            Constraint::Length(1), // padding
            Constraint::Length(1), // separador danger zone
            Constraint::Length(1), // padding
            Constraint::Length(1), // aviso
            Constraint::Length(1), // botão Remover
            Constraint::Min(0),    // espaço
            Constraint::Length(1), // dicas
        ])
        .split(inner);

    let st = &app.project_settings;

    // ── Nome ──────────────────────────────────────────────────────────────────
    let name_focused = st.focused == ProjectSettingsField::Name;
    f.render_widget(
        Paragraph::new(Span::styled(
            "  Nome",
            Style::default().fg(if name_focused {
                Color::Cyan
            } else {
                Color::DarkGray
            }),
        )),
        chunks[1],
    );
    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if name_focused {
            Color::Cyan
        } else {
            Color::DarkGray
        }));
    let name_inner = name_block.inner(chunks[2]);
    f.render_widget(name_block, chunks[2]);
    f.render_widget(
        Paragraph::new(Span::styled(
            if name_focused {
                format!(" {}▌", st.name)
            } else {
                format!(" {}", st.name)
            },
            Style::default().fg(Color::White),
        )),
        name_inner,
    );

    // ── Descrição ─────────────────────────────────────────────────────────────
    let desc_focused = st.focused == ProjectSettingsField::Description;
    f.render_widget(
        Paragraph::new(Span::styled(
            "  Descrição  (opcional)",
            Style::default().fg(if desc_focused {
                Color::Cyan
            } else {
                Color::DarkGray
            }),
        )),
        chunks[3],
    );
    let desc_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if desc_focused {
            Color::Cyan
        } else {
            Color::DarkGray
        }));
    let desc_inner = desc_block.inner(chunks[4]);
    f.render_widget(desc_block, chunks[4]);
    let desc_content = if desc_focused {
        format!(" {}▌", st.description)
    } else if st.description.is_empty() {
        " opcional...".to_string()
    } else {
        format!(" {}", st.description)
    };
    f.render_widget(
        Paragraph::new(Span::styled(
            desc_content,
            if !desc_focused && st.description.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            },
        )),
        desc_inner,
    );

    // ── Botão Salvar ──────────────────────────────────────────────────────────
    let save_focused = st.focused == ProjectSettingsField::Save;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            if save_focused {
                Span::styled(
                    " [ Salvar Alterações ] ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(" [ Salvar Alterações ] ", Style::default().fg(Color::White))
            },
        ])),
        chunks[6],
    );

    // ── Danger zone ───────────────────────────────────────────────────────────
    f.render_widget(
        Paragraph::new(Span::styled(
            "─── Zona de Perigo ────────────────────────────────",
            Style::default().fg(Color::Red),
        )),
        chunks[8],
    );

    let service_count = app
        .services
        .iter()
        .filter(|s| s.spec.project_id == project.id)
        .count();
    let (warn_text, warn_color) = if service_count > 0 {
        (
            format!(
                "  Este projeto possui {} serviço{}. Remova-os primeiro.",
                service_count,
                if service_count == 1 { "" } else { "s" }
            ),
            Color::Yellow,
        )
    } else {
        (
            "  Nenhum serviço. O projeto pode ser removido.".to_string(),
            Color::DarkGray,
        )
    };
    f.render_widget(
        Paragraph::new(Span::styled(warn_text, Style::default().fg(warn_color))),
        chunks[10],
    );

    let del_focused = st.focused == ProjectSettingsField::Delete;
    let can_delete = service_count == 0;
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            if del_focused && can_delete {
                Span::styled(
                    " [ Remover Projeto ] ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Red)
                        .add_modifier(Modifier::BOLD),
                )
            } else if del_focused {
                Span::styled(
                    " [ Remover Projeto ] ",
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )
            } else {
                Span::styled(
                    " [ Remover Projeto ] ",
                    Style::default().fg(if can_delete {
                        Color::Red
                    } else {
                        Color::DarkGray
                    }),
                )
            },
        ])),
        chunks[11],
    );

    // ── Dicas ────────────────────────────────────────────────────────────────
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ↑↓/Tab", Style::default().fg(Color::Cyan)),
            Span::styled(" nav  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter/Space", Style::default().fg(Color::Cyan)),
            Span::styled(" ação  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[←→/1/2/3]", Style::default().fg(Color::DarkGray)),
            Span::styled(" abas", Style::default().fg(Color::DarkGray)),
        ])),
        chunks[13],
    );
}
