pub mod deploy_log;
pub mod metrics;
pub mod projects;
pub mod service_detail;
pub mod settings;
pub mod sidebar;

use crate::app::{App, DbKind, Focus, NewServiceStep, View};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

const SIDEBAR_WIDTH: u16 = 26;

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    render_titlebar(f, main_chunks[0], app);
    render_body(f, main_chunks[1], app);
    render_statusbar(f, main_chunks[2], app);

    if app.creating_project {
        render_new_project_popup(f, area, app);
    }

    if app.new_service.is_some() {
        render_new_service_popup(f, area, app);
    }

    if let Some(notif) = &app.notification {
        render_notification(f, area, &notif.message, notif.is_error);
    }
}

fn render_titlebar(f: &mut Frame, area: Rect, _app: &App) {
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" Rustploy v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw("  "),
        Span::styled("PaaS Engine", Style::default().fg(Color::DarkGray)),
    ]));
    f.render_widget(title, area);
}

fn render_body(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(SIDEBAR_WIDTH), Constraint::Min(0)])
        .split(area);

    sidebar::render(f, app, chunks[0]);
    render_content(f, app, chunks[1]);
}

fn render_content(f: &mut Frame, app: &App, area: Rect) {
    match &app.view {
        View::ProjectDetail => projects::render_project_detail(f, app, area),
        View::ServiceDetail => service_detail::render(f, app, area),
        View::HomeDeployments => render_home_placeholder(f, area, "Deployments", "Ver todos os deploys ativos em todos os projetos."),
        View::HomeMonitoring => metrics::render_global(f, app, area),
        View::HomeSchedules => render_home_placeholder(f, area, "Schedules", "Agendamentos de auto-deploy (v2)."),
        View::HomePingoraFs => render_home_placeholder(f, area, "Pingora File System", "Tabela de rotas ativa no Pingora."),
        View::HomeDocker => render_home_placeholder(f, area, "Docker", "Containers, redes e imagens gerenciadas."),
        View::HomeDeployEngine => render_home_placeholder(f, area, "Deploy Engine", "Estado interno do motor de deploy."),
        View::HomeRequests => render_home_placeholder(f, area, "Requests", "Log de requisições recebidas pelo Pingora."),
        View::SettingsWebServer
        | View::SettingsProfile
        | View::SettingsUsers
        | View::SettingsAuditLogs
        | View::SettingsSshKeys
        | View::SettingsTags
        | View::SettingsGit
        | View::SettingsRegistry
        | View::SettingsS3
        | View::SettingsCerts
        | View::SettingsSso => settings::render(f, app, area),
        View::Account => settings::render_account(f, app, area),
        View::Confirm { message, .. } => render_confirm_overlay(f, area, message),
    }
}

fn render_home_placeholder(f: &mut Frame, area: Rect, title: &str, desc: &str) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .border_style(Style::default().fg(Color::DarkGray));
    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(desc, Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(Span::styled("Em construção.", Style::default().fg(Color::Yellow))),
    ])
    .block(block);
    f.render_widget(text, area);
}

fn render_statusbar(f: &mut Frame, area: Rect, app: &App) {
    let hints = match (&app.focus, &app.view) {
        (Focus::Sidebar, _) => " [Tab] conteúdo  [↑↓] nav  [Enter] abrir  [q] quit",
        (Focus::Content, View::ProjectDetail) => {
            if app.service_filtering {
                " [Enter/Esc] sair do filtro  [Backspace] apagar"
            } else {
                " [/] filtrar  [n] novo  [Enter] abrir  [D] deletar  [Tab] sidebar"
            }
        }
        (Focus::Content, View::ServiceDetail) => {
            " [←→] abas  [1-6] aba direta  [↑↓] nav campo  [Esc] voltar  [Tab] sidebar"
        }
        _ => " [Tab] sidebar  [Esc] voltar",
    };

    let bar = Paragraph::new(hints).style(Style::default().fg(Color::DarkGray));
    f.render_widget(bar, area);
}

fn render_new_project_popup(f: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect_h(56, 14, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Novo Projeto ")
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // padding
            Constraint::Length(1), // label Nome
            Constraint::Length(3), // input Nome
            Constraint::Length(1), // label Descrição
            Constraint::Length(3), // input Descrição
            Constraint::Length(1), // padding
            Constraint::Length(1), // hints
        ])
        .split(inner);

    // Name
    let name_focused = app.new_proj_field == 0;
    f.render_widget(
        Paragraph::new(Span::styled(
            "  Nome",
            Style::default().fg(if name_focused { Color::Cyan } else { Color::DarkGray }),
        )),
        chunks[1],
    );
    let name_box = Block::default()
        .borders(Borders::ALL)
        .border_style(if name_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });
    let name_inner = name_box.inner(chunks[2]);
    f.render_widget(name_box, chunks[2]);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                if name_focused {
                    format!("{}▌", app.new_proj_name)
                } else {
                    app.new_proj_name.clone()
                },
                Style::default().fg(Color::White),
            ),
        ])),
        name_inner,
    );

    // Description
    let desc_focused = app.new_proj_field == 1;
    f.render_widget(
        Paragraph::new(Span::styled(
            "  Descrição  (opcional)",
            Style::default().fg(if desc_focused { Color::Cyan } else { Color::DarkGray }),
        )),
        chunks[3],
    );
    let desc_box = Block::default()
        .borders(Borders::ALL)
        .border_style(if desc_focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        });
    let desc_inner = desc_box.inner(chunks[4]);
    f.render_widget(desc_box, chunks[4]);
    let desc_content = if desc_focused {
        format!(" {}▌", app.new_proj_desc)
    } else if app.new_proj_desc.is_empty() {
        " opcional...".to_string()
    } else {
        format!(" {}", app.new_proj_desc)
    };
    f.render_widget(
        Paragraph::new(Span::styled(
            desc_content,
            if desc_focused {
                Style::default().fg(Color::White)
            } else if app.new_proj_desc.is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default().fg(Color::White)
            },
        )),
        desc_inner,
    );

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Tab", Style::default().fg(Color::Cyan)),
            Span::styled(" alternar  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::styled(" criar  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::styled(" cancelar", Style::default().fg(Color::DarkGray)),
        ])),
        chunks[6],
    );
}

fn render_confirm_overlay(f: &mut Frame, area: Rect, message: &str) {
    let popup = centered_rect(60, 20, area);
    f.render_widget(Clear, popup);
    let block = Block::default()
        .title(" Confirmar ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(message),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [y] Sim  ", Style::default().fg(Color::Green)),
            Span::styled("[n] Não", Style::default().fg(Color::Red)),
        ]),
    ])
    .block(block);
    f.render_widget(text, popup);
}

fn render_notification(f: &mut Frame, area: Rect, message: &str, is_error: bool) {
    let width = (message.len() as u16 + 4).min(area.width.saturating_sub(2));
    let notif_area = Rect {
        x: area.width.saturating_sub(width + 1),
        y: area.height.saturating_sub(3),
        width,
        height: 3,
    };
    let color = if is_error { Color::Red } else { Color::Green };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color));
    let text = Paragraph::new(message).block(block).style(Style::default().fg(color));
    f.render_widget(Clear, notif_area);
    f.render_widget(text, notif_area);
}

/// Centers a popup with a fixed height and percentage width.
fn centered_rect_h(percent_x: u16, height: u16, r: Rect) -> Rect {
    let y_offset = r.height.saturating_sub(height) / 2;
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(y_offset),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// ── New service popup ─────────────────────────────────────────────────────────

fn render_new_service_popup(f: &mut Frame, area: Rect, app: &App) {
    let state = match &app.new_service {
        Some(s) => s,
        None => return,
    };
    match &state.step {
        NewServiceStep::PickType => render_ns_pick_type(f, area, app),
        NewServiceStep::PickDbType => render_ns_pick_db(f, area, app),
        NewServiceStep::ApplicationForm => render_ns_app_form(f, area, app),
        NewServiceStep::DatabaseForm => render_ns_db_form(f, area, app),
    }
}

fn render_ns_pick_type(f: &mut Frame, area: Rect, app: &App) {
    let state = app.new_service.as_ref().unwrap();
    let popup = centered_rect_h(62, 18, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Novo Serviço ")
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // padding
            Constraint::Length(5), // row 1 (App, Database)
            Constraint::Length(1), // gap
            Constraint::Length(5), // row 2 (Compose, Template)
            Constraint::Length(1), // gap
            Constraint::Length(1), // hints
            Constraint::Min(0),
        ])
        .split(inner);

    let kinds = [
        (0usize, "Application"),
        (1, "Database"),
        (2, "Compose"),
        (3, "Template"),
    ];
    let descriptions = [
        "Web app via Git ou imagem",
        "Banco de dados gerenciado",
        "Stack Docker Compose",
        "A partir de preset",
    ];

    for row in 0..2 {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(rows[1 + row * 2]);

        for col in 0..2 {
            let idx = row * 2 + col;
            let selected = state.type_cursor == idx;
            let (border_color, title_color) = if selected {
                (Color::Cyan, Color::Cyan)
            } else {
                (Color::DarkGray, Color::White)
            };
            let card = Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", kinds[idx].1))
                .title_style(
                    Style::default().fg(title_color).add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
                )
                .border_style(Style::default().fg(border_color));
            let card_inner = card.inner(cols[1 + col * 2]);
            f.render_widget(card, cols[1 + col * 2]);
            let desc_color = if selected { Color::White } else { Color::DarkGray };
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    format!(" {}", descriptions[idx]),
                    Style::default().fg(desc_color),
                )))
                .centered(),
                card_inner,
            );
        }
    }

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ←→↑↓", Style::default().fg(Color::Cyan)),
            Span::styled(" nav  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::styled(" selecionar  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::styled(" cancelar", Style::default().fg(Color::DarkGray)),
        ])),
        rows[5],
    );
}

fn render_ns_pick_db(f: &mut Frame, area: Rect, app: &App) {
    let state = app.new_service.as_ref().unwrap();
    let popup = centered_rect_h(56, 22, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Banco de Dados ")
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let n = DbKind::ALL.len();
    let mut constraints = vec![Constraint::Length(1)]; // top padding
    for _ in 0..n {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Min(0)); // spacing
    constraints.push(Constraint::Length(1)); // hints
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, db) in DbKind::ALL.iter().enumerate() {
        let selected = state.db_cursor == i;
        let (border_color, text_color) = if selected {
            (Color::Cyan, Color::Cyan)
        } else {
            (Color::DarkGray, Color::White)
        };
        let item_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));
        let item_inner = item_block.inner(rows[1 + i]);
        f.render_widget(item_block, rows[1 + i]);
        let marker = if selected { "▸ " } else { "  " };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("{}{}", marker, db.label()),
                Style::default().fg(text_color).add_modifier(if selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ))),
            item_inner,
        );
    }

    let hints_row = rows[1 + n + 1];
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ↑↓", Style::default().fg(Color::Cyan)),
            Span::styled(" nav  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::styled(" selecionar  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::styled(" voltar", Style::default().fg(Color::DarkGray)),
        ])),
        hints_row,
    );
}

// Application form: 3 fields with bordered boxes (label above box, new-project style).
fn render_ns_app_form(f: &mut Frame, area: Rect, app: &App) {
    let state = app.new_service.as_ref().unwrap();
    // 2 border + 1 pad + 3×(1 label + 3 box) + 1 pad + 1 btn + 1 hints = 20
    let popup = centered_rect_h(60, 20, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Nova Application ")
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // padding
            Constraint::Length(1), // label Nome
            Constraint::Length(3), // input Nome
            Constraint::Length(1), // label App Name
            Constraint::Length(3), // input App Name
            Constraint::Length(1), // label Descrição
            Constraint::Length(3), // input Descrição
            Constraint::Length(1), // padding
            Constraint::Length(1), // button
            Constraint::Length(1), // hints
            Constraint::Min(0),
        ])
        .split(inner);

    render_ns_labeled_box(f, chunks[1], chunks[2], "  Nome", &state.name, state.focused_field == 0);
    render_ns_labeled_box(f, chunks[3], chunks[4], "  App Name", &state.app_name, state.focused_field == 1);
    render_ns_labeled_box(f, chunks[5], chunks[6], "  Descrição  (opcional)", &state.description, state.focused_field == 2);

    let btn_focused = state.is_button();
    let btn = if btn_focused {
        Span::styled(
            " [ Criar Application ] ",
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(" [ Criar Application ] ", Style::default().fg(Color::White))
    };
    f.render_widget(Paragraph::new(Line::from(vec![Span::raw("  "), btn])), chunks[8]);
    render_ns_hints(f, chunks[9]);
}

// Bordered-box DB form with scroll (5 fields visible at a time).
fn render_ns_db_form(f: &mut Frame, area: Rect, app: &App) {
    let state = app.new_service.as_ref().unwrap();
    let db = match state.db_kind {
        Some(d) => d,
        None => return,
    };

    // Fields: (label, value_str, field_idx, is_checkbox)
    let all_fields: Vec<(&str, String, usize, bool)> = match db {
        DbKind::Postgres => vec![
            ("Nome", state.name.clone(), 0, false),
            ("App Name", state.app_name.clone(), 1, false),
            ("Descrição", state.description.clone(), 2, false),
            ("Database Name", state.db_name.clone(), 3, false),
            ("User", state.db_user.clone(), 4, false),
            ("Password", state.db_password.clone(), 5, false),
            ("Docker Image", state.docker_image.clone(), 6, false),
        ],
        DbKind::MongoDB => vec![
            ("Nome", state.name.clone(), 0, false),
            ("App Name", state.app_name.clone(), 1, false),
            ("Descrição", state.description.clone(), 2, false),
            ("User", state.db_user.clone(), 3, false),
            ("Password", state.db_password.clone(), 4, false),
            ("Docker Image", state.docker_image.clone(), 5, false),
            ("Use Replica Sets", String::new(), 6, true),
        ],
        DbKind::MariaDB | DbKind::MySQL => vec![
            ("Nome", state.name.clone(), 0, false),
            ("App Name", state.app_name.clone(), 1, false),
            ("Descrição", state.description.clone(), 2, false),
            ("Database Name", state.db_name.clone(), 3, false),
            ("User", state.db_user.clone(), 4, false),
            ("Password", state.db_password.clone(), 5, false),
            ("Root Password", state.db_root_password.clone(), 6, false),
            ("Docker Image", state.docker_image.clone(), 7, false),
        ],
        DbKind::Redis => vec![
            ("Nome", state.name.clone(), 0, false),
            ("App Name", state.app_name.clone(), 1, false),
            ("Descrição", state.description.clone(), 2, false),
            ("Password", state.db_password.clone(), 3, false),
            ("Docker Image", state.docker_image.clone(), 4, false),
        ],
    };

    // Fixed popup: 4 visible fields (3 rows each = 12) + 1 pad + 1 sep + 1 btn + 1 hints = 16 inner + 2 border = 18
    const VISIBLE: usize = 4;
    let popup_h: u16 = 18;
    let popup = centered_rect_h(64, popup_h.min(area.height), area);
    f.render_widget(Clear, popup);

    let title = format!(" Nova {} ", db.label());
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = outer_block.inner(popup);
    f.render_widget(outer_block, popup);

    // Layout: pad + 4 fields (3 rows each) + sep + btn + hints
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // padding
            Constraint::Length(12), // 4 × 3-row fields
            Constraint::Length(1),  // separator
            Constraint::Length(1),  // button
            Constraint::Length(1),  // hints
        ])
        .split(inner);

    let field_area = chunks[1];
    let total = all_fields.len();
    let scroll = state.form_scroll;

    // Each field slot: 3 rows
    let visible_slice = &all_fields[scroll..total.min(scroll + VISIBLE)];
    let mut field_chunks_constraints: Vec<Constraint> =
        visible_slice.iter().map(|_| Constraint::Length(3)).collect();
    // Fill remaining space if fewer than VISIBLE fields
    field_chunks_constraints.push(Constraint::Min(0));
    let field_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(field_chunks_constraints)
        .split(field_area);

    for (slot, (label, value, field_idx, is_cb)) in visible_slice.iter().enumerate() {
        let focused = state.focused_field == *field_idx;
        if *is_cb {
            // MongoDB checkbox field
            let checked = state.use_replica_sets;
            let content = if checked {
                "[x] Sim   (Espaço para alternar)".to_string()
            } else {
                "[ ] Não   (Espaço para alternar)".to_string()
            };
            render_ns_field_box(f, field_rows[slot], label, &content, focused);
        } else {
            render_ns_field_box(f, field_rows[slot], label, value, focused);
        }
    }

    // Scroll indicators inside the separator row
    let scroll_text = if total > VISIBLE {
        let above = scroll > 0;
        let below = scroll + VISIBLE < total;
        format!(
            " {} campo {}/{}  {}",
            if above { "▲" } else { " " },
            state.focused_field + 1,
            total + 1, // +1 for button
            if below { "▼ mais ↓" } else { "" }
        )
    } else {
        String::new()
    };
    f.render_widget(
        Paragraph::new(Span::styled(scroll_text, Style::default().fg(Color::DarkGray))),
        chunks[2],
    );

    // Button
    let btn_focused = state.is_button();
    let btn_label = format!(" [ Criar {} ] ", db.label());
    let btn = if btn_focused {
        Span::styled(
            btn_label,
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(btn_label, Style::default().fg(Color::White))
    };
    f.render_widget(Paragraph::new(Line::from(vec![Span::raw("  "), btn])), chunks[3]);

    render_ns_hints(f, chunks[4]);
}

// 3-row bordered box with the label embedded in the border title.
fn render_ns_field_box(f: &mut Frame, area: Rect, label: &str, value: &str, focused: bool) {
    let (border_color, title_color) = if focused {
        (Color::Cyan, Color::Cyan)
    } else {
        (Color::DarkGray, Color::DarkGray)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {} ", label))
        .title_style(Style::default().fg(title_color))
        .border_style(Style::default().fg(border_color));
    let inner_area = block.inner(area);
    f.render_widget(block, area);
    let cursor = if focused { "▌" } else { "" };
    let val_style = if focused {
        Style::default().fg(Color::White)
    } else if value.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Gray)
    };
    f.render_widget(
        Paragraph::new(Span::styled(format!(" {value}{cursor}"), val_style)),
        inner_area,
    );
}

// Renders a label line + 3-row bordered input box (new-project style, label above box).
fn render_ns_labeled_box(
    f: &mut Frame,
    label_area: Rect,
    box_area: Rect,
    label: &str,
    value: &str,
    focused: bool,
) {
    f.render_widget(
        Paragraph::new(Span::styled(
            label,
            Style::default().fg(if focused { Color::Cyan } else { Color::DarkGray }),
        )),
        label_area,
    );
    let border_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let input_block = Block::default().borders(Borders::ALL).border_style(border_style);
    let input_inner = input_block.inner(box_area);
    f.render_widget(input_block, box_area);
    let cursor = if focused { "▌" } else { "" };
    let display = if !focused && value.is_empty() && label.contains("opcional") {
        Span::styled(" opcional...", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(format!(" {value}{cursor}"), Style::default().fg(Color::White))
    };
    f.render_widget(Paragraph::new(display), input_inner);
}

fn render_ns_hints(f: &mut Frame, area: Rect) {
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" ↑↓/Tab", Style::default().fg(Color::Cyan)),
            Span::styled(" nav  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Enter", Style::default().fg(Color::Cyan)),
            Span::styled(" próximo/criar  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::styled(" voltar", Style::default().fg(Color::DarkGray)),
        ])),
        area,
    );
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
