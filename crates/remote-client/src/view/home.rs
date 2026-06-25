//! Home screens: Deployments table, Deploy Engine dashboard, Monitoring.

use super::widgets::*;
use crate::model::palette;
use crate::{App, Message};
use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Alignment, Color, Element, Length};
use shared::{ActiveDeployInfo, DeployState};

pub fn deployments(app: &App) -> Element<'_, Message> {
    if app.home_deployments.is_empty() {
        let msg = if app.projects.is_empty() {
            "Nenhum projeto cadastrado ainda."
        } else {
            "Nenhum deployment encontrado."
        };
        return panel("Deployments", muted(msg));
    }

    let header = row![
        cell("Serviço", 0.24, palette::GRAY),
        cell("Projeto", 0.20, palette::GRAY),
        cell("Estado", 0.22, palette::GRAY),
        cell("Duração", 0.16, palette::GRAY),
        cell("Início", 0.18, palette::GRAY),
    ]
    .spacing(16)
    .padding([0, 4]);

    let mut rows = column![header].spacing(2);
    for s in &app.home_deployments {
        let dep = &s.deployment;
        let (lbl, color) = deploy_state_display(&dep.state);
        let duration = dep
            .finished_at
            .map(|f| fmt_dur((f - dep.started_at).num_seconds()))
            .unwrap_or_else(|| fmt_dur((chrono::Utc::now() - dep.started_at).num_seconds()));
        let started = dep.started_at.format("%H:%M:%S").to_string();
        rows = rows.push(
            container(
                row![
                    cell(&s.service_name, 0.24, palette::CYAN),
                    cell(&s.project_name, 0.20, palette::WHITE),
                    cell(lbl, 0.22, color),
                    cell(&duration, 0.16, palette::GRAY),
                    cell(&started, 0.18, palette::GRAY),
                ]
                .spacing(16)
                .align_y(Alignment::Center),
            )
            .padding([8, 4]),
        );
    }

    panel("Deployments", scrollable(rows).height(Length::Fill).into())
}

fn cell<'a>(s: &str, portion: f32, color: Color) -> Element<'a, Message> {
    container(text(s.to_string()).size(14).color(color))
        .width(Length::FillPortion((portion * 100.0) as u16))
        .into()
}

pub fn deploy_engine(app: &App) -> Element<'_, Message> {
    let Some(s) = &app.deploy_engine else {
        return panel("Deploy Engine", muted("Carregando…"));
    };

    let cards = row![
        stat_card("Ativos", &s.active.len().to_string(), palette::YELLOW),
        stat_card("Sucesso 24h", &s.successful_24h.to_string(), palette::GREEN),
        stat_card("Falhas 24h", &s.failed_24h.to_string(), if s.failed_24h > 0 { palette::RED } else { palette::GRAY }),
        stat_card("Total 24h", &s.total_24h.to_string(), palette::GRAY),
        stat_card("Uptime", &super::fmt_uptime(s.uptime_secs), palette::CYAN),
    ]
    .spacing(14);

    let mut active = column![section("Executando agora")].spacing(6);
    if s.active.is_empty() {
        active = active.push(muted("Nenhum deploy em progresso."));
    } else {
        for info in &s.active {
            active = active.push(active_row(info));
        }
    }

    let mut recent = column![section("Histórico 24h")].spacing(6);
    if s.recent.is_empty() {
        recent = recent.push(muted("Nenhum deploy concluído nas últimas 24h."));
    } else {
        let now = chrono::Utc::now();
        for info in &s.recent {
            recent = recent.push(recent_row(info, now));
        }
    }

    panel(
        "Deploy Engine",
        column![
            cards,
            Space::with_height(Length::Fixed(20.0)),
            active,
            Space::with_height(Length::Fixed(20.0)),
            scrollable(recent).height(Length::Fill),
        ]
        .spacing(8)
        .into(),
    )
}

fn stat_card<'a>(label: &str, value: &str, color: Color) -> Element<'a, Message> {
    container(
        column![
            text(value.to_string())
                .size(30)
                .color(color)
                .wrapping(text::Wrapping::None),
            text(label.to_string())
                .size(12)
                .color(palette::GRAY)
                .wrapping(text::Wrapping::None),
        ]
        .spacing(6),
    )
    .padding(20)
    .width(Length::FillPortion(1))
    .style(container::rounded_box)
    .into()
}

fn active_row(info: &ActiveDeployInfo) -> Element<'static, Message> {
    let bar = progress_bar(info.percent, 18);
    let color = match info.state {
        DeployState::RollingBack | DeployState::Failed => palette::RED,
        DeployState::HealthcheckPolling | DeployState::Staging => palette::YELLOW,
        DeployState::Live => palette::GREEN,
        _ => palette::CYAN,
    };
    row![
        text(info.service_name.clone()).size(13).color(palette::WHITE).width(Length::Fixed(150.0)),
        text(format!("[{}]", info.project_name)).size(12).color(palette::GRAY).width(Length::Fixed(130.0)),
        text(bar).size(13).color(color),
        text(format!("{:>3}%", info.percent)).size(12).color(palette::WHITE),
        text(info.state.label().to_string()).size(12).color(palette::YELLOW).width(Length::Fixed(150.0)),
        text(format!("total {}", fmt_secs(info.elapsed_secs))).size(11).color(palette::GRAY),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn recent_row(info: &ActiveDeployInfo, now: chrono::DateTime<chrono::Utc>) -> Element<'static, Message> {
    let (icon, color) = match info.state {
        DeployState::Live => ("✓", palette::GREEN),
        DeployState::Failed => ("✕", palette::RED),
        _ => ("○", palette::GRAY),
    };
    let ago = fmt_secs(((now - info.started_at).num_seconds()).max(0) as u64);
    row![
        text(icon).size(13).color(color).width(Length::Fixed(20.0)),
        text(info.service_name.clone()).size(13).color(palette::WHITE).width(Length::Fixed(150.0)),
        text(format!("[{}]", info.project_name)).size(12).color(palette::GRAY).width(Length::Fixed(130.0)),
        text(info.state.label().to_string()).size(12).color(color).width(Length::Fixed(100.0)),
        text(format!("há {ago}")).size(11).color(palette::GRAY),
        text(format!("duração {}", fmt_secs(info.elapsed_secs))).size(11).color(palette::GRAY),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub fn monitoring(app: &App) -> Element<'_, Message> {
    if app.metrics.is_empty() {
        return panel("Monitoring", muted("Sem métricas recebidas ainda. Abra um serviço Running."));
    }
    let mut col = column![section("Containers")].spacing(4);
    for (sid, buf) in &app.metrics {
        if let Some(m) = buf.last() {
            let name = app
                .services
                .iter()
                .find(|s| s.id == *sid)
                .map(|s| s.spec.name.clone())
                .unwrap_or_else(|| sid.chars().take(8).collect());
            col = col.push(
                row![
                    text(name).size(13).color(palette::CYAN).width(Length::Fixed(220.0)),
                    text(format!("CPU {:.1}%", m.cpu_percent)).size(12).color(palette::WHITE).width(Length::Fixed(120.0)),
                    text(format!("MEM {} MiB", m.mem_used_bytes / (1024 * 1024))).size(12).color(palette::WHITE).width(Length::Fixed(140.0)),
                    text(format!("RX {} KiB", m.net_rx_bytes / 1024)).size(11).color(palette::GRAY),
                    text(format!("TX {} KiB", m.net_tx_bytes / 1024)).size(11).color(palette::GRAY),
                ]
                .spacing(8),
            );
        }
    }
    panel("Monitoring", scrollable(col).height(Length::Fill).into())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn deploy_state_display(state: &DeployState) -> (&'static str, Color) {
    match state {
        DeployState::Live => ("● Live", palette::GREEN),
        DeployState::Stopped => ("○ Stopped", palette::GRAY),
        DeployState::Failed => ("✕ Failed", palette::RED),
        DeployState::RollingBack => ("↩ Rolling back", palette::RED),
        DeployState::Pending => ("◌ Pending", palette::YELLOW),
        DeployState::ResolvingDeps => ("◌ Resolving", palette::YELLOW),
        DeployState::PullingImage => ("◌ Pulling", palette::YELLOW),
        DeployState::CloningRepo => ("◌ Cloning", palette::YELLOW),
        DeployState::BuildingImage => ("◌ Building", palette::YELLOW),
        DeployState::Staging => ("◌ Staging", palette::YELLOW),
        DeployState::HealthcheckPolling => ("◌ Healthcheck", palette::YELLOW),
        DeployState::SwappingIn => ("◌ Swapping", palette::YELLOW),
        DeployState::Draining => ("◌ Draining", palette::YELLOW),
        DeployState::Promoting => ("◌ Promoting", palette::YELLOW),
        DeployState::Pruning => ("◌ Pruning", palette::GRAY),
        DeployState::ComposingUp => ("◌ Composing", palette::YELLOW),
    }
}

fn progress_bar(percent: u8, width: usize) -> String {
    let filled = (width * percent.min(100) as usize) / 100;
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

fn fmt_dur(secs: i64) -> String {
    if secs < 0 {
        return "—".into();
    }
    if secs < 60 {
        format!("{secs}s")
    } else {
        format!("{}m {}s", secs / 60, secs % 60)
    }
}

fn fmt_secs(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}
