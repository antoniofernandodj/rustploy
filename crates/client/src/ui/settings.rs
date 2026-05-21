use crate::app::{App, View};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let (title, desc) = match &app.view {
        View::SettingsWebServer => ("Web Server", "Configurar Pingora: portas HTTP/HTTPS, bind address, cabeçalhos globais."),
        View::SettingsProfile => ("Profile", "Informações da instalação, versão, uso de recursos do daemon."),
        View::SettingsUsers => ("Users", "Controle de acesso ao Unix Domain Socket (v2)."),
        View::SettingsAuditLogs => ("Audit Logs", "Histórico de ações administrativas no daemon."),
        View::SettingsSshKeys => ("SSH Keys", "Chaves SSH disponíveis para autenticação em repositórios privados."),
        View::SettingsTags => ("Tags", "Tags para organização de projetos e serviços."),
        View::SettingsGit => ("Git", "Configurações globais de clone e build Git."),
        View::SettingsRegistry => ("Registry", "Credenciais para Docker registries privados."),
        View::SettingsS3 => ("S3 Destinations", "Destinos S3 para backups de volumes e logs (v2)."),
        View::SettingsCerts => ("Certificates", "Certificados TLS manuais e status ACME por domínio."),
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
        Line::from(Span::styled("Em construção.", Style::default().fg(Color::Yellow))),
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
            Span::styled("/run/rustploy/rustploy.sock", Style::default().fg(Color::White)),
        ]),
    ])
    .block(block);

    f.render_widget(text, area);
}
