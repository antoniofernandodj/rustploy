//! Network bridge between the daemon (RWP) and the glacier-ui context.
//!
//! Two halves:
//! - [`poll_stream`] — a long-lived `Subscription` source the root component
//!   hands to the engine. It connects, then periodically pulls daemon status,
//!   recent deployments and projects, emitting them as
//!   `EngineMessage::ContextPatch` so the templates re-render. It also relays
//!   live daemon events.
//! - [`run_command`] — a one-shot RPC used by `ctx.perform` effects (connect
//!   test, deploy, stop, …).

use crate::rwp;
use chrono::{Local, Utc};
use glacier_ui::EngineMessage;
use iced::futures::{SinkExt, Stream};
use shared::{Command, DeploymentSummary, DeployState, Project, Response, RwpFrame};

/// One-shot: open a fresh connection, run `cmd`, return the response.
/// Used by `ctx.perform` action effects (deploy/stop/reload) as screens land.
#[allow(dead_code)]
pub async fn run_command(
    addr: String,
    token: Option<String>,
    cmd: Command,
) -> anyhow::Result<Response> {
    let mut conn = rwp::connect(&addr, token.as_deref()).await?;
    rwp::rpc(&mut conn, cmd).await
}

/// Long-lived polling + event stream feeding the context. Yields
/// `EngineMessage::ContextPatch` items.
pub fn poll_stream(addr: String, token: Option<String>) -> impl Stream<Item = EngineMessage> {
    iced::stream::channel(64, move |mut output| async move {
        macro_rules! patch {
            ($pairs:expr) => {
                let _ = output.send(EngineMessage::ContextPatch($pairs)).await;
            };
        }

        // Command connection (RPC polling) + event connection (live updates).
        let mut cmd = match rwp::connect(&addr, token.as_deref()).await {
            Ok(s) => s,
            Err(e) => {
                patch!(vec![
                    ("connected".into(), "false".into()),
                    ("screen".into(), "login".into()),
                    ("error".into(), e.to_string()),
                    ("status_line".into(), "falha na conexão".into()),
                ]);
                return;
            }
        };
        let mut evt = match rwp::connect(&addr, token.as_deref()).await {
            Ok(s) => s,
            Err(_) => return,
        };
        let _ = rwp::write_frame(&mut evt, &RwpFrame::Subscribe { service_id: None }).await;

        patch!(vec![
            ("connected".into(), "true".into()),
            ("screen".into(), "shell".into()),
            ("error".into(), String::new()),
            ("status_line".into(), "conectado".into()),
        ]);

        let mut poll = tokio::time::interval(std::time::Duration::from_secs(2));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = poll.tick() => {
                    let mut pairs = Vec::new();
                    if let Ok(Response::DaemonStatus(d)) = rwp::rpc(&mut cmd, Command::DaemonStatus).await {
                        pairs.push(("daemon_version".into(), d.version.clone()));
                        pairs.push(("daemon_uptime".into(), fmt_uptime(d.uptime_secs)));
                        pairs.push(("services_running".into(), d.services_running.to_string()));
                        pairs.push(("services_total".into(), d.services_total.to_string()));
                        pairs.push(("services_label".into(), format!("{}/{}", d.services_running, d.services_total)));
                    }
                    if let Ok(Response::DeploymentSummaries(list)) =
                        rwp::rpc(&mut cmd, Command::RecentDeployments { limit: 40 }).await
                    {
                        pairs.push(("deployments".into(), deployments_json(&list)));
                        pairs.push(("deployments_count".into(), list.len().to_string()));
                    }
                    if let Ok(Response::Projects(list)) = rwp::rpc(&mut cmd, Command::ProjectList).await {
                        pairs.push(("projects".into(), projects_json(&list)));
                        pairs.push(("projects_count".into(), list.len().to_string()));
                    }
                    if !pairs.is_empty() {
                        patch!(pairs);
                    }
                }
                frame = rwp::read_frame::<shared::RwpReply>(&mut evt) => match frame {
                    Ok(_) => { /* live event: next poll picks up fresh data */ }
                    Err(_) => {
                        patch!(vec![
                            ("connected".into(), "false".into()),
                            ("screen".into(), "login".into()),
                            ("status_line".into(), "conexão encerrada".into()),
                        ]);
                        return;
                    }
                },
            }
        }
    })
}

// ── Formatting helpers (model → context strings) ────────────────────────────

fn deployments_json(list: &[DeploymentSummary]) -> String {
    let rows: Vec<serde_json::Value> = list
        .iter()
        .map(|s| {
            let d = &s.deployment;
            let (label, color) = state_label_color(&d.state);
            serde_json::json!({
                "service": s.service_name,
                "project": s.project_name,
                "state_label": label,
                "state_color": color,
                "state_dot": color,
                "duration": fmt_duration(d),
                "start": d.started_at.with_timezone(&Local).format("%H:%M:%S").to_string(),
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

fn projects_json(list: &[Project]) -> String {
    let rows: Vec<serde_json::Value> = list
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "description": p.description.clone().unwrap_or_default(),
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Uppercase status label plus the accent color used by the design.
fn state_label_color(state: &DeployState) -> (&'static str, &'static str) {
    match state {
        DeployState::Live => ("LIVE", "#3FB950"),
        DeployState::Stopped => ("STOPPED", "#8B949E"),
        DeployState::Failed => ("FAILED", "#F85149"),
        DeployState::Pending
        | DeployState::ResolvingDeps
        | DeployState::PullingImage
        | DeployState::CloningRepo
        | DeployState::BuildingImage
        | DeployState::Staging
        | DeployState::HealthcheckPolling
        | DeployState::SwappingIn
        | DeployState::Draining
        | DeployState::Promoting
        | DeployState::RollingBack
        | DeployState::Pruning
        | DeployState::ComposingUp => ("BUILDING", "#58A6FF"),
    }
}

fn fmt_duration(d: &shared::Deployment) -> String {
    let end = d.finished_at.unwrap_or_else(Utc::now);
    let secs = (end - d.started_at).num_seconds().max(0) as u64;
    let m = secs / 60;
    let s = secs % 60;
    if m > 0 { format!("{m}m {s}s") } else { format!("{s}s") }
}

pub fn fmt_uptime(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m {s}s")
    }
}
