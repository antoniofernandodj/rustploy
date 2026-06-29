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
use shared::protocol::{LogEntry, LogStream};
use shared::{
    Command, ContainerMetricsPoint, DeploymentSummary, DeployState, EnvVar, EnvVarValue, Event,
    Healthcheck, HealthcheckKind, Project, Response, RwpFrame, RwpReply, Service, ServiceSource,
    ServiceStatus,
};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// How many service cards sit side by side in the Projects grid.
const GRID_COLS: usize = 3;
/// Max log lines kept per service in the live ring buffer.
const LOG_RING: usize = 200;

/// One-shot: open a fresh connection, run `cmd`, return the response.
/// Used by `ctx.perform` action effects (deploy/stop/reload).
pub async fn run_command(
    addr: String,
    token: Option<String>,
    cmd: Command,
) -> anyhow::Result<Response> {
    let mut conn = rwp::connect(&addr, token.as_deref()).await?;
    rwp::rpc(&mut conn, cmd).await
}

/// Runs a lifecycle command against the selected service, then re-fetches its
/// detail so the panel reflects the new state. Surfaces a one-line outcome in
/// `svc_action_msg`.
pub async fn run_service_action(
    addr: String,
    token: Option<String>,
    cmd: Command,
    service_id: String,
) -> Vec<(String, String)> {
    let msg = match run_command(addr.clone(), token.clone(), cmd).await {
        Ok(Response::Ok) => "ação concluída".to_string(),
        Ok(Response::Deployment(d)) => format!("deploy iniciado · {}", d.state.label()),
        Ok(other) => format!("{other:?}"),
        Err(e) => format!("erro: {e}"),
    };
    let mut pairs = fetch_service_detail(addr, token, service_id).await;
    pairs.push(("svc_action_msg".into(), msg));
    pairs
}

/// One-shot fetch of everything the Service detail screen needs: the full
/// `Service` spec/status, its project name and the most recent container logs.
/// Returns context pairs (`svc_*`) merged by a `ctx.perform` effect.
pub async fn fetch_service_detail(
    addr: String,
    token: Option<String>,
    service_id: String,
) -> Vec<(String, String)> {
    match fetch_service_detail_inner(&addr, token.as_deref(), &service_id).await {
        Ok(pairs) => pairs,
        Err(e) => vec![
            ("svc_loading".into(), "false".into()),
            ("svc_error".into(), e.to_string()),
        ],
    }
}

async fn fetch_service_detail_inner(
    addr: &str,
    token: Option<&str>,
    service_id: &str,
) -> anyhow::Result<Vec<(String, String)>> {
    let mut conn = rwp::connect(addr, token).await?;

    let svc = match rwp::rpc(&mut conn, Command::ServiceGet { id: service_id.into() }).await? {
        Response::Service(s) => s,
        other => anyhow::bail!("resposta inesperada para ServiceGet: {other:?}"),
    };

    // Resolve the project name (ServiceGet only carries the id).
    let mut project_name = svc.spec.project_id.clone();
    if let Ok(Response::Projects(list)) = rwp::rpc(&mut conn, Command::ProjectList).await
        && let Some(p) = list.iter().find(|p| p.id == svc.spec.project_id)
    {
        project_name = p.name.clone();
    }

    // Recent stdout/stderr for the LIVE OUTPUT panel (one-shot tail; live
    // streaming is a later step).
    let logs = match rwp::rpc(&mut conn, Command::LogsGet { service_id: service_id.into(), tail: 200 }).await {
        Ok(Response::Logs(l)) => l,
        _ => Vec::new(),
    };

    let (status_label, status_color) = service_status_label_color(&svc.status);
    let (source_kind, source_detail, build_engine) = source_summary(&svc.spec.source);
    let spec = &svc.spec;

    Ok(vec![
        ("svc_loading".into(), "false".into()),
        ("svc_error".into(), String::new()),
        ("svc_id".into(), svc.id.clone()),
        ("svc_name".into(), spec.name.clone()),
        ("svc_project".into(), project_name),
        ("svc_status_label".into(), status_label.into()),
        ("svc_status_color".into(), status_color.into()),
        ("svc_source_kind".into(), source_kind.into()),
        ("svc_source_detail".into(), source_detail),
        ("svc_build".into(), build_engine),
        ("svc_port".into(), spec.port.to_string()),
        ("svc_host_port".into(), spec.host_port.map(|p| p.to_string()).unwrap_or_else(|| "—".into())),
        ("svc_domain".into(), spec.domain.clone().unwrap_or_else(|| "—".into())),
        ("svc_tls".into(), if spec.tls_enabled { "enabled" } else { "disabled" }.into()),
        ("svc_replicas".into(), spec.replicas.to_string()),
        ("svc_db_kind".into(), spec.db_kind.clone().unwrap_or_else(|| "—".into())),
        ("svc_hc".into(), healthcheck_summary(&spec.healthcheck)),
        ("svc_env".into(), env_json(&spec.env_vars)),
        ("svc_env_count".into(), spec.env_vars.len().to_string()),
        ("svc_logs".into(), logs_json(&logs)),
        ("svc_logs_count".into(), logs.len().to_string()),
    ])
}

/// Long-lived polling + event stream feeding the context. Yields
/// `EngineMessage::ContextPatch` items.
///
/// `selected` mirrors the service open in the detail view; the live log stream
/// reads it to decide which `LogLine` events to surface as `svc_logs`.
pub fn poll_stream(
    addr: String,
    token: Option<String>,
    selected: Arc<Mutex<String>>,
) -> impl Stream<Item = EngineMessage> {
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

        // Latest per-service container metrics, fed by the live event stream and
        // merged into the Projects grid cards on each poll tick.
        let mut metrics: HashMap<String, ContainerMetricsPoint> = HashMap::new();

        // Live log ring buffer per service, fed by `Event::LogLine`. Only the
        // selected service's buffer is rendered (`svc_logs`).
        let mut logs: HashMap<String, VecDeque<LogEntry>> = HashMap::new();
        // Service whose buffer we last seeded; detects selection changes.
        let mut seeded: String = String::new();

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

                        // Fan out one ServiceList per project, tagging each
                        // service with its project name for the grid cards.
                        let mut all: Vec<(Service, String)> = Vec::new();
                        for p in &list {
                            if let Ok(Response::Services(svcs)) = rwp::rpc(
                                &mut cmd,
                                Command::ServiceList { project_id: p.id.clone() },
                            ).await {
                                for s in svcs {
                                    all.push((s, p.name.clone()));
                                }
                            }
                        }
                        pairs.push(("services".into(), services_json(&all, &metrics)));
                        pairs.push(("services_count".into(), all.len().to_string()));
                        pairs.push(("service_rows".into(), service_rows_json(&all, &metrics)));
                    }
                    if !pairs.is_empty() {
                        patch!(pairs);
                    }

                    // Seed the live buffer from the historical tail whenever the
                    // selected service changes, so live lines continue from it.
                    let current = selected.lock().map(|s| s.clone()).unwrap_or_default();
                    if current != seeded {
                        seeded = current.clone();
                        if !current.is_empty()
                            && let Ok(Response::Logs(tail)) = rwp::rpc(
                                &mut cmd,
                                Command::LogsGet { service_id: current.clone(), tail: LOG_RING },
                            ).await
                        {
                            let buf: VecDeque<LogEntry> = tail.into_iter().collect();
                            let snapshot = logs_json_buf(&buf);
                            let count = buf.len().to_string();
                            logs.insert(current.clone(), buf);
                            patch!(vec![
                                ("svc_logs".into(), snapshot),
                                ("svc_logs_count".into(), count),
                            ]);
                        }
                    }
                }
                frame = rwp::read_frame::<RwpReply>(&mut evt) => match frame {
                    // Cache the freshest metrics; next poll re-renders the grid.
                    Ok(RwpReply::Event(Event::ContainerMetrics(p))) => {
                        metrics.insert(p.service_id.clone(), p);
                    }
                    // Append live log lines; re-render when it's the open service.
                    Ok(RwpReply::Event(Event::LogLine { service_id, stream, line, timestamp, .. })) => {
                        let buf = logs.entry(service_id.clone()).or_default();
                        buf.push_back(LogEntry { stream, line, timestamp });
                        while buf.len() > LOG_RING {
                            buf.pop_front();
                        }
                        let current = selected.lock().map(|s| s.clone()).unwrap_or_default();
                        if service_id == current {
                            patch!(vec![
                                ("svc_logs".into(), logs_json_buf(buf)),
                                ("svc_logs_count".into(), buf.len().to_string()),
                            ]);
                        }
                    }
                    Ok(_) => { /* other live event: next poll picks up fresh data */ }
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

/// One JSON card object per service, with live CPU/memory merged in. Shared by
/// the flat `services` key and the chunked `service_rows` grid.
fn service_card(svc: &Service, project: &str, m: Option<&ContainerMetricsPoint>) -> serde_json::Value {
    let (status_label, status_color) = service_status_label_color(&svc.status);
    let (cpu, mem) = match m {
        Some(p) => (format!("{:.1}%", p.cpu_percent), fmt_bytes(p.mem_used_bytes)),
        None => ("—".to_string(), "—".to_string()),
    };
    serde_json::json!({
        "filler": "0",
        "id": svc.id,
        "name": svc.spec.name,
        "project": project,
        "port": svc.spec.port.to_string(),
        "status_label": status_label,
        "status_color": status_color,
        "cpu": cpu,
        "mem": mem,
    })
}

/// Flat array of service cards (used for selection / counts).
fn services_json(all: &[(Service, String)], metrics: &HashMap<String, ContainerMetricsPoint>) -> String {
    let rows: Vec<serde_json::Value> = all
        .iter()
        .map(|(s, proj)| service_card(s, proj, metrics.get(&s.id)))
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Service cards chunked into rows of [`GRID_COLS`], each padded with invisible
/// filler cards so every row keeps the same column widths. glacier-ui has no
/// wrapping grid, so the layout is materialised here as `[{ "cards": [...] }]`
/// and rendered with a nested `<ForEach>`.
fn service_rows_json(all: &[(Service, String)], metrics: &HashMap<String, ContainerMetricsPoint>) -> String {
    let filler = serde_json::json!({ "filler": "1" });
    let rows: Vec<serde_json::Value> = all
        .chunks(GRID_COLS)
        .map(|chunk| {
            let mut cards: Vec<serde_json::Value> = chunk
                .iter()
                .map(|(s, proj)| service_card(s, proj, metrics.get(&s.id)))
                .collect();
            while cards.len() < GRID_COLS {
                cards.push(filler.clone());
            }
            serde_json::json!({ "cards": cards })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Status pill label plus the accent color used by the design.
fn service_status_label_color(status: &ServiceStatus) -> (&'static str, &'static str) {
    match status {
        ServiceStatus::Running => ("Running", "#3FB950"),
        ServiceStatus::Deploying => ("Deploying", "#58A6FF"),
        ServiceStatus::Degraded => ("Degraded", "#D29922"),
        ServiceStatus::Stopping => ("Stopping", "#8B949E"),
        ServiceStatus::Stopped => ("Stopped", "#8B949E"),
        ServiceStatus::Error(_) => ("Error", "#F85149"),
    }
}

/// `(kind, detail, build_engine)` describing where a service is built from.
fn source_summary(source: &ServiceSource) -> (&'static str, String, String) {
    match source {
        ServiceSource::Registry { image } => ("Registry", image.clone(), "—".into()),
        ServiceSource::Git(g) => (
            "Git",
            format!("{} @ {}", g.url, g.branch),
            g.dockerfile_path.clone(),
        ),
        ServiceSource::Compose(_) => ("Compose", "docker-compose".into(), "Compose".into()),
    }
}

/// One-line summary of the healthcheck policy for the detail panel.
fn healthcheck_summary(hc: &Healthcheck) -> String {
    let kind = match &hc.kind {
        HealthcheckKind::None => return "disabled".into(),
        HealthcheckKind::Tcp => "TCP".to_string(),
        HealthcheckKind::DockerNative => "Docker".to_string(),
        HealthcheckKind::Http { path, expected_status } => format!("HTTP {path} → {expected_status}"),
    };
    format!(
        "{kind} · every {}s · timeout {}s · {} retries",
        hc.interval_secs, hc.timeout_secs, hc.retries
    )
}

/// Environment variables as JSON card rows (secrets shown by reference only).
fn env_json(vars: &[EnvVar]) -> String {
    let rows: Vec<serde_json::Value> = vars
        .iter()
        .map(|v| {
            let (value, kind) = match &v.value {
                EnvVarValue::Plain(s) => (s.clone(), "plain"),
                EnvVarValue::Secret(name) => (format!("secret:{name}"), "secret"),
            };
            serde_json::json!({ "key": v.key, "value": value, "kind": kind })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Recent log lines as JSON rows, colored by stream.
fn logs_json(logs: &[LogEntry]) -> String {
    logs_json_iter(logs.iter())
}

/// Same as [`logs_json`] over the live ring buffer.
fn logs_json_buf(buf: &VecDeque<LogEntry>) -> String {
    logs_json_iter(buf.iter())
}

fn logs_json_iter<'a>(it: impl Iterator<Item = &'a LogEntry>) -> String {
    let rows: Vec<serde_json::Value> = it
        .map(|e| {
            let color = match e.stream {
                LogStream::Stderr => "#F85149",
                LogStream::Stdout => "#9DA7B3",
            };
            serde_json::json!({
                "time": e.timestamp.with_timezone(&Local).format("%H:%M:%S").to_string(),
                "line": e.line,
                "color": color,
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Human-readable byte size for the memory stat on a card.
fn fmt_bytes(b: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let f = b as f64;
    if b == 0 {
        "—".to_string()
    } else if f >= GB {
        format!("{:.1} GB", f / GB)
    } else if f >= MB {
        format!("{:.0} MB", f / MB)
    } else {
        format!("{:.0} KB", f / KB)
    }
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
