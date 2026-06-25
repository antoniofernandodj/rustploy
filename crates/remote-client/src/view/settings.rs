//! Settings screens: Web Server (functional), Git providers and Account.

use super::widgets::*;
use crate::model::{palette, GpForm};
use crate::{App, Message};
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length};
use shared::GitAuthMode;

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
            muted("Ex: https://rustploy.meusite.com  ou  http://192.168.1.42:8788"),
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

pub fn git(app: &App) -> Element<'_, Message> {
    // ── Lista de contas conectadas ────────────────────────────────────────
    let mut connected = column![].spacing(6);
    if app.git_providers.is_empty() {
        connected = connected.push(muted("Nenhuma conta Gitea conectada ainda."));
    }
    for p in &app.git_providers {
        let login = p
            .account
            .as_ref()
            .map(|a| format!("@{}", a.login))
            .unwrap_or_else(|| "(pendente — autorize no navegador)".into());
        let mode = match p.auth_mode {
            GitAuthMode::OAuth => "OAuth2",
            GitAuthMode::Pat => "PAT",
        };
        connected = connected.push(
            container(
                row![
                    column![
                        text(p.name.clone()).size(14).color(palette::WHITE),
                        text(format!("{}  ·  {mode}  ·  {login}", p.base_url))
                            .size(12)
                            .color(palette::GRAY),
                    ]
                    .spacing(2),
                    Space::with_width(Length::Fill),
                    danger_btn("Remover", Message::GpDelete(p.id.clone())),
                ]
                .align_y(Alignment::Center),
            )
            .padding(10)
            .width(Length::Fill)
            .style(container::rounded_box),
        );
    }

    // ── Formulário "Conectar Gitea" ───────────────────────────────────────
    let f: &GpForm = &app.gp_form;
    let mode_btn = |label: &str, mode: GitAuthMode| -> Element<'_, Message> {
        let active = f.mode == mode;
        button(text(label.to_string()).size(13))
            .on_press(Message::GpMode(mode))
            .style(if active { button::primary } else { button::secondary })
            .padding([8, 16])
            .into()
    };

    let mut form = column![
        section("Conectar conta Gitea"),
        labeled_input("Nome", "Meu Gitea", &f.name, Message::GpName),
        labeled_input("Base URL", "https://gitea.exemplo.com", &f.base_url, Message::GpBaseUrl),
        row![
            container(label_text("Método")).width(Length::Fixed(190.0)),
            mode_btn("OAuth2", GitAuthMode::OAuth),
            mode_btn("Token (PAT)", GitAuthMode::Pat),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
    .spacing(8);

    form = match f.mode {
        GitAuthMode::OAuth => {
            let domain_set = !app.ss_domain.trim().is_empty();
            let redirect = if domain_set {
                format!("{}/oauth/gitea/callback", app.ss_domain.trim_end_matches('/'))
            } else {
                "<configure o domínio em Settings → Web Server>/oauth/gitea/callback".to_string()
            };
            // Linha destacada e copiável com a Redirect URI a colar no Gitea.
            let redirect_row = row![
                container(label_text("Redirect URI")).width(Length::Fixed(190.0)),
                text(redirect.clone())
                    .size(13)
                    .color(if domain_set { palette::CYAN } else { palette::YELLOW }),
                Space::with_width(Length::Fixed(8.0)),
                ghost_btn("Copiar", Message::Copy(redirect)),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            form.push(labeled_input("Client ID", "", &f.client_id, Message::GpClientId))
                .push(labeled_secret("Client Secret", &f.client_secret, Message::GpClientSecret))
                .push(redirect_row)
                .push(muted(
                    "No Gitea: Settings → Applications → crie um OAuth2 app e cole a Redirect URI acima.",
                ))
        }
        GitAuthMode::Pat => form
            .push(labeled_secret("Personal Access Token", &f.pat, Message::GpPat))
            .push(muted(
                "No Gitea: Settings → Applications → Generate New Token (escopo: repo).",
            )),
    };

    form = form.push(Space::with_height(Length::Fixed(6.0))).push(
        row![
            primary_btn("Conectar", Message::GpConnect),
            ghost_btn("Atualizar lista", Message::GpRefresh),
        ]
        .spacing(8),
    );

    panel(
        "Git",
        scrollable(
            column![
                section("Contas conectadas"),
                connected,
                Space::with_height(Length::Fixed(16.0)),
                form,
            ]
            .spacing(8),
        )
        .height(Length::Fill)
        .into(),
    )
}

/// Labeled single-line input that masks its value (for secrets/tokens).
fn labeled_secret<'a>(
    label: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    row![
        container(text(label.to_string()).size(13).color(palette::GRAY))
            .width(Length::Fixed(190.0)),
        text_input("", value).on_input(on_input).secure(true).padding(6).size(13),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub fn account(app: &App) -> Element<'_, Message> {
    let version = app.daemon_status().map(|d| d.version.clone()).unwrap_or_else(|| "—".into());
    panel(
        "Account",
        column![
            kv("Endpoint", app.url.clone()),
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
