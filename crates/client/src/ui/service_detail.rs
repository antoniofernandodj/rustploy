use crate::app::{App, EnvEditField, GeneralTabField, HcField, ServiceTab};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use shared::{EnvVarValue, ServiceSource};

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
    let tabs = ServiceTab::all();
    let mut spans: Vec<Span> = vec![Span::styled(
        format!(" {svc_name} "),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )];

    for tab in tabs {
        let active = tab == &app.service_tab;
        let style = if active {
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
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
        ServiceTab::Environment => render_env_tab(f, app, area),
        ServiceTab::Domains => render_domains_tab(f, app, area),
        ServiceTab::Deployments => render_deployments_tab(f, app, area),
        ServiceTab::Healthcheck => render_healthcheck_tab(f, app, area),
        ServiceTab::Logs => render_logs_tab(f, app, area),
        ServiceTab::Patches => render_patches_tab(f, area),
    }
}

// ─── General Tab ─────────────────────────────────────────────────────────────

fn render_general_tab(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // spacing
            Constraint::Length(1), // action buttons
            Constraint::Length(1), // spacing
            Constraint::Length(1), // Provider header
            Constraint::Length(1), // Repo URL
            Constraint::Length(1), // Branch
            Constraint::Length(1), // Build Path
            Constraint::Length(1), // Watch Paths
            Constraint::Length(1), // Submodules
            Constraint::Length(1), // spacing
            Constraint::Length(1), // SSH + Save buttons (provider)
            Constraint::Length(1), // spacing
            Constraint::Length(1), // Build Type header
            Constraint::Length(1), // Docker File
            Constraint::Length(1), // Context Path
            Constraint::Length(1), // Build Stage
            Constraint::Length(1), // spacing
            Constraint::Length(1), // Build Save button
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
        btn_span("[ Rebuild ]", gt.focused_field == GeneralTabField::BtnRebuild),
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

    render_form_row(f, chunks[4], "Repository URL", &gt.repo_url, gt.focused_field == GeneralTabField::RepoUrl);
    render_form_row(f, chunks[5], "Branch", &gt.branch, gt.focused_field == GeneralTabField::Branch);
    render_form_row(f, chunks[6], "Build Path", &gt.build_path, gt.focused_field == GeneralTabField::BuildPath);
    render_form_row(f, chunks[7], "Watch Paths", &gt.watch_paths, gt.focused_field == GeneralTabField::WatchPaths);

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
        chunks[8],
    );

    // SSH Keys + Provider Save buttons
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            btn_span("[ Add SSH Keys ]", gt.focused_field == GeneralTabField::AddSshKeys),
            Span::raw("   "),
            btn_span("[ Save ]", gt.focused_field == GeneralTabField::ProviderSave),
        ])),
        chunks[10],
    );

    // Build Type header
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── Build Type: Dockerfile ─────────────────────────────────",
            Style::default().fg(Color::Yellow),
        ))),
        chunks[12],
    );

    render_form_row(f, chunks[13], "Docker File", &gt.dockerfile, gt.focused_field == GeneralTabField::DockerFile);
    render_form_row(f, chunks[14], "Docker Context Path", &gt.context_path, gt.focused_field == GeneralTabField::DockerContextPath);
    render_form_row(f, chunks[15], "Docker Build Stage", &gt.build_stage, gt.focused_field == GeneralTabField::DockerBuildStage);

    // Build Save button
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            btn_span("[ Save ]", gt.focused_field == GeneralTabField::BuildSave),
        ])),
        chunks[17],
    );
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
            Span::styled(format!("  {:<22}", "Kind"), Style::default().fg(Color::DarkGray)),
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
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM)
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
                if http_active { Style::default().fg(Color::DarkGray) } else { Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM) },
            ),
            Span::styled(
                format!("{}{}", hc.http_path, if hc.focused == HcField::HttpPath && http_active { "▌" } else { "" }),
                dim(hc.focused == HcField::HttpPath),
            ),
        ])),
        chunks[5],
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("  {:<22}", "Expected Status"),
                if http_active { Style::default().fg(Color::DarkGray) } else { Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM) },
            ),
            Span::styled(
                format!("{}{}", hc.expected_status, if hc.focused == HcField::ExpectedStatus && http_active { "▌" } else { "" }),
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

    render_form_row(f, chunks[9], "Interval (s)", &hc.interval, hc.focused == HcField::Interval);
    render_form_row(f, chunks[10], "Timeout (s)", &hc.timeout, hc.focused == HcField::Timeout);
    render_form_row(f, chunks[11], "Retries", &hc.retries, hc.focused == HcField::Retries);
    render_form_row(f, chunks[12], "Start Period (s)", &hc.start_period, hc.focused == HcField::StartPeriod);

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
            Span::styled(format!("  {:<22}", label), Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{value}{cursor}"), val_style),
        ])),
        area,
    );
}

fn btn_span(label: &str, focused: bool) -> Span<'static> {
    if focused {
        Span::styled(
            label.to_string(),
            Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD),
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
                    if v.len() > 30 { format!("{}...", &v[..30]) } else { v.clone() }
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

    let key_style = if key_focused { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::White) };
    let val_style = if val_focused { Style::default().fg(Color::Cyan) } else { Style::default().fg(Color::White) };

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
    let svc = match app.current_active_service() {
        Some(s) => s,
        None => return,
    };

    let domain_display = svc.spec.domain.as_deref().unwrap_or("— não configurado —");
    let domain_style = if svc.spec.domain.is_some() {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let source_info = match &svc.spec.source {
        ServiceSource::Git(g) => format!("Git: {} @ {}", g.url, g.branch),
        ServiceSource::Registry { image } => format!("Registry: {image}"),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Domains ")
        .border_style(Style::default().fg(Color::DarkGray));

    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Domínio:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(domain_display, domain_style),
        ]),
        Line::from(vec![
            Span::styled("  TLS:      ", Style::default().fg(Color::DarkGray)),
            Span::styled("Não configurado", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("  Porta:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(svc.spec.port.to_string(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Fonte:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(source_info, Style::default().fg(Color::White)),
        ]),
    ])
    .block(block);
    f.render_widget(text, area);
}

// ─── Deployments Tab ──────────────────────────────────────────────────────────

fn render_deployments_tab(f: &mut Frame, app: &App, area: Rect) {
    let svc = match app.current_active_service() {
        Some(s) => s,
        None => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Deployments — [r] rollback  [a] abortar ")
        .border_style(Style::default().fg(Color::DarkGray));

    if let Some(dep) = &app.last_deployment {
        if dep.service_id == svc.id {
            let states: Vec<&str> = dep.states_log.iter().map(|t| t.to.label()).collect();
            let chain = states.join(" → ");
            let duration = dep
                .finished_at
                .map(|fin| (fin - dep.started_at).num_seconds())
                .map(|s| format!("{s}s"))
                .unwrap_or_else(|| "em andamento".into());

            let text = Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![
                    Span::styled("  ID:       ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&dep.id, Style::default().fg(Color::Cyan)),
                ]),
                Line::from(vec![
                    Span::styled("  Estado:   ", Style::default().fg(Color::DarkGray)),
                    Span::styled(dep.state.label(), Style::default().fg(Color::Green)),
                ]),
                Line::from(vec![
                    Span::styled("  Duração:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(duration, Style::default().fg(Color::White)),
                ]),
                Line::from(vec![
                    Span::styled("  Caminho:  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(chain, Style::default().fg(Color::DarkGray)),
                ]),
            ])
            .block(block);
            f.render_widget(text, area);
            return;
        }
    }

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
}

// ─── Logs Tab ─────────────────────────────────────────────────────────────────

fn render_logs_tab(f: &mut Frame, app: &App, area: Rect) {
    let sid = match &app.active_service_id {
        Some(s) => s.clone(),
        None => return,
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Logs — [↑↓] scroll  [f] ir ao final ")
        .border_style(Style::default().fg(Color::DarkGray));

    let log_lines: Vec<&crate::app::LogLine> =
        app.logs.get(&sid).into_iter().flatten().collect();

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
            let color = if line.is_stderr { Color::Red } else { Color::White };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{ts} "), Style::default().fg(Color::DarkGray)),
                Span::styled(line.text.clone(), Style::default().fg(color)),
            ]))
        })
        .collect();

    let list = List::new(visible).block(block);
    f.render_widget(list, area);
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
