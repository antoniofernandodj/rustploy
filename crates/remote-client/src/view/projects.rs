//! Projects list and project-detail screen (Services / Environment / Secrets /
//! Settings tabs), mirroring the TUI.

use super::service::kv_form;
use super::widgets::*;
use crate::model::{palette, ConfirmAction, ProjectTab};
use crate::update::status_color;
use crate::{App, Message};
use iced::widget::{button, column, row, scrollable, text, text_editor, Space};
use iced::{Alignment, Element, Length};
use shared::EnvVarValue;

pub fn list(app: &App) -> Element<'_, Message> {
    let header = row![
        text("Projects").size(16).color(palette::CYAN),
        Space::with_width(Length::Fill),
        primary_btn("+ Novo Projeto", Message::NewProjectOpen),
    ]
    .align_y(Alignment::Center);

    let body: Element<Message> = if app.projects.is_empty() {
        muted("Nenhum projeto criado. Clique em '+ Novo Projeto'.")
    } else {
        let mut col = column![].spacing(4);
        for p in &app.projects {
            let count = app.services.iter().filter(|s| s.spec.project_id == p.id).count();
            let desc = p.description.clone().unwrap_or_else(|| "sem descrição".into());
            col = col.push(
                button(
                    row![
                        text(p.name.clone()).size(14).color(palette::WHITE).width(Length::Fixed(240.0)),
                        text(desc).size(12).color(palette::GRAY).width(Length::Fill),
                        text(format!("{count} serviço{}", if count == 1 { "" } else { "s" }))
                            .size(12)
                            .color(palette::CYAN),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::OpenProject(p.id.clone()))
                .width(Length::Fill)
                .style(button::secondary)
                .padding([6, 10]),
            );
        }
        scrollable(col).height(Length::Fill).into()
    };

    panel_with_header(header, body)
}

pub fn detail(app: &App) -> Element<'_, Message> {
    let name = app.current_project().map(|p| p.name.clone()).unwrap_or_else(|| "Projeto".into());
    let tabs = [
        (ProjectTab::Services, "Services"),
        (ProjectTab::Environment, "Environment"),
        (ProjectTab::Secrets, "Secrets"),
        (ProjectTab::Settings, "Settings"),
    ];

    let header = row![
        ghost_btn("‹ Projetos", Message::BackToProjects),
        text(name).size(18).color(palette::CYAN),
        Space::with_width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let body = match app.project_tab {
        ProjectTab::Services => services_tab(app),
        ProjectTab::Environment => environment_tab(app),
        ProjectTab::Secrets => secrets_tab(app),
        ProjectTab::Settings => settings_tab(app),
    };

    column![
        header,
        Space::with_height(Length::Fixed(6.0)),
        tab_bar(&tabs, app.project_tab, Message::ProjectTab),
        Space::with_height(Length::Fixed(8.0)),
        body,
    ]
    .spacing(2)
    .height(Length::Fill)
    .into()
}

fn services_tab(app: &App) -> Element<'_, Message> {
    let head = row![
        section("Services"),
        Space::with_width(Length::Fill),
        primary_btn("+ Novo Serviço", Message::NewServiceOpen),
    ]
    .align_y(Alignment::Center);

    let services = app.project_services();
    let body: Element<Message> = if services.is_empty() {
        muted("Nenhum serviço. Clique em '+ Novo Serviço'.")
    } else {
        let mut col = column![].spacing(4);
        for svc in services {
            let metrics = app
                .metrics
                .get(&svc.id)
                .and_then(|m| m.last())
                .map(|m| format!("↑{:.0}M {:.0}%", m.mem_used_bytes as f64 / 1_048_576.0, m.cpu_percent))
                .unwrap_or_default();
            col = col.push(
                row![
                    button(
                        row![
                            text(svc.spec.name.clone()).size(13).color(palette::WHITE).width(Length::Fixed(260.0)),
                            text(format!("[{}]", svc.status)).size(12).color(status_color(&svc.status)).width(Length::Fixed(130.0)),
                            text(metrics).size(11).color(palette::GRAY),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .on_press(Message::OpenService(svc.id.clone()))
                    .width(Length::Fill)
                    .style(button::secondary)
                    .padding([6, 10]),
                    button(text("✕").size(12))
                        .on_press(Message::AskDelete(ConfirmAction::DeleteService(svc.id.clone())))
                        .style(button::danger)
                        .padding([4, 8]),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
        }
        scrollable(col).height(Length::Fill).into()
    };

    panel_with_header(head, body)
}

fn environment_tab(app: &App) -> Element<'_, Message> {
    let project = match app.current_project() {
        Some(p) => p,
        None => return panel("Environment", text("—").into()),
    };

    let text_btn_label = if app.p_env_text_open { "Fechar .env" } else { ".env" };
    let head = row![
        section("Environment — herdado por todos os serviços"),
        Space::with_width(Length::Fill),
        ghost_btn("Exportar", Message::PEnvExport),
        ghost_btn(text_btn_label, Message::PEnvTextOpen),
        ghost_btn("+ Adicionar", Message::PEnvOpen),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let mut col = column![head].spacing(6);

    if app.p_env_text_open {
        col = col.push(
            column![
                muted("Cole ou edite no formato KEY=VALUE (# para comentários). Importar substitui todas as variáveis."),
                text_editor(&app.p_env_text_editor)
                    .on_action(Message::PEnvTextAction)
                    .height(Length::Fixed(200.0)),
                row![
                    primary_btn("Importar", Message::PEnvImport),
                    ghost_btn("Cancelar", Message::PEnvTextOpen),
                ].spacing(8),
            ]
            .spacing(6),
        );
    } else if app.p_env_editor.open {
        col = col.push(kv_form(
            &app.p_env_editor.key,
            &app.p_env_editor.value,
            Message::PEnvKey,
            Message::PEnvVal,
            Message::PEnvSubmit,
            Message::PEnvCancel,
            false,
        ));
    }

    if project.env_vars.is_empty() {
        col = col.push(muted("Nenhuma variável."));
    } else {
        for (i, ev) in project.env_vars.iter().enumerate() {
            let val = match &ev.value {
                EnvVarValue::Plain(v) => v.clone(),
                EnvVarValue::Secret(s) => format!("<secret:{s}>"),
            };
            col = col.push(
                row![
                    text(ev.key.clone()).size(13).color(palette::CYAN).width(Length::Fixed(240.0)),
                    text("=").size(13).color(palette::GRAY),
                    text(val).size(13).color(palette::WHITE).width(Length::Fill),
                    button(text("✕").size(12)).on_press(Message::PEnvDelete(i)).style(button::danger).padding([2, 8]),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }
    panel("Environment", scrollable(col).height(Length::Fill).into())
}

fn secrets_tab(app: &App) -> Element<'_, Message> {
    let head = row![
        section("Secrets — credenciais criptografadas por projeto"),
        Space::with_width(Length::Fill),
        ghost_btn("+ Novo Secret", Message::SecretOpen),
    ]
    .align_y(Alignment::Center);

    let mut col = column![head].spacing(6);

    if app.secret_editor.open {
        col = col.push(kv_form(
            &app.secret_editor.key,
            &app.secret_editor.value,
            Message::SecretName,
            Message::SecretVal,
            Message::SecretSubmit,
            Message::SecretCancel,
            true,
        ));
    }

    if app.project_secrets.is_empty() {
        col = col.push(muted("Nenhum secret."));
    } else {
        for name in &app.project_secrets {
            col = col.push(
                row![
                    text(name.clone()).size(13).color(palette::CYAN).width(Length::Fixed(280.0)),
                    text("••••••••").size(13).color(palette::GRAY).width(Length::Fill),
                    button(text("✕").size(12)).on_press(Message::SecretDelete(name.clone())).style(button::danger).padding([2, 8]),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }
    panel("Secrets", scrollable(col).height(Length::Fill).into())
}

fn settings_tab(app: &App) -> Element<'_, Message> {
    let project_id = app.active_project_id.clone().unwrap_or_default();
    let svc_count = app.services.len();
    let can_delete = svc_count == 0;

    let mut col = column![
        section("Configurações do Projeto"),
        labeled_input("Nome", "nome", &app.ps_name, Message::PsName),
        labeled_input("Descrição (opcional)", "opcional…", &app.ps_desc, Message::PsDesc),
        Space::with_height(Length::Fixed(8.0)),
        primary_btn("Salvar Alterações", Message::PsSave),
        Space::with_height(Length::Fixed(14.0)),
        text("─── Zona de Perigo ───").size(13).color(palette::RED),
    ]
    .spacing(6);

    if can_delete {
        col = col.push(muted("Nenhum serviço. O projeto pode ser removido."));
        col = col.push(danger_btn("Remover Projeto", Message::AskDelete(ConfirmAction::DeleteProject(project_id))));
    } else {
        col = col.push(
            text(format!("Este projeto possui {svc_count} serviço(s). Remova-os primeiro."))
                .size(13)
                .color(palette::YELLOW),
        );
    }

    panel("Settings", col.into())
}

// ── Local helper ──────────────────────────────────────────────────────────────

fn panel_with_header<'a>(
    header: iced::widget::Row<'a, Message>,
    body: Element<'a, Message>,
) -> Element<'a, Message> {
    iced::widget::container(
        column![header, Space::with_height(Length::Fixed(8.0)), body]
            .spacing(2)
            .height(Length::Fill),
    )
    .padding(12)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(iced::widget::container::rounded_box)
    .into()
}
