use crate::app::{App, ServerSettingsField, View};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    match &app.view {
        View::SettingsWebServer => render_web_server(f, app, area),
        _ => render_placeholder(f, app, area),
    }
}

fn render_web_server(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Web Server ")
        .border_style(Style::default().fg(Color::DarkGray));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // [0]  espaço
            Constraint::Length(1), // [1]  header domínio
            Constraint::Length(1), // [2]  espaço
            Constraint::Length(1), // [3]  campo domínio
            Constraint::Length(1), // [4]  dica domínio
            Constraint::Length(1), // [5]  espaço
            Constraint::Length(1), // [6]  header HTTPS
            Constraint::Length(1), // [7]  espaço
            Constraint::Length(1), // [8]  campo email ACME
            Constraint::Length(1), // [9]  dica email
            Constraint::Length(1), // [10] espaço
            Constraint::Length(1), // [11] botão Save
            Constraint::Length(1), // [12] espaço
            Constraint::Length(1), // [13] header webhook
            Constraint::Length(1), // [14] explicação webhook
            Constraint::Min(0),
        ])
        .split(block.inner(area));
    f.render_widget(block, area);

    let ss = &app.server_settings;
    let focused_style = Style::default().fg(Color::Cyan);
    let normal_style = Style::default().fg(Color::White);
    let label_style = Style::default().fg(Color::DarkGray);

    // ── Domínio do Servidor ───────────────────────────────────────────────────
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── Domínio do Servidor ─────────────────────────────────────",
            Style::default().fg(Color::Yellow),
        ))),
        chunks[1],
    );

    let domain_val = format!(
        "{}{}",
        ss.server_domain,
        if ss.focused == ServerSettingsField::ServerDomain { "▌" } else { "" }
    );
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("  {:<22}", "Domínio / URL base"), label_style),
            Span::styled(
                domain_val,
                if ss.focused == ServerSettingsField::ServerDomain { focused_style } else { normal_style },
            ),
        ])),
        chunks[3],
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  Ex: https://rustploy.meusite.com  ou  http://192.168.1.42:9001",
            label_style,
        ))),
        chunks[4],
    );

    // ── HTTPS / ACME ──────────────────────────────────────────────────────────
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── HTTPS automático (Let's Encrypt) ────────────────────────",
            Style::default().fg(Color::Yellow),
        ))),
        chunks[6],
    );

    let email_val = format!(
        "{}{}",
        ss.acme_email,
        if ss.focused == ServerSettingsField::AcmeEmail { "▌" } else { "" }
    );
    let https_status = if ss.acme_email.trim().is_empty() {
        Span::styled(" (desabilitado)", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(" (ativo no próximo restart)", Style::default().fg(Color::Green))
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("  {:<22}", "E-mail Let's Encrypt"), label_style),
            Span::styled(
                email_val,
                if ss.focused == ServerSettingsField::AcmeEmail { focused_style } else { normal_style },
            ),
            https_status,
        ])),
        chunks[8],
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  Certificados TLS emitidos automaticamente. Requer restart do daemon.",
            label_style,
        ))),
        chunks[9],
    );

    // ── Save ──────────────────────────────────────────────────────────────────
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  "),
            btn_span("[ Save ]", ss.focused == ServerSettingsField::Save),
        ])),
        chunks[11],
    );

    // ── Webhooks ──────────────────────────────────────────────────────────────
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "── Webhooks ────────────────────────────────────────────────",
            Style::default().fg(Color::Yellow),
        ))),
        chunks[13],
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  A URL de webhook de cada serviço é montada com este domínio como base.",
            label_style,
        ))),
        chunks[14],
    );
}

fn render_placeholder(f: &mut Frame, app: &App, area: Rect) {
    let (title, desc) = match &app.view {
        View::SettingsProfile => (
            "Profile",
            "Informações da instalação, versão, uso de recursos do daemon.",
        ),
        View::SettingsUsers => ("Users", "Controle de acesso ao Unix Domain Socket (v2)."),
        View::SettingsAuditLogs => (
            "Audit Logs",
            "Histórico de ações administrativas no daemon.",
        ),
        View::SettingsSshKeys => (
            "SSH Keys",
            "Chaves SSH disponíveis para autenticação em repositórios privados.",
        ),
        View::SettingsTags => ("Tags", "Tags para organização de projetos e serviços."),
        View::SettingsGit => ("Git", "Configurações globais de clone e build Git."),
        View::SettingsRegistry => ("Registry", "Credenciais para Docker registries privados."),
        View::SettingsS3 => (
            "S3 Destinations",
            "Destinos S3 para backups de volumes e logs (v2).",
        ),
        View::SettingsCerts => (
            "Certificates",
            "Certificados TLS manuais e status ACME por domínio.",
        ),
        View::SettingsSso => ("SSO", "Single Sign-On para acesso ao TUI (v2)."),
        _ => ("Settings", ""),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" {title} "))
        .border_style(Style::default().fg(Color::DarkGray));

    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(desc, Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(Span::styled(
            "Em construção.",
            Style::default().fg(Color::Yellow),
        )),
    ])
    .block(block);

    f.render_widget(text, area);
}

pub fn render_account(f: &mut Frame, _app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Account ")
        .border_style(Style::default().fg(Color::DarkGray));

    let text = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  Usuário:  ", Style::default().fg(Color::DarkGray)),
            Span::styled("root (socket)", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("  Versão:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("  Socket:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "/run/rustploy/rustploy.sock",
                Style::default().fg(Color::White),
            ),
        ]),
    ])
    .block(block);

    f.render_widget(text, area);
}

fn btn_span(label: &'static str, focused: bool) -> Span<'static> {
    if focused {
        Span::styled(
            label,
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(label, Style::default().fg(Color::DarkGray))
    }
}
