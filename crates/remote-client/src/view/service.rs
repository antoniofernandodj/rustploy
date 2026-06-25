//! Service detail screen with all tabs, mirroring the TUI.

use super::widgets::*;
use crate::model::{
    palette, DbKind, GenField, HcField, ProviderChoice, ProviderTab, RepoChoice, ServiceTab,
};
use crate::update::status_color;
use crate::{App, Message};
use iced::widget::{
    button, checkbox, column, container, pick_list, row, scrollable, text, text_editor, text_input,
    Space,
};
use iced::{Alignment, Color, Element, Length};
use chrono::Utc;
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

    // Provider tem duas sub-abas: Git (URL crua) e Gitea (contas conectadas).
    // A sub-aba Gitea só aparece quando há ao menos um provider conectado.
    let has_gitea = !app.git_providers.is_empty();
    let active = if has_gitea { app.provider_tab } else { ProviderTab::Git };
    let subtabs: Vec<(ProviderTab, &str)> = if has_gitea {
        vec![(ProviderTab::Git, "Git"), (ProviderTab::Gitea, "Gitea")]
    } else {
        vec![(ProviderTab::Git, "Git")]
    };

    let body = match active {
        ProviderTab::Git => git_provider_body(app),
        ProviderTab::Gitea => gitea_provider_body(app),
    };

    scrollable(column![
        actions_row(),
        Space::with_height(Length::Fixed(10.0)),
        section("Provider"),
        tab_bar(&subtabs, active, Message::ProviderTabChanged),
        Space::with_height(Length::Fixed(8.0)),
        body,
    ]
    .spacing(6))
    .height(Length::Fill)
    .into()
}

/// The generic Git/registry URL form (Provider → Git sub-tab).
fn git_provider_body(app: &App) -> Element<'_, Message> {
    let g = &app.general;
    column![
        labeled_input("Repository URL / Image", "github.com/user/repo", &g.repo_url, |v| Message::GenField(GenField::RepoUrl, v)),
        labeled_input("Branch", "main", &g.branch, |v| Message::GenField(GenField::Branch, v)),
        labeled_input("Username", "", &g.username, |v| Message::GenField(GenField::Username, v)),
        labeled_input("Credentials (secret)", "", &g.credentials, |v| Message::GenField(GenField::Credentials, v)),
        labeled_input("Build Path", ".", &g.build_path, |v| Message::GenField(GenField::BuildPath, v)),
        labeled_input("Watch Paths", "src, Cargo.toml", &g.watch_paths, |v| Message::GenField(GenField::WatchPaths, v)),
        row![
            container(label_text("Enable Submodules")).width(Length::Fixed(190.0)),
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
    .spacing(6)
    .into()
}

/// The connected-Gitea picker form (Provider → Gitea sub-tab).
fn gitea_provider_body(app: &App) -> Element<'_, Message> {
    let gf = &app.gitea;

    // Conta Gitea (dropdown)
    let providers: Vec<ProviderChoice> = app
        .git_providers
        .iter()
        .map(|p| {
            let login = p.account.as_ref().map(|a| format!(" (@{})", a.login)).unwrap_or_default();
            ProviderChoice { id: p.id.clone(), label: format!("{}{login}", p.name) }
        })
        .collect();
    let selected_provider = gf
        .provider_id
        .as_ref()
        .and_then(|id| providers.iter().find(|c| &c.id == id).cloned());

    // Repositórios (dropdown) — disponível após escolher a conta.
    let repos: Vec<RepoChoice> = app
        .git_repos
        .iter()
        .map(|r| RepoChoice {
            full_name: r.full_name.clone(),
            clone_url: r.clone_url.clone(),
            default_branch: r.default_branch.clone(),
        })
        .collect();
    let selected_repo = repos.iter().find(|r| {
        gf.repo_full_name.as_deref() == Some(&r.full_name) || r.clone_url == gf.clone_url
    });

    // Branches (dropdown) — disponível após escolher o repo.
    let branches: Vec<String> = app.git_branches.iter().map(|b| b.name.clone()).collect();

    let account_row = row![
        container(label_text("Conta Gitea")).width(Length::Fixed(190.0)),
        pick_list(providers, selected_provider, Message::GiteaProviderPick)
            .placeholder("selecione uma conta"),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let repo_row = row![
        container(label_text("Repositório")).width(Length::Fixed(190.0)),
        pick_list(repos.clone(), selected_repo.cloned(), Message::GiteaRepoPick)
            .placeholder(if gf.provider_id.is_some() { "selecione um repositório" } else { "escolha a conta primeiro" }),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let branch_row = row![
        container(label_text("Branch")).width(Length::Fixed(190.0)),
        pick_list(branches, gf.branch.clone(), Message::GiteaBranchPick)
            .placeholder("selecione a branch"),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // Watch Paths como array editável (linha por path).
    let mut watch_col = column![row![
        container(label_text("Watch Paths")).width(Length::Fixed(190.0)),
        ghost_btn("+ Adicionar", Message::GiteaWatchAdd),
    ]
    .spacing(8)
    .align_y(Alignment::Center)]
    .spacing(6);
    if gf.watch_paths.is_empty() {
        watch_col = watch_col.push(row![
            Space::with_width(Length::Fixed(190.0)),
            muted("Nenhum path — dispara em qualquer mudança."),
        ].spacing(8));
    }
    for (i, p) in gf.watch_paths.iter().enumerate() {
        watch_col = watch_col.push(
            row![
                Space::with_width(Length::Fixed(190.0)),
                text_input("src/", p).on_input(move |v| Message::GiteaWatch(i, v)).padding(6).size(13),
                danger_btn("✕", Message::GiteaWatchDelete(i)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    column![
        muted("Selecione a conta conectada, o repositório e a branch a implantar."),
        account_row,
        repo_row,
        branch_row,
        Space::with_height(Length::Fixed(8.0)),
        section("Build"),
        labeled_input("Build Path", ".", &gf.build_path, Message::GiteaBuildPath),
        labeled_input("Dockerfile", "Dockerfile", &gf.dockerfile, Message::GiteaDockerfile),
        row![
            container(label_text("Enable Submodules")).width(Length::Fixed(190.0)),
            checkbox("", gf.submodules).on_toggle(Message::GiteaSubmodules),
        ].spacing(8).align_y(Alignment::Center),
        labeled_input("Port", "80", &gf.port, Message::GiteaPort),
        Space::with_height(Length::Fixed(8.0)),
        watch_col,
        Space::with_height(Length::Fixed(10.0)),
        primary_btn("Save", Message::GiteaSave),
    ]
    .spacing(6)
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
    let text_btn_label = if app.s_env_text_open { "Fechar .env" } else { ".env" };
    let mut col = column![
        row![
            section("Environment Variables"),
            Space::with_width(Length::Fill),
            ghost_btn("Exportar", Message::SEnvExport),
            ghost_btn(text_btn_label, Message::SEnvTextOpen),
            ghost_btn("+ Adicionar", Message::SEnvOpen),
        ].spacing(6).align_y(Alignment::Center),
    ]
    .spacing(6);

    if app.s_env_text_open {
        col = col.push(
            column![
                muted("Cole ou edite no formato KEY=VALUE (# para comentários). Importar substitui todas as variáveis."),
                text_editor(&app.s_env_text_editor)
                    .on_action(Message::SEnvTextAction)
                    .height(Length::Fixed(200.0)),
                row![
                    primary_btn("Importar", Message::SEnvImport),
                    ghost_btn("Cancelar", Message::SEnvTextOpen),
                ].spacing(8),
            ]
            .spacing(6),
        );
    } else if app.s_env_editor.open {
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
                    button(text("✕").size(11)).on_press(Message::SEnvDelete(i)).style(button::danger).padding([2, 6]),
                    text(ev.key.clone()).size(13).color(palette::CYAN).width(Length::Fixed(200.0)),
                    text("=").size(13).color(palette::GRAY),
                    text(val).size(13).color(palette::WHITE).width(Length::Fill),
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

    let now = Utc::now();
    let mut list = column![].spacing(3);
    for (i, dep) in app.service_deployments.iter().enumerate() {
        let selected = i == app.selected_deployment;
        let duration = match dep.finished_at {
            Some(f) => fmt_duration((f - dep.started_at).num_seconds().max(0) as u64),
            None => format!("⏱ {}", fmt_duration((now - dep.started_at).num_seconds().max(0) as u64)),
        };
        let (lbl, color) = super::home::deploy_state_display(&dep.state);
        list = list.push(
            button(
                row![
                    text(dep.id.chars().take(12).collect::<String>()).size(12).color(palette::CYAN).width(Length::Fixed(120.0)),
                    text(lbl).size(12).color(color).width(Length::Fixed(130.0)),
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
    let ago = fmt_ago((now - dep.started_at).num_seconds().max(0) as u64);
    let duration_val = match dep.finished_at {
        Some(f) => fmt_duration((f - dep.started_at).num_seconds().max(0) as u64),
        None => format!("⏱ {} (em andamento)", fmt_duration((now - dep.started_at).num_seconds().max(0) as u64)),
    };
    let mut detail = column![
        labeled_static("ID", dep.id.clone()),
        labeled_static("Imagem", dep.image.clone()),
        labeled_static("Iniciado", ago),
        labeled_static("Duração", duration_val),
    ]
    .spacing(2);

    if let Some(url) = &app.webhook_url {
        detail = detail.push(Space::with_height(Length::Fixed(6.0)));
        detail = detail.push(section("Webhook"));
        detail = detail.push(text(url.clone()).size(12).color(palette::CYAN));
        detail = detail.push(ghost_btn("Regenerar token", Message::WebhookRegen));
    }

    let has_logs = app.build_logs.get(&dep.id).map(|b| !b.is_empty()).unwrap_or(false);
    let open_logs_btn = row![
        Space::with_width(Length::Fill),
        ghost_btn(
            if has_logs { "Ver build log →" } else { "Build log (vazio)" },
            Message::BuildLogModal(true),
        ),
    ]
    .align_y(Alignment::Center);

    panel(
        "Deployments",
        column![
            container(scrollable(list)).max_height(160),
            Space::with_height(Length::Fixed(8.0)),
            detail,
            Space::with_height(Length::Fixed(4.0)),
            open_logs_btn,
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
    let full: String = app
        .logs
        .get(sid)
        .map(|buf| {
            buf.iter()
                .map(|l| format!("{} {}", l.timestamp.format("%H:%M:%S%.3f"), l.text))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();
    use iced::widget::text_editor::{Action, Motion};
    let header = row![
        Space::with_width(Length::Fill),
        ghost_btn("↑ Topo", Message::LogAction(Action::Move(Motion::DocumentStart))),
        ghost_btn("↓ Fim", Message::LogAction(Action::Move(Motion::DocumentEnd))),
        copy_all_btn(full.clone()),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    // Read-only: o texto pode ser selecionado e copiado (Ctrl+C) além do botão.
    let body: Element<'_, Message> = if full.is_empty() {
        muted("Aguardando logs... (serviço precisa estar Running)")
    } else {
        text_editor(&app.log_editor)
            .on_action(Message::LogAction)
            .size(11)
            .height(Length::Fill)
            .padding(8)
            .into()
    };
    panel("Logs", column![header, body].spacing(6).height(Length::Fill).into())
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

/// Conteúdo do modal de build log (aberto por Message::BuildLogModal(true)).
pub fn build_log_modal_content(app: &App) -> Element<'_, Message> {
    if app.service_deployments.is_empty() {
        return column![
            text("Build Log").size(18).color(palette::CYAN),
            Space::with_height(Length::Fixed(12.0)),
            muted("Nenhum deployment selecionado."),
            Space::with_height(Length::Fixed(16.0)),
            ghost_btn("Fechar", Message::BuildLogModal(false)),
        ]
        .spacing(8)
        .into();
    }

    let dep = &app.service_deployments
        [app.selected_deployment.min(app.service_deployments.len() - 1)];
    let (state_lbl, state_color) = super::home::deploy_state_display(&dep.state);
    let logs = app.build_logs.get(&dep.id).map(|b| b.as_slice()).unwrap_or(&[]);
    let build_text: String = logs.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");

    let header = row![
        text(format!("Build Log — {}", dep.id.chars().take(12).collect::<String>()))
            .size(16)
            .color(palette::CYAN),
        text(state_lbl).size(13).color(state_color),
        Space::with_width(Length::Fill),
        ghost_btn("↑ Topo", Message::BuildLogScrollTo(0.0)),
        ghost_btn("↓ Fim", Message::BuildLogScrollTo(f32::MAX)),
        copy_all_btn(build_text),
        ghost_btn("✕ Fechar", Message::BuildLogModal(false)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let body: Element<'_, Message> = if logs.is_empty() {
        muted("Sem logs de build para este deployment.")
    } else {
        scrollable(
            column(
                logs.iter()
                    .map(|log| render_ansi_line(&log.text))
                    .collect::<Vec<_>>(),
            )
            .padding(8),
        )
        .id(scrollable::Id::new("build_log"))
        .height(Length::Fixed(520.0))
        .into()
    };

    column![header, Space::with_height(Length::Fixed(10.0)), body]
        .spacing(4)
        .into()
}

fn render_ansi_line(line: &str) -> Element<'static, Message> {
    let spans = parse_ansi_spans(line);
    if spans.is_empty() {
        return text("").size(11).into();
    }
    iced::widget::rich_text(
        spans
            .into_iter()
            .map(|(color, txt)| {
                let s = iced::widget::span(txt).size(11.0);
                match color {
                    Some(c) => s.color(c),
                    None => s,
                }
            })
            .collect::<Vec<_>>(),
    )
    .into()
}

fn parse_ansi_spans(line: &str) -> Vec<(Option<Color>, String)> {
    let mut spans: Vec<(Option<Color>, String)> = Vec::new();
    let mut current_color: Option<Color> = None;
    let mut current_text = String::new();
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            let mut code = String::new();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    if next == 'm' && !current_text.is_empty() {
                        spans.push((current_color, std::mem::take(&mut current_text)));
                        current_color = ansi_code_to_color(&code).unwrap_or(current_color);
                    }
                    break;
                }
                code.push(next);
            }
        } else {
            current_text.push(c);
        }
    }
    if !current_text.is_empty() {
        spans.push((current_color, current_text));
    }
    spans
}

fn ansi_code_to_color(code: &str) -> Option<Option<Color>> {
    let last = code.split(';').filter_map(|s| s.parse::<u8>().ok()).last()?;
    Some(match last {
        0 => None,
        30 => Some(Color::from_rgb(0.3, 0.3, 0.3)),
        31 => Some(Color::from_rgb(0.8, 0.2, 0.2)),
        32 => Some(Color::from_rgb(0.2, 0.7, 0.2)),
        33 => Some(Color::from_rgb(0.8, 0.7, 0.2)),
        34 => Some(Color::from_rgb(0.2, 0.4, 0.9)),
        35 => Some(Color::from_rgb(0.7, 0.2, 0.9)),
        36 => Some(Color::from_rgb(0.2, 0.7, 0.8)),
        37 => Some(Color::from_rgb(0.9, 0.9, 0.9)),
        90 => Some(Color::from_rgb(0.5, 0.5, 0.5)),
        91 => Some(Color::from_rgb(1.0, 0.4, 0.4)),
        92 => Some(Color::from_rgb(0.4, 1.0, 0.4)),
        93 => Some(Color::from_rgb(1.0, 1.0, 0.4)),
        94 => Some(Color::from_rgb(0.4, 0.6, 1.0)),
        95 => Some(Color::from_rgb(1.0, 0.4, 1.0)),
        96 => Some(Color::from_rgb(0.4, 1.0, 1.0)),
        97 => Some(Color::from_rgb(1.0, 1.0, 1.0)),
        _ => return None,
    })
}

/// Formata uma duração em segundos como "Xm Ys" ou "Xh Ym" etc.
fn fmt_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

/// Formata quantos segundos atrás como "há Xm Ys" etc.
fn fmt_ago(secs: u64) -> String {
    if secs < 5 {
        "agora mesmo".into()
    } else if secs < 60 {
        format!("há {secs}s")
    } else if secs < 3600 {
        format!("há {}m {}s", secs / 60, secs % 60)
    } else if secs < 86400 {
        format!("há {}h {}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("há {}d", secs / 86400)
    }
}
