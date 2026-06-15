//! Service detail screen with all tabs, mirroring the TUI.

use super::widgets::*;
use crate::model::{palette, DbKind, HcField, GenField, ServiceTab};
use crate::update::status_color;
use crate::{App, Message};
use iced::widget::{button, checkbox, column, row, scrollable, text, text_editor, text_input, Space};
use iced::{Alignment, Element, Length};
use shared::{EnvVarValue, ServiceSource};

pub fn detail(app: &App) -> Element<'_, Message> {
    let Some(svc) = app.current_service() else {
        return panel("Service", text("Nenhum serviço selecionado.").size(13).into());
    };

    let is_db = DbKind::detect_from_env(&svc.spec.env_vars).is_some();
    let mut tabs: Vec<(ServiceTab, &str)> = vec![(ServiceTab::General, "General")];
    if is_db {
        tabs.push((ServiceTab::Connection, "Connection"));
    }
    tabs.extend([
        (ServiceTab::Environment, "Environment"),
        (ServiceTab::Domains, "Domains"),
        (ServiceTab::Deployments, "Deployments"),
        (ServiceTab::Healthcheck, "Healthcheck"),
        (ServiceTab::Logs, "Logs"),
        (ServiceTab::Patches, "Patches"),
        (ServiceTab::Advanced, "Advanced"),
    ]);

    let header = row![
        ghost_btn("‹ Voltar", Message::BackToProject),
        text(svc.spec.name.clone()).size(18).color(palette::CYAN),
        text(format!(" {} ", svc.status)).size(12).color(status_color(&svc.status)),
        Space::with_width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let body = match app.service_tab {
        ServiceTab::General => general(app, svc),
        ServiceTab::Connection => connection(app),
        ServiceTab::Environment => environment(app, svc),
        ServiceTab::Domains => domains(app),
        ServiceTab::Deployments => deployments(app),
        ServiceTab::Healthcheck => healthcheck(app),
        ServiceTab::Logs => logs(app),
        ServiceTab::Patches => placeholder("Patches", "Histórico de patches de configuração (v2)."),
        ServiceTab::Advanced => advanced(app),
    };

    column![
        header,
        Space::with_height(Length::Fixed(6.0)),
        tab_bar(&tabs, app.service_tab, Message::ServiceTab),
        Space::with_height(Length::Fixed(8.0)),
        iced::widget::container(body).width(Length::Fill).height(Length::Fill),
    ]
    .spacing(2)
    .height(Length::Fill)
    .into()
}

fn actions_row() -> Element<'static, Message> {
    row![
        success_btn("Deploy", Message::SvcDeploy),
        ghost_btn("Reload", Message::SvcReload),
        ghost_btn("Rebuild", Message::SvcDeploy),
        danger_btn("Stop", Message::SvcStop),
    ]
    .spacing(8)
    .into()
}

fn general<'a>(app: &'a App, svc: &'a shared::Service) -> Element<'a, Message> {
    if matches!(svc.spec.source, ServiceSource::Compose(_)) {
        return column![
            actions_row(),
            Space::with_height(Length::Fixed(10.0)),
            section("Compose YAML"),
            text_editor(&app.compose_editor)
                .on_action(Message::ComposeAction)
                .height(Length::Fill)
                .padding(8),
            row![
                muted(format!("{} linhas", app.compose_editor.line_count())),
                Space::with_width(Length::Fill),
                primary_btn("Salvar Compose", Message::ComposeSave),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(8)
        .height(Length::Fill)
        .into();
    }

    let g = &app.general;
    scrollable(column![
        actions_row(),
        Space::with_height(Length::Fixed(10.0)),
        section(if g.is_git { "Provider: Git" } else { "Provider: Registry" }),
        labeled_input("Repository URL / Image", "github.com/user/repo", &g.repo_url, |v| Message::GenField(GenField::RepoUrl, v)),
        labeled_input("Branch", "main", &g.branch, |v| Message::GenField(GenField::Branch, v)),
        labeled_input("Username", "", &g.username, |v| Message::GenField(GenField::Username, v)),
        labeled_input("Credentials (secret)", "", &g.credentials, |v| Message::GenField(GenField::Credentials, v)),
        labeled_input("Build Path", ".", &g.build_path, |v| Message::GenField(GenField::BuildPath, v)),
        labeled_input("Watch Paths", "src, Cargo.toml", &g.watch_paths, |v| Message::GenField(GenField::WatchPaths, v)),
        row![
            iced::widget::container(label_text("Enable Submodules")).width(Length::Fixed(190.0)),
            checkbox("", g.submodules).on_toggle(Message::GenSubmodules),
        ].spacing(8).align_y(Alignment::Center),
        labeled_input("Port", "80", &g.port, |v| Message::GenField(GenField::Port, v)),
        Space::with_height(Length::Fixed(8.0)),
        section("Build Type: Dockerfile"),
        labeled_input("Docker File", "Dockerfile", &g.dockerfile, |v| Message::GenField(GenField::Dockerfile, v)),
        labeled_input("Docker Context Path", ".", &g.context_path, |v| Message::GenField(GenField::ContextPath, v)),
        labeled_input("Docker Build Stage", "", &g.build_stage, |v| Message::GenField(GenField::BuildStage, v)),
        Space::with_height(Length::Fixed(8.0)),
        primary_btn("Save", Message::GenSave),
    ]
    .spacing(6))
    .height(Length::Fill)
    .into()
}

fn connection(app: &App) -> Element<'_, Message> {
    let Some(ci) = &app.conn_info else {
        return panel("Connection", text("Não é um serviço de banco de dados.").size(13).into());
    };

    let mut col = column![
        row![
            text(format!(" {} ", ci.db_label)).size(13).color(palette::CYAN),
            muted(format!("{} gerenciado pelo Rustploy", ci.db_label)),
        ]
        .spacing(8),
        Space::with_height(Length::Fixed(8.0)),
        section("Conexão interna"),
        copyable_row("Host", &ci.host),
        labeled_static("Port", ci.port.clone()),
    ]
    .spacing(4);
    for (k, v) in &ci.fields {
        col = col.push(copyable_row(k, v));
    }
    col = col.push(Space::with_height(Length::Fixed(8.0)));
    col = col.push(section("URL de conexão"));
    col = col.push(copyable_row("URL", &ci.url));
    col = col.push(muted("Use este hostname em outros serviços do mesmo projeto."));
    panel("Connection", col.into())
}

/// A read-only, selectable (Ctrl+C) value field with a Copiar button.
fn copyable_row<'a>(label: &'a str, value: &'a str) -> Element<'a, Message> {
    row![
        iced::widget::container(text(label.to_string()).size(13).color(palette::GRAY))
            .width(Length::Fixed(110.0)),
        text_input("", value).on_input(|_| Message::Ignore).size(13).padding(4),
        button(text("Copiar").size(12))
            .on_press(Message::Copy(value.to_string()))
            .style(button::secondary)
            .padding([4, 8]),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

fn labeled_static(label: &str, value: String) -> Element<'static, Message> {
    row![
        iced::widget::container(text(label.to_string()).size(13).color(palette::GRAY)).width(Length::Fixed(110.0)),
        text(value).size(13).color(palette::WHITE),
    ]
    .spacing(6)
    .into()
}

fn environment<'a>(app: &'a App, svc: &'a shared::Service) -> Element<'a, Message> {
    let mut col = column![
        row![
            section("Environment Variables"),
            Space::with_width(Length::Fill),
            ghost_btn("+ Adicionar", Message::SEnvOpen),
        ].align_y(Alignment::Center),
    ]
    .spacing(6);

    if app.s_env_editor.open {
        col = col.push(kv_form(
            &app.s_env_editor.key,
            &app.s_env_editor.value,
            Message::SEnvKey,
            Message::SEnvVal,
            Message::SEnvSubmit,
            Message::SEnvCancel,
            false,
        ));
    }

    if svc.spec.env_vars.is_empty() {
        col = col.push(muted("Nenhuma variável."));
    } else {
        for (i, ev) in svc.spec.env_vars.iter().enumerate() {
            let val = match &ev.value {
                EnvVarValue::Plain(v) => v.clone(),
                EnvVarValue::Secret(s) => format!("<secret:{s}>"),
            };
            col = col.push(
                row![
                    text(ev.key.clone()).size(13).color(palette::CYAN).width(Length::Fixed(220.0)),
                    text("=").size(13).color(palette::GRAY),
                    text(val).size(13).color(palette::WHITE).width(Length::Fill),
                    button(text("✕").size(12)).on_press(Message::SEnvDelete(i)).style(button::danger).padding([2, 8]),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }
    panel("Environment", scrollable(col).height(Length::Fill).into())
}

fn domains(app: &App) -> Element<'_, Message> {
    let d = &app.domains;
    panel(
        "Domains",
        column![
            section("Roteamento"),
            labeled_input("Domínio", "app.exemplo.com", &d.domain, Message::DomDomain),
            labeled_input("Porta externa", "(padrão do serviço)", &d.host_port, Message::DomHostPort),
            row![
                iced::widget::container(label_text("HTTPS / TLS")).width(Length::Fixed(190.0)),
                checkbox("requer domínio", d.tls_enabled).on_toggle(Message::DomTls),
            ].spacing(8).align_y(Alignment::Center),
            Space::with_height(Length::Fixed(8.0)),
            primary_btn("Save", Message::DomSave),
        ]
        .spacing(6)
        .into(),
    )
}

fn healthcheck(app: &App) -> Element<'_, Message> {
    let h = &app.health;
    let kinds = ["None", "Tcp", "Http", "DockerNative"];
    let picker = iced::widget::pick_list(kinds.to_vec(), Some(h.kind.as_str()), |s| {
        Message::HcKind(s.to_string())
    })
    .padding(6)
    .text_size(13);

    panel(
        "Healthcheck",
        column![
            row![
                iced::widget::container(label_text("Kind")).width(Length::Fixed(190.0)),
                picker,
            ].spacing(8).align_y(Alignment::Center),
            section("HTTP options"),
            labeled_input("HTTP Path", "/health", &h.http_path, |v| Message::HcField(HcField::HttpPath, v)),
            labeled_input("Expected Status", "200", &h.expected_status, |v| Message::HcField(HcField::ExpectedStatus, v)),
            section("Timing"),
            labeled_input("Interval (s)", "5", &h.interval, |v| Message::HcField(HcField::Interval, v)),
            labeled_input("Timeout (s)", "3", &h.timeout, |v| Message::HcField(HcField::Timeout, v)),
            labeled_input("Retries", "10", &h.retries, |v| Message::HcField(HcField::Retries, v)),
            labeled_input("Start Period (s)", "5", &h.start_period, |v| Message::HcField(HcField::StartPeriod, v)),
            Space::with_height(Length::Fixed(8.0)),
            primary_btn("Save", Message::HcSave),
        ]
        .spacing(6)
        .into(),
    )
}

fn advanced(app: &App) -> Element<'_, Message> {
    let a = &app.advanced;
    let mut args = column![
        row![
            section("Args"),
            Space::with_width(Length::Fill),
            ghost_btn("+ Arg", Message::AdvArgAdd),
        ].align_y(Alignment::Center),
    ]
    .spacing(4);
    if a.run_args.is_empty() {
        args = args.push(muted("Nenhum argumento."));
    } else {
        for (i, arg) in a.run_args.iter().enumerate() {
            args = args.push(
                row![
                    text_input("arg", arg).on_input(move |v| Message::AdvArg(i, v)).padding(5).size(13),
                    button(text("✕").size(12)).on_press(Message::AdvArgDelete(i)).style(button::danger).padding([2, 8]),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
        }
    }

    panel(
        "Advanced",
        column![
            section("Scaling"),
            labeled_input("Replicas", "1", &a.replicas, Message::AdvReplicas),
            section("Run Command"),
            muted("Comando custom executado no container após o início."),
            labeled_input("Command", "/bin/sh", &a.run_command, Message::AdvCommand),
            Space::with_height(Length::Fixed(6.0)),
            args,
            Space::with_height(Length::Fixed(8.0)),
            primary_btn("Save", Message::AdvSave),
        ]
        .spacing(6)
        .into(),
    )
}

fn deployments(app: &App) -> Element<'_, Message> {
    if app.service_deployments.is_empty() {
        return panel("Deployments", column![
            muted("Nenhum deployment recente."),
            muted("Use o botão Deploy na aba General para iniciar um deploy."),
        ].spacing(6).into());
    }

    let mut list = column![].spacing(3);
    for (i, dep) in app.service_deployments.iter().enumerate() {
        let selected = i == app.selected_deployment;
        let duration = dep
            .finished_at
            .map(|f| format!("{}s", (f - dep.started_at).num_seconds()))
            .unwrap_or_else(|| "em andamento".into());
        let (lbl, color) = super::home::deploy_state_display(&dep.state);
        list = list.push(
            button(
                row![
                    text(dep.id.chars().take(12).collect::<String>()).size(12).color(palette::CYAN).width(Length::Fixed(120.0)),
                    text(lbl).size(12).color(color).width(Length::Fixed(140.0)),
                    text(duration).size(12).color(palette::GRAY),
                ]
                .spacing(8),
            )
            .on_press(Message::DeploySelect(i))
            .width(Length::Fill)
            .style(if selected { button::primary } else { button::text })
            .padding([3, 6]),
        );
    }

    let dep = &app.service_deployments[app.selected_deployment.min(app.service_deployments.len() - 1)];
    let mut detail = column![
        labeled_static("ID", dep.id.clone()),
        labeled_static("Imagem", dep.image.clone()),
    ]
    .spacing(2);

    if let Some(url) = &app.webhook_url {
        detail = detail.push(Space::with_height(Length::Fixed(6.0)));
        detail = detail.push(section("Webhook"));
        detail = detail.push(text(url.clone()).size(12).color(palette::CYAN));
        detail = detail.push(ghost_btn("Regenerar token", Message::WebhookRegen));
    }

    // Build logs
    let mut logs = column![section("Build log")].spacing(1);
    if let Some(buf) = app.build_logs.get(&dep.id) {
        for l in buf.iter() {
            logs = logs.push(text(l.text.clone()).size(11).color(palette::GRAY));
        }
    } else {
        logs = logs.push(muted("Sem logs de build para este deployment."));
    }

    panel(
        "Deployments",
        column![
            list,
            Space::with_height(Length::Fixed(8.0)),
            detail,
            Space::with_height(Length::Fixed(8.0)),
            scrollable(logs).height(Length::Fill),
        ]
        .spacing(4)
        .into(),
    )
}

fn logs(app: &App) -> Element<'_, Message> {
    let sid = match &app.active_service_id {
        Some(s) => s,
        None => return panel("Logs", text("—").into()),
    };
    let lines = app.logs.get(sid);
    let mut col = column![].spacing(0);
    match lines {
        Some(buf) if !buf.is_empty() => {
            for l in buf.iter() {
                let ts = l.timestamp.format("%H:%M:%S%.3f").to_string();
                let color = if l.is_stderr { palette::RED } else { palette::WHITE };
                col = col.push(
                    row![
                        text(ts).size(11).color(palette::GRAY),
                        text(l.text.clone()).size(11).color(color),
                    ]
                    .spacing(6),
                );
            }
        }
        _ => {
            col = col.push(muted("Aguardando logs... (serviço precisa estar Running)"));
        }
    }
    panel("Logs", scrollable(col).height(Length::Fill).into())
}

/// Inline key/value editor row used by env and secret panels.
pub fn kv_form<'a>(
    key: &'a str,
    value: &'a str,
    on_key: impl Fn(String) -> Message + 'a,
    on_val: impl Fn(String) -> Message + 'a,
    submit: Message,
    cancel: Message,
    secret: bool,
) -> Element<'a, Message> {
    let val_input = if secret {
        text_input("value", value).on_input(on_val).secure(true).padding(6).size(13)
    } else {
        text_input("value", value).on_input(on_val).padding(6).size(13)
    };
    row![
        text_input("KEY", key).on_input(on_key).padding(6).size(13).width(Length::Fixed(220.0)),
        val_input,
        primary_btn("Salvar", submit),
        ghost_btn("Cancelar", cancel),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}
