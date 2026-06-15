//! Settings screens: Web Server (functional) and Account.

use super::widgets::*;
use crate::model::palette;
use crate::{App, Message};
use iced::widget::{column, row, text, Space};
use iced::{Element, Length};

pub fn web_server(app: &App) -> Element<'_, Message> {
    let https_status = if app.ss_email.trim().is_empty() {
        text("(desabilitado)").size(12).color(palette::GRAY)
    } else {
        text("(ativo)").size(12).color(palette::GREEN)
    };

    panel(
        "Web Server",
        column![
            section("Domínio do Servidor"),
            labeled_input("Domínio / URL base", "https://rustploy.meusite.com", &app.ss_domain, Message::SsDomain),
            muted("Ex: https://rustploy.meusite.com  ou  http://192.168.1.42:9001"),
            Space::with_height(Length::Fixed(8.0)),
            section("HTTPS automático (Let's Encrypt)"),
            row![
                labeled_input("E-mail Let's Encrypt", "voce@exemplo.com", &app.ss_email, Message::SsEmail),
                https_status,
            ]
            .spacing(8),
            muted("Certificados TLS emitidos automaticamente. Requer restart do daemon."),
            Space::with_height(Length::Fixed(8.0)),
            primary_btn("Save", Message::SsSave),
            Space::with_height(Length::Fixed(12.0)),
            section("Webhooks"),
            muted("A URL de webhook de cada serviço é montada com este domínio como base."),
        ]
        .spacing(6)
        .into(),
    )
}

pub fn account(app: &App) -> Element<'_, Message> {
    let version = app.daemon_status().map(|d| d.version.clone()).unwrap_or_else(|| "—".into());
    panel(
        "Account",
        column![
            kv("Endpoint", app.address.clone()),
            kv("Versão do daemon", version),
            kv("Transporte", "RWP / TCP".into()),
            kv("Autenticado", if app.token.is_empty() { "não".into() } else { "token".into() }),
        ]
        .spacing(4)
        .into(),
    )
}

fn kv(label: &str, value: String) -> Element<'_, Message> {
    row![
        iced::widget::container(text(label.to_string()).size(13).color(palette::GRAY)).width(Length::Fixed(160.0)),
        text(value).size(13).color(palette::WHITE),
    ]
    .spacing(8)
    .into()
}
