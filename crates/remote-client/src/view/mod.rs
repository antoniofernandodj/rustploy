//! Top-level view router and window chrome (titlebar, sidebar, statusbar,
//! overlays). Mirrors the TUI layout.

mod home;
mod projects;
mod service;
mod settings;
mod sidebar;
mod widgets;
mod wizard;

use crate::model::{palette, ConfirmAction, View};
use crate::{App, Message};
use iced::widget::{
    button, checkbox, column, container, mouse_area, row, stack, text, text_input, Space,
};
use iced::{Alignment, Background, Color, Element, Length};
use widgets::*;

pub fn view(app: &App) -> Element<'_, Message> {
    if !app.connected {
        return connect_screen(app);
    }

    let base = column![
        titlebar(app),
        container(
            row![sidebar::view(app), content(app)]
                .spacing(16)
                .height(Length::Fill),
        )
        .padding(16)
        .height(Length::Fill),
        statusbar(app),
    ]
    .height(Length::Fill);

    let mut layers: Vec<Element<Message>> = vec![base.into()];

    if app.build_log_modal_open {
        layers.push(wide_modal(service::build_log_modal_content(app)));
    }
    if app.new_project_open {
        layers.push(modal(new_project_form(app)));
    }
    if app.ns.is_some() {
        layers.push(modal(wizard::view(app)));
    }
    if let Some(action) = &app.confirm {
        layers.push(modal(confirm_dialog(action)));
    }
    if let Some(n) = &app.notification {
        layers.push(toast(&n.message, n.is_error));
    }

    stack(layers).into()
}

fn content(app: &App) -> Element<'_, Message> {
    let inner: Element<Message> = match app.view {
        View::HomeDeployments => home::deployments(app),
        View::HomeDeployEngine => home::deploy_engine(app),
        View::HomeMonitoring => home::monitoring(app),
        View::HomeSchedules => placeholder("Schedules", "Agendamentos de auto-deploy (v2)."),
        View::HomeIngress => placeholder("Ingress Routes", "Tabela de rotas ativa no proxy hyper."),
        View::HomeDocker => placeholder("Docker", "Containers, redes e imagens gerenciadas."),
        View::HomeRequests => placeholder("Requests", "Log de requisições recebidas pelo proxy."),
        View::Projects => projects::list(app),
        View::ProjectDetail => projects::detail(app),
        View::ServiceDetail => service::detail(app),
        View::SettingsWebServer => settings::web_server(app),
        View::SettingsProfile => placeholder("Profile", "Informações da instalação e uso de recursos."),
        View::SettingsUsers => placeholder("Users", "Controle de acesso ao socket (v2)."),
        View::SettingsAuditLogs => placeholder("Audit Logs", "Histórico de ações administrativas."),
        View::SettingsSshKeys => placeholder("SSH Keys", "Chaves SSH para repositórios privados."),
        View::SettingsTags => placeholder("Tags", "Tags para organização de projetos e serviços."),
        View::SettingsGit => settings::git(app),
        View::SettingsRegistry => placeholder("Registry", "Credenciais para Docker registries privados."),
        View::SettingsS3 => placeholder("S3 Destinations", "Destinos S3 para backups (v2)."),
        View::SettingsCerts => placeholder("Certificates", "Certificados TLS e status ACME por domínio."),
        View::SettingsSso => placeholder("SSO", "Single Sign-On (v2)."),
        View::Account => settings::account(app),
    };
    container(inner)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn titlebar(app: &App) -> Element<'_, Message> {
    let status = match app.daemon_status() {
        Some(d) => format!(
            "daemon v{} · up {} · {}/{} serviços",
            d.version,
            fmt_uptime(d.uptime_secs),
            d.services_running,
            d.services_total
        ),
        None => app.status_msg.clone(),
    };
    container(
        row![
            text(format!(" Rustploy Remote  v{}", env!("CARGO_PKG_VERSION")))
                .size(15)
                .color(palette::CYAN),
            text("RWP").size(12).color(palette::GRAY),
            Space::with_width(Length::Fill),
            text(format!("● {}  ", app.url)).size(12).color(palette::GRAY),
            text(status).size(12).color(palette::GRAY),
            Space::with_width(Length::Fixed(16.0)),
            button(
                text("Desconectar")
                    .size(13)
                    .wrapping(text::Wrapping::None),
            )
            .on_press(Message::Disconnect)
            .style(button::danger)
            .padding([8, 16]),
        ]
        .spacing(14)
        .align_y(Alignment::Center),
    )
    .padding([14, 20])
    .width(Length::Fill)
    .into()
}

fn statusbar(app: &App) -> Element<'_, Message> {
    let hint = match app.view {
        View::Projects => "Clique em um projeto para abrir · [+ Novo Projeto]",
        View::ProjectDetail => "Abas: Services / Environment / Secrets / Settings",
        View::ServiceDetail => "Abas de configuração do serviço · Deploy / Stop / Reload / Rollback",
        _ => "Selecione um item na barra lateral",
    };
    container(text(hint).size(12).color(palette::GRAY))
        .padding([10, 20])
        .width(Length::Fill)
        .into()
}

// ── Connect screen ────────────────────────────────────────────────────────────

fn connect_screen(app: &App) -> Element<'_, Message> {
    let mut form = column![
        text("Rustploy Remote").size(30).color(palette::CYAN),
        text("Controle um daemon rustployd via RWP (TCP)").size(14).color(palette::GRAY),
        Space::with_height(Length::Fixed(8.0)),
        label_text("URL do servidor"),
        text_input("rwp://127.0.0.1:8787", &app.url)
            .on_input(Message::UrlChanged)
            .on_submit(Message::Connect)
            .padding(10),
        checkbox("Lembrar servidor (URL)", app.remember_url)
            .on_toggle(Message::RememberUrlToggled)
            .size(16)
            .text_size(13),
        label_text("Token (opcional)"),
        text_input("token de acesso", &app.token)
            .on_input(Message::TokenChanged)
            .on_submit(Message::Connect)
            .secure(true)
            .padding(10),
        checkbox("Lembrar token", app.remember_token)
            .on_toggle(Message::RememberTokenToggled)
            .size(16)
            .text_size(13),
        Space::with_height(Length::Fixed(6.0)),
        button(text("Conectar")).on_press(Message::Connect).padding([10, 24]),
        text(app.status_msg.clone()).size(13).color(palette::GRAY),
    ]
    .spacing(10)
    .max_width(440);

    if let Some(err) = &app.error {
        form = form.push(text(format!("⚠ {err}")).size(13).color(palette::RED));
    }

    container(form)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(40)
        .into()
}

// ── Overlays ──────────────────────────────────────────────────────────────────

fn backdrop() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.55))),
        ..Default::default()
    }
}

fn modal(body: Element<'_, Message>) -> Element<'_, Message> {
    // mouse_area cobre o ecrã inteiro e captura todos os eventos de ponteiro,
    // impedindo que o hover chegue aos elementos por baixo do modal.
    mouse_area(
        container(
            container(body)
                .padding(32)
                .max_width(720)
                .style(container::rounded_box),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .style(|_| backdrop()),
    )
    .into()
}

fn wide_modal(body: Element<'_, Message>) -> Element<'_, Message> {
    mouse_area(
        container(
            container(body)
                .padding(24)
                .max_width(1200)
                .width(Length::Fill)
                .style(container::rounded_box),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .style(|_| backdrop()),
    )
    .into()
}

fn new_project_form(app: &App) -> Element<'_, Message> {
    column![
        text("Novo Projeto").size(20).color(palette::CYAN),
        Space::with_height(Length::Fixed(10.0)),
        label_text("Nome"),
        text_input("meu-projeto", &app.np_name).on_input(Message::NpName).on_submit(Message::NpSubmit).padding(8),
        label_text("Descrição (opcional)"),
        text_input("opcional…", &app.np_desc).on_input(Message::NpDesc).padding(8),
        Space::with_height(Length::Fixed(12.0)),
        row![
            primary_btn("Criar", Message::NpSubmit),
            ghost_btn("Cancelar", Message::NpCancel),
        ]
        .spacing(8),
    ]
    .spacing(8)
    .width(Length::Fixed(420.0))
    .into()
}

fn confirm_dialog(action: &ConfirmAction) -> Element<'static, Message> {
    let msg = match action {
        ConfirmAction::DeleteProject(_) => "Remover este projeto? Esta ação é irreversível.",
        ConfirmAction::DeleteService(_) => "Remover este serviço? Esta ação é irreversível.",
    };
    column![
        text("Confirmar").size(20).color(palette::YELLOW),
        Space::with_height(Length::Fixed(10.0)),
        text(msg).size(14),
        Space::with_height(Length::Fixed(14.0)),
        row![
            danger_btn("Sim, remover", Message::ConfirmYes),
            ghost_btn("Cancelar", Message::ConfirmNo),
        ]
        .spacing(8),
    ]
    .spacing(6)
    .width(Length::Fixed(420.0))
    .into()
}

fn toast(message: &str, is_error: bool) -> Element<'static, Message> {
    let color = if is_error { palette::RED } else { palette::GREEN };
    container(
        button(text(message.to_string()).size(13).color(color))
            .on_press(Message::DismissNotification)
            .style(button::secondary)
            .padding([8, 14]),
    )
    .align_right(Length::Fill)
    .align_bottom(Length::Fill)
    .padding(16)
    .into()
}

pub fn fmt_uptime(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}
