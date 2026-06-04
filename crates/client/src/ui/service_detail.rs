use crate::app::{AdvancedField, App, DbKind, EnvEditField, GeneralTabField, HcField, ServiceTab};
use shared::ServiceSource;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use shared::EnvVarValue;

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let svc = match app.current_active_service() {
        Some(s) => s,
        None => {
            f.render_widget(Paragraph::new("Nenhum serviço selecionado."), area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    render_tab_bar(f, app, chunks[0], &svc.spec.name);
    render_tab_content(f, app, chunks[1]);
}

fn render_tab_bar(f: &mut Frame, app: &App, area: Rect, svc_name: &str) {
    use shared::ServiceStatus;

    let status_style = app.current_active_service().map(|s| match &s.status {
        ServiceStatus::Running => Style::default().fg(Color::Green),
        ServiceStatus::Stopping => Style::default().fg(Color::Yellow),
        ServiceStatus::Stopped => Style::default().fg(Color::DarkGray),
        ServiceStatus::Deploying => Style::default().fg(Color::Yellow),
        ServiceStatus::Degraded => Style::default().fg(Color::Red),
        ServiceStatus::Error(_) => Style::default().fg(Color::Red),
    });
    let status_label = app.current_active_service().map(|s| match &s.status {
        ServiceStatus::Running => " ● Running ",
        ServiceStatus::Stopping => " ◌ Stopping ",
        ServiceStatus::Stopped => " ○ Stopped ",
        ServiceStatus::Deploying => " ◌ Deploying ",
        ServiceStatus::Degraded => " ◐ Degraded ",
        ServiceStatus::Error(_) => " ✕ Error ",
    });

    let tabs = app.visible_service_tabs();
    let mut spans: Vec<Span> = vec![
        Span::styled(
            format!(" {svc_name} "),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(status_label.unwrap_or(""), status_style.unwrap_or_default()),
        Span::raw(" "),
    ];

    for tab in tabs {
        let active = tab == &app.service_tab;
        let style = if active {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!(" {} ", tab.label()), style));
        spans.push(Span::raw(" "));
    }

    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(Color::DarkGray));
    let header = Paragraph::new(Line::from(spans)).block(block);
    f.render_widget(header, area);
}

fn render_tab_content(f: &mut Frame, app: &App, area: Rect) {
    match app.service_tab {
        ServiceTab::General => render_general_tab(f, app, area),
        ServiceTab::Connection => render_connection_tab(f, app, area),
        ServiceTab::Environment => render_env_tab(f, app, area),
        ServiceTab::Domains => render_domains_tab(f, app, area),
        ServiceTab::Deployments => render_deployments_tab(f, app, area),
        ServiceTab::Healthcheck => render_healthcheck_tab(f, app, area),
        ServiceTab::Logs => render_logs_tab(f, app, area),
        ServiceTab::Patches => render_patches_tab(f, area),
        ServiceTab::Advanced => render_advanced_tab(f, app, area),
    }
}

// ─── General Tab ─────────────────────────────────────────────────────────────

fn render_general_tab(f: &mut Frame, app: &App, area: Rect) {
    let is_compose = app
        .current_active_service()
        .map(|s| matches!(s.spec.source, ServiceSource::Compose(_)))
        .unwrap_or(false);

    if is_compose {
        render_compose_general_tab(f, app, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // spacing           [0]
            Constraint::Length(1), // action buttons    [1]
            Constraint::Length(1), // spacing           [2]
            Constraint::Length(1), // Provider header   [3]
            Constraint::Length(1), // Repo URL          [4]
            Constraint::Length(1), // Branch            [5]
            Constraint::Length(1), // Credentials       [6]
            Constraint::Length(1), // Build Path        [7]
            Constraint::Length(1), // Watch Paths       [8]
            Constraint::Length(1), // Submodules        [9]
            Constraint::Length(1), // Port              [10]
            Constraint::Length(1), // spacing           [11]
            Constraint::Length(1), // SSH + Save (prov) [12]
            Constraint::Length(1), // spacing           [13]
            Constraint::Length(1), // Build Type header [14]
            Constraint::Length(1), // Docker File       [15]
            Constraint::Length(1), // Context Path      [16]
            Constraint::Length(1), // Build Stage       [17]
            Constraint::Length(1), // spacing           [18]
            Constraint::Length(1), // Build Save button [19]
            Constraint::Min(0),
        ])
        .split(area);

    let gt = &app.general_tab;

    // Action buttons row
    let btn_row = Line::from(vec![
        Span::raw("  "),
        btn_span("[ Deploy ]", gt.focused_field == GeneralTabField::BtnDeploy),
        Span::raw("  "),
        btn_span("[ Reload ]", gt.focused_field == GeneralTabField::BtnReload),
        Span::raw("  "),
        btn_span(
            "[ Rebuild ]",
            gt.focused_field == GeneralTabField::BtnRebuild,
        ),
        Span::raw("  "),
        btn_span("[ Stop ]", gt.focused_field == GeneralTabField::BtnStop),
    ]);
    f.render_widget(Paragraph::new(btn_row), chunks[1]);

    // Provider header
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── Provider: Git ──────────────────────────────────────────",
            Style::default().fg(Color::Yellow),
        ))),
        chunks[3],
    );

    render_form_row(
        f,
        chunks[4],
        "Repository URL",
        &gt.repo_url,
        gt.focused_field == GeneralTabField::RepoUrl,
    );
    render_form_row(
        f,
        chunks[5],
        "Branch",
        &gt.branch,
        gt.focused_field == GeneralTabField::Branch,
    );
    render_form_row(
        f,
        chunks[6],
        "Credentials (secret)",
        &gt.credentials,
        gt.focused_field == GeneralTabField::Credentials,
    );
    render_form_row(
        f,
        chunks[7],
        "Build Path",
        &gt.build_path,
        gt.focused_field == GeneralTabField::BuildPath,
    );
    render_form_row(
        f,
        chunks[8],
        "Watch Paths",
        &gt.watch_paths,
        gt.focused_field == GeneralTabField::WatchPaths,
    );

    // Submodules toggle
    let sub_label_style = Style::default().fg(Color::DarkGray);
    let sub_val_style = if gt.focused_field == GeneralTabField::Submodules {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("  {:<22}", "Enable Submodules"), sub_label_style),
            Span::styled(
                if gt.submodules { "[ Yes ]" } else { "[ No  ]" },
                sub_val_style,
            ),
        ])),
        chunks[9],
    );

    render_form_row(
        f,
        chunks[10],
        "Port",
        &gt.port,
        gt.focused_field == GeneralTabField::Port,
    );

    // SSH Keys + Provider Save buttons
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            btn_span(
                "[ Add SSH Keys ]",
                gt.focused_field == GeneralTabField::AddSshKeys,
            ),
            Span::raw("   "),
            btn_span(
                "[ Save ]",
                gt.focused_field == GeneralTabField::ProviderSave,
            ),
        ])),
        chunks[12],
    );

    // Build Type header
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── Build Type: Dockerfile ─────────────────────────────────",
            Style::default().fg(Color::Yellow),
        ))),
        chunks[14],
    );

    render_form_row(
        f,
        chunks[15],
        "Docker File",
        &gt.dockerfile,
        gt.focused_field == GeneralTabField::DockerFile,
    );
    render_form_row(
        f,
        chunks[16],
        "Docker Context Path",
        &gt.context_path,
        gt.focused_field == GeneralTabField::DockerContextPath,
    );
    render_form_row(
        f,
        chunks[17],
        "Docker Build Stage",
        &gt.build_stage,
        gt.focused_field == GeneralTabField::DockerBuildStage,
    );

    // Build Save button
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            btn_span("[ Save ]", gt.focused_field == GeneralTabField::BuildSave),
        ])),
        chunks[19],
    );
}

fn render_compose_general_tab(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // spacing        [0]
            Constraint::Length(1), // action buttons [1]
            Constraint::Length(1), // spacing        [2]
            Constraint::Length(1), // editor header  [3]
            Constraint::Min(0),    // textarea       [4]
            Constraint::Length(1), // hints          [5]
        ])
        .split(area);

    let gt = &app.general_tab;

    let btn_row = Line::from(vec![
        Span::raw("  "),
        btn_span("[ Deploy ]", gt.focused_field == GeneralTabField::BtnDeploy),
        Span::raw("  "),
        btn_span("[ Reload ]", gt.focused_field == GeneralTabField::BtnReload),
        Span::raw("  "),
        btn_span(
            "[ Rebuild ]",
            gt.focused_field == GeneralTabField::BtnRebuild,
        ),
        Span::raw("  "),
        btn_span("[ Stop ]", gt.focused_field == GeneralTabField::BtnStop),
    ]);
    f.render_widget(Paragraph::new(btn_row), chunks[1]);

    let (header_text, header_color) = if app.compose_tab.editing {
        (
            "── Compose YAML  [Ctrl+S] salvar  [Esc] sair do editor ──────",
            Color::Cyan,
        )
    } else {
        (
            "── Compose YAML  [Enter] editar ──────────────────────────────",
            Color::Yellow,
        )
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            header_text,
            Style::default().fg(header_color),
        ))),
        chunks[3],
    );

    let border_color = if app.compose_tab.editing {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let editor_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));
    let editor_inner = editor_block.inner(chunks[4]);
    f.render_widget(editor_block, chunks[4]);
    f.render_widget(app.compose_tab.textarea.widget(), editor_inner);

    let hints = if app.compose_tab.editing {
        Line::from(vec![
            Span::styled(" Ctrl+S", Style::default().fg(Color::Cyan)),
            Span::styled(" salvar  ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Cyan)),
            Span::styled(" sair do editor", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        let lines = app.compose_tab.textarea.lines().len();
        let is_empty = app.compose_tab.content().trim().is_empty();
        let info = if is_empty {
            " (vazio — não é possível fazer deploy)".to_string()
        } else {
            format!(" ({lines} linhas)")
        };
        Line::from(vec![
            Span::styled(" [Enter]", Style::default().fg(Color::Cyan)),
            Span::styled(" editar  ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                info,
                Style::default().fg(if is_empty {
                    Color::Red
                } else {
                    Color::DarkGray
                }),
            ),
        ])
    };
    f.render_widget(Paragraph::new(hints), chunks[5]);
}

// ─── Healthcheck Tab ─────────────────────────────────────────────────────────

fn render_healthcheck_tab(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // spacing          [0]
            Constraint::Length(1), // header           [1]
            Constraint::Length(1), // Kind             [2]
            Constraint::Length(1), // spacing          [3]
            Constraint::Length(1), // HTTP section hdr [4]
            Constraint::Length(1), // HTTP Path        [5]
            Constraint::Length(1), // Expected Status  [6]
            Constraint::Length(1), // spacing          [7]
            Constraint::Length(1), // Timing header    [8]
            Constraint::Length(1), // Interval         [9]
            Constraint::Length(1), // Timeout          [10]
            Constraint::Length(1), // Retries          [11]
            Constraint::Length(1), // Start Period     [12]
            Constraint::Length(1), // spacing          [13]
            Constraint::Length(1), // Save             [14]
            Constraint::Min(0),    //                  [15]
        ])
        .split(area);

    let hc = &app.healthcheck_tab;
    let http_active = hc.kind == "Http";

    // Kind toggle
    let kind_display = match hc.kind.as_str() {
        "Http" => "[ Http          ]",
        "DockerNative" => "[ DockerNative  ]",
        _ => "[ Tcp           ]",
    };
    let kind_val_style = if hc.focused == HcField::Kind {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("  {:<22}", "Kind"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(kind_display, kind_val_style),
            Span::styled("  [Space] to cycle", Style::default().fg(Color::DarkGray)),
        ])),
        chunks[2],
    );

    // HTTP sub-section header
    let http_header_style = if http_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── HTTP options ───────────────────────────────────────────",
            http_header_style,
        ))),
        chunks[4],
    );

    // HTTP-only fields (dimmed when kind != Http)
    let dim = |focused: bool| {
        if !http_active {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM)
        } else if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::White)
        }
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("  {:<22}", "HTTP Path"),
                if http_active {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM)
                },
            ),
            Span::styled(
                format!(
                    "{}{}",
                    hc.http_path,
                    if hc.focused == HcField::HttpPath && http_active {
                        "▌"
                    } else {
                        ""
                    }
                ),
                dim(hc.focused == HcField::HttpPath),
            ),
        ])),
        chunks[5],
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("  {:<22}", "Expected Status"),
                if http_active {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM)
                },
            ),
            Span::styled(
                format!(
                    "{}{}",
                    hc.expected_status,
                    if hc.focused == HcField::ExpectedStatus && http_active {
                        "▌"
                    } else {
                        ""
                    }
                ),
                dim(hc.focused == HcField::ExpectedStatus),
            ),
        ])),
        chunks[6],
    );

    // Timing section header
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── Timing ─────────────────────────────────────────────────",
            Style::default().fg(Color::Yellow),
        ))),
        chunks[8],
    );

    render_form_row(
        f,
        chunks[9],
        "Interval (s)",
        &hc.interval,
        hc.focused == HcField::Interval,
    );
    render_form_row(
        f,
        chunks[10],
        "Timeout (s)",
        &hc.timeout,
        hc.focused == HcField::Timeout,
    );
    render_form_row(
        f,
        chunks[11],
        "Retries",
        &hc.retries,
        hc.focused == HcField::Retries,
    );
    render_form_row(
        f,
        chunks[12],
        "Start Period (s)",
        &hc.start_period,
        hc.focused == HcField::StartPeriod,
    );

    // Save button
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            btn_span("[ Save ]", hc.focused == HcField::Save),
        ])),
        chunks[14],
    );
}

fn render_form_row(f: &mut Frame, area: Rect, label: &str, value: &str, focused: bool) {
    let cursor = if focused { "▌" } else { "" };
    let val_style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("  {:<22}", label),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(format!("{value}{cursor}"), val_style),
        ])),
        area,
    );
}

fn btn_span(label: &str, focused: bool) -> Span<'static> {
    if focused {
        Span::styled(
            label.to_string(),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(label.to_string(), Style::default().fg(Color::White))
    }
}

// ─── Environment Tab ─────────────────────────────────────────────────────────

fn render_env_tab(f: &mut Frame, app: &App, area: Rect) {
    let svc = match app.current_active_service() {
        Some(s) => s,
        None => return,
    };

    if app.env_tab.editing {
        render_env_edit_popup(f, app, area);
        return;
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Environment Variables — [n] add  [e] edit  [D] delete ")
        .border_style(Style::default().fg(Color::DarkGray));

    let items: Vec<ListItem> = svc
        .spec
        .env_vars
        .iter()
        .enumerate()
        .map(|(i, ev)| {
            let selected = i == app.env_tab.cursor;
            let val_str = match &ev.value {
                EnvVarValue::Plain(v) => {
                    if v.len() > 30 {
                        format!("{}...", &v[..30])
                    } else {
                        v.clone()
                    }
                }
                EnvVarValue::Secret(s) => format!("<secret:{s}>"),
            };
            let style = if selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(
                format!("  {:<24}= {}", ev.key, val_str),
                style,
            )))
        })
        .collect();

    if svc.spec.env_vars.is_empty() {
        let p = Paragraph::new(Line::from(Span::styled(
            "  Nenhuma variável. Pressione [n] para adicionar.",
            Style::default().fg(Color::DarkGray),
        )))
        .block(block);
        f.render_widget(p, area);
    } else {
        let list = List::new(items).block(block);
        let mut state = ListState::default();
        state.select(Some(app.env_tab.cursor));
        f.render_stateful_widget(list, area, &mut state);
    }
}

fn render_env_edit_popup(f: &mut Frame, app: &App, area: Rect) {
    use crate::ui::centered_rect;
    let popup = centered_rect(60, 30, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Editar Variável ")
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    let key_focused = app.env_tab.edit_field == EnvEditField::Key;
    let val_focused = app.env_tab.edit_field == EnvEditField::Value;

    let key_style = if key_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };
    let val_style = if val_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::White)
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Chave: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}▌", app.env_tab.edit_key), key_style),
        ])),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(" Valor: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{}▌", app.env_tab.edit_value), val_style),
        ])),
        chunks[2],
    );
    f.render_widget(
        Paragraph::new(" [Tab] campo  [Enter] salvar  [Esc] cancelar")
            .style(Style::default().fg(Color::DarkGray)),
        chunks[3],
    );
}

// ─── Domains Tab ──────────────────────────────────────────────────────────────

fn render_domains_tab(f: &mut Frame, app: &App, area: Rect) {
    use crate::app::DomainsField;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // spacing            [0]
            Constraint::Length(1), // header             [1]
            Constraint::Length(1), // spacing            [2]
            Constraint::Length(1), // Domain             [3]
            Constraint::Length(1), // Host Port          [4]
            Constraint::Length(1), // spacing            [5]
            Constraint::Length(1), // Save               [6]
            Constraint::Min(0),
        ])
        .split(area);

    let dt = &app.domains_tab;

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── Roteamento ─────────────────────────────────────────────",
            Style::default().fg(Color::Yellow),
        ))),
        chunks[1],
    );

    let domain_val = format!(
        "{}{}",
        dt.domain,
        if dt.focused == DomainsField::Domain {
            "▌"
        } else {
            ""
        }
    );
    let hp_val = format!(
        "{}{}",
        dt.host_port,
        if dt.focused == DomainsField::HostPort {
            "▌"
        } else {
            ""
        }
    );

    let focused_style = Style::default().fg(Color::Cyan);
    let normal_style = Style::default().fg(Color::White);
    let label_style = Style::default().fg(Color::DarkGray);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("  {:<22}", "Domínio"), label_style),
            Span::styled(
                domain_val,
                if dt.focused == DomainsField::Domain {
                    focused_style
                } else {
                    normal_style
                },
            ),
        ])),
        chunks[3],
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("  {:<22}", "Porta externa"), label_style),
            Span::styled(
                if dt.host_port.is_empty() && dt.focused != DomainsField::HostPort {
                    format!(
                        "(padrão: {})",
                        app.current_active_service()
                            .map(|s| s.spec.port)
                            .unwrap_or(0)
                    )
                } else {
                    hp_val
                },
                if dt.focused == DomainsField::HostPort {
                    focused_style
                } else {
                    if dt.host_port.is_empty() {
                        Style::default().fg(Color::DarkGray)
                    } else {
                        normal_style
                    }
                },
            ),
        ])),
        chunks[4],
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            btn_span("[ Save ]", dt.focused == DomainsField::Save),
        ])),
        chunks[6],
    );
}

// ─── Deployments Tab ──────────────────────────────────────────────────────────

fn render_deployments_tab(f: &mut Frame, app: &App, area: Rect) {
    let has_webhook = app.webhook_url.is_some();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Deployments — [↑↓] navegar  [[] log ▲  []] log ▼  [g/G] início/fim  [r] rollback ")
        .border_style(Style::default().fg(Color::DarkGray));

    let deps = &app.service_deployments;

    if deps.is_empty() {
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Nenhum deployment recente.",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Pressione [d] na aba General para iniciar um deploy.",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(block);
        f.render_widget(p, area);
        return;
    }

    let webhook_height = if has_webhook { 3u16 } else { 0u16 };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(5),
            Constraint::Length(webhook_height),
            Constraint::Min(3),
        ])
        .split(block.inner(area));
    f.render_widget(block, area);

    // ── Deployment list ───────────────────────────────────────────────────────

    let cursor = app.deployment_cursor.min(deps.len().saturating_sub(1));
    let items: Vec<ListItem> = deps
        .iter()
        .enumerate()
        .map(|(i, dep)| {
            let duration = dep
                .finished_at
                .map(|fin| (fin - dep.started_at).num_seconds())
                .map(|s| format!("{s}s"))
                .unwrap_or_else(|| "em andamento".into());
            let state_color = match dep.state {
                shared::DeployState::Live => Color::Green,
                shared::DeployState::Stopped => Color::DarkGray,
                shared::DeployState::Failed | shared::DeployState::RollingBack => Color::Red,
                _ => Color::Yellow,
            };
            let selected = i == cursor;
            let line = Line::from(vec![
                Span::raw(if selected { "▶ " } else { "  " }),
                Span::styled(
                    &dep.id[..dep.id.len().min(12)],
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw("  "),
                Span::styled(dep.state.label(), Style::default().fg(state_color)),
                Span::raw("  "),
                Span::styled(duration, Style::default().fg(Color::DarkGray)),
            ]);
            ListItem::new(line).style(if selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            })
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(cursor));
    f.render_stateful_widget(
        List::new(items).highlight_style(Style::default().add_modifier(Modifier::BOLD)),
        chunks[0],
        &mut list_state,
    );

    let Some(dep) = deps.get(cursor) else { return };

    // ── Detail ────────────────────────────────────────────────────────────────

    let chain = {
        let states: Vec<&str> = dep.states_log.iter().map(|t| t.to.label()).collect();
        if states.is_empty() {
            dep.state.label().to_string()
        } else {
            states.join(" → ")
        }
    };
    let detail = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("  ID:      ", Style::default().fg(Color::DarkGray)),
            Span::styled(&dep.id, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  Imagem:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(&dep.image, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Caminho: ", Style::default().fg(Color::DarkGray)),
            Span::styled(chain, Style::default().fg(Color::DarkGray)),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(detail, chunks[1]);

    // ── Webhook URL ───────────────────────────────────────────────────────────

    if let Some(url) = &app.webhook_url {
        let webhook_block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = webhook_block.inner(chunks[2]);
        f.render_widget(webhook_block, chunks[2]);

        // Trunca a URL para caber na largura disponível (desconta "  Webhook:  " = 12 chars)
        let prefix_len = 12usize;
        let max_url = inner.width.saturating_sub(prefix_len as u16) as usize;
        let display_url = if url.len() > max_url && max_url > 1 {
            format!("{}…", &url[..max_url.saturating_sub(1)])
        } else {
            url.clone()
        };

        f.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled("  Webhook:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(display_url, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("  ", Style::default()),
                    Span::styled("[c]", Style::default().fg(Color::Yellow)),
                    Span::styled(" copiar URL  ", Style::default().fg(Color::DarkGray)),
                    Span::styled("[w]", Style::default().fg(Color::Yellow)),
                    Span::styled(" regenerar token", Style::default().fg(Color::DarkGray)),
                ]),
            ]),
            inner,
        );
    }

    // ── Build logs ────────────────────────────────────────────────────────────

    let area_h = chunks[3].height.saturating_sub(1) as usize; // minus border

    let (log_lines, scroll_hint) = if let Some(buf) = app.build_logs.get(&dep.id) {
        let total = buf.len();
        // Clamp scroll: usize::MAX means follow tail
        let max_skip = total.saturating_sub(area_h);
        let skip = if app.build_log_scroll >= max_skip {
            max_skip
        } else {
            app.build_log_scroll
        };
        let items: Vec<ListItem> = buf
            .iter()
            .skip(skip)
            .take(area_h)
            .map(|l| {
                ListItem::new(Line::from(Span::styled(
                    format!("  {}", l.text),
                    Style::default().fg(Color::DarkGray),
                )))
            })
            .collect();
        let hint = if total == 0 {
            " Build log ".to_string()
        } else if total <= area_h {
            format!(" Build log  {total} linhas ")
        } else {
            format!(
                " Build log  {}/{total}  [[]] scroll  [g] início  [G] fim ",
                skip + items.len().min(area_h)
            )
        };
        (items, hint)
    } else {
        let items = vec![ListItem::new(Line::from(Span::styled(
            "  Sem logs de build para este deployment.",
            Style::default().fg(Color::DarkGray),
        )))];
        (items, " Build log ".to_string())
    };

    let build_block = Block::default()
        .borders(Borders::TOP)
        .title(scroll_hint)
        .border_style(Style::default().fg(Color::DarkGray));

    f.render_widget(List::new(log_lines).block(build_block), chunks[3]);
}

// ─── Logs Tab ─────────────────────────────────────────────────────────────────

fn render_logs_tab(f: &mut Frame, app: &App, area: Rect) {
    let sid = match &app.active_service_id {
        Some(s) => s.clone(),
        None => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Logs — [↑↓] scroll  [f] final  [r] refresh ")
        .border_style(Style::default().fg(Color::DarkGray));

    let log_lines: Vec<&crate::app::LogLine> = app.logs.get(&sid).into_iter().flatten().collect();

    if log_lines.is_empty() {
        let p = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Aguardando logs... (serviço precisa estar Running)",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .block(block);
        f.render_widget(p, area);
        return;
    }

    let inner_height = area.height.saturating_sub(2) as usize;
    let total = log_lines.len();
    let start = if total > inner_height {
        let auto_start = total - inner_height;
        app.log_cursor.min(auto_start)
    } else {
        0
    };

    let visible: Vec<ListItem> = log_lines
        .iter()
        .skip(start)
        .take(inner_height)
        .map(|line| {
            let ts = line.timestamp.format("%H:%M:%S%.3f");
            let color = if line.is_stderr {
                Color::Red
            } else {
                Color::White
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{ts} "), Style::default().fg(Color::DarkGray)),
                Span::styled(line.text.clone(), Style::default().fg(color)),
            ]))
        })
        .collect();

    let list = List::new(visible).block(block);
    f.render_widget(list, area);
}

// ─── Advanced Tab ─────────────────────────────────────────────────────────────

fn render_advanced_tab(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // spacing         [0]
            Constraint::Length(1), // Scaling header  [1]
            Constraint::Length(1), // Replicas        [2]
            Constraint::Length(1), // spacing         [3]
            Constraint::Length(1), // Run Cmd header  [4]
            Constraint::Length(1), // hint text       [5]
            Constraint::Length(1), // Command input   [6]
            Constraint::Length(1), // spacing         [7]
            Constraint::Min(3),    // Args block      [8]
            Constraint::Length(1), // spacing         [9]
            Constraint::Length(1), // Save            [10]
        ])
        .split(area);

    let adv = &app.advanced_tab;

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── Scaling ────────────────────────────────────────────────",
            Style::default().fg(Color::Yellow),
        ))),
        chunks[1],
    );

    render_form_row(f, chunks[2], "Replicas", &adv.replicas, adv.focused == AdvancedField::Replicas);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── Run Command ─────────────────────────────────────────────",
            Style::default().fg(Color::Yellow),
        ))),
        chunks[4],
    );

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  Run a custom command in the container after the application initialized.",
            Style::default().fg(Color::DarkGray),
        ))),
        chunks[5],
    );

    // Command input
    let cmd_focused = adv.focused == AdvancedField::RunCommand;
    let cmd_span = if adv.run_command.is_empty() && !cmd_focused {
        Span::styled("/bin/sh", Style::default().fg(Color::DarkGray))
    } else {
        let cursor = if cmd_focused { "▌" } else { "" };
        Span::styled(
            format!("{}{}", adv.run_command, cursor),
            if cmd_focused { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::White) },
        )
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("  {:<22}", "Command"), Style::default().fg(Color::DarkGray)),
            cmd_span,
        ])),
        chunks[6],
    );

    // Args form-array block
    let args_focused = adv.focused == AdvancedField::RunArgs;
    let args_border_style = if args_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let args_block = Block::default()
        .borders(Borders::ALL)
        .title(" Args — [a] adicionar  [D] remover ")
        .border_style(args_border_style);

    let items: Vec<ListItem> = adv
        .run_args
        .iter()
        .enumerate()
        .map(|(i, arg)| {
            let selected = args_focused && i == adv.args_cursor;
            let editing = selected && adv.args_editing;
            let content = if editing {
                format!(" {}▌", arg)
            } else {
                format!(" {}", arg)
            };
            let style = if selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(Span::styled(content, style)))
        })
        .collect();

    if adv.run_args.is_empty() {
        let placeholder = Paragraph::new(Line::from(Span::styled(
            " Nenhum argumento. Pressione [a] para adicionar.",
            Style::default().fg(Color::DarkGray),
        )))
        .block(args_block);
        f.render_widget(placeholder, chunks[8]);
    } else {
        let list = List::new(items).block(args_block);
        let mut state = ListState::default();
        if args_focused {
            state.select(Some(adv.args_cursor));
        }
        f.render_stateful_widget(list, chunks[8], &mut state);
    }

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            btn_span("[ Save ]", adv.focused == AdvancedField::Save),
        ])),
        chunks[10],
    );
}

// ─── Patches Tab ──────────────────────────────────────────────────────────────

fn render_patches_tab(f: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Patches ")
        .border_style(Style::default().fg(Color::DarkGray));
    let p = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Histórico de patches de configuração aplicados.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  Em breve (v2).",
            Style::default().fg(Color::Yellow),
        )),
    ])
    .block(block);
    f.render_widget(p, area);
}

// ─── Connection Tab (Databases) ───────────────────────────────────────────────

fn render_connection_tab(f: &mut Frame, app: &App, area: Rect) {
    let svc = match app.current_active_service() {
        Some(s) => s,
        None => return,
    };

    let db_kind = match DbKind::detect_from_env(&svc.spec.env_vars) {
        Some(k) => k,
        None => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Connection ")
                .border_style(Style::default().fg(Color::DarkGray));
            let p = Paragraph::new(Span::styled(
                "  Não é um serviço de banco de dados.",
                Style::default().fg(Color::DarkGray),
            ))
            .block(block);
            f.render_widget(p, area);
            return;
        }
    };

    fn env_plain<'a>(vars: &'a [shared::EnvVar], key: &str) -> &'a str {
        vars.iter()
            .find(|e| e.key == key)
            .and_then(|e| {
                if let EnvVarValue::Plain(ref v) = e.value {
                    Some(v.as_str())
                } else {
                    None
                }
            })
            .unwrap_or("")
    }

    let vars = &svc.spec.env_vars;
    let svc_name = &svc.spec.name;
    let yaml_svc = db_kind.yaml_service_name();
    let hostname = format!("rp_{svc_name}-{yaml_svc}-1");
    let port = db_kind.default_port();

    let (conn_url, extras) = match db_kind {
        DbKind::Postgres => {
            let db = env_plain(vars, "POSTGRES_DB");
            let user = env_plain(vars, "POSTGRES_USER");
            let pass = env_plain(vars, "POSTGRES_PASSWORD");
            let url = format!("postgresql://{user}:{pass}@{hostname}:{port}/{db}");
            let extra = vec![
                ("Database", db.to_string()),
                ("User", user.to_string()),
                ("Password", pass.to_string()),
            ];
            (url, extra)
        }
        DbKind::MongoDB => {
            let user = env_plain(vars, "MONGO_INITDB_ROOT_USERNAME");
            let pass = env_plain(vars, "MONGO_INITDB_ROOT_PASSWORD");
            let url = format!("mongodb://{user}:{pass}@{hostname}:{port}");
            let extra = vec![("User", user.to_string()), ("Password", pass.to_string())];
            (url, extra)
        }
        DbKind::MariaDB | DbKind::MySQL => {
            let db = env_plain(vars, "MYSQL_DATABASE");
            let user = env_plain(vars, "MYSQL_USER");
            let pass = env_plain(vars, "MYSQL_PASSWORD");
            let url = format!("mysql://{user}:{pass}@{hostname}:{port}/{db}");
            let extra = vec![
                ("Database", db.to_string()),
                ("User", user.to_string()),
                ("Password", pass.to_string()),
            ];
            (url, extra)
        }
        DbKind::Redis => {
            let pass = env_plain(vars, "REDIS_PASSWORD");
            let url = if pass.is_empty() {
                format!("redis://{hostname}:{port}")
            } else {
                format!("redis://:{pass}@{hostname}:{port}")
            };
            let extra = if pass.is_empty() {
                vec![]
            } else {
                vec![("Password", pass.to_string())]
            };
            (url, extra)
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0)])
        .split(area);

    let mut lines: Vec<Line> = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                format!(" {} ", db_kind.label()),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                format!("{} gerenciado pelo Rustploy", db_kind.label()),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  ── Conexão interna ─────────────────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Host      "),
            Span::styled(
                &hostname,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Port      "),
            Span::styled(port.to_string(), Style::default().fg(Color::Cyan)),
        ]),
    ];

    for (label, value) in &extras {
        lines.push(Line::from(vec![
            Span::raw(format!("  {label:<10} ")),
            Span::styled(value.clone(), Style::default().fg(Color::White)),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  ── URL de conexão ──────────────────────────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            &conn_url,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Use este hostname em outros serviços do mesmo projeto.",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Connection ")
        .border_style(Style::default().fg(Color::DarkGray));
    let p = Paragraph::new(lines).block(block);
    f.render_widget(p, chunks[0]);
}
