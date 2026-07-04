//! Pure model → display-string/JSON formatters. Nothing here touches
//! `RwpClient` or does I/O — every function is a deterministic transform from
//! a `shared` model (or a raw field) to the string/JSON a template renders.
//! Shared by every op module (`services`, `projects`, `docker`, `git`) and by
//! `poll_stream` in `mod.rs`.

use chrono::{Local, Utc};
use shared::protocol::{BuildLogLine, LogEntry, LogStream};
use shared::{
    ContainerMetricsPoint, DeployState, DeploymentSummary, EnvComment, EnvVar, EnvVarValue,
    GitBranch, GitProvider, GitRepo, Healthcheck, HealthcheckKind, Project, Response, Service,
    ServiceSource, ServiceStatus,
};
use std::collections::{HashMap, VecDeque};

/// How many service cards sit side by side in the Projects grid.
const GRID_COLS: usize = 3;

/// Renders an unexpected/`Err` response into a one-line, human-readable message.
pub(crate) fn resp_msg(r: &Response) -> String {
    match r {
        Response::Err { code, message } => format!("erro: {code}: {message}"),
        other => format!("resposta inesperada: {other:?}"),
    }
}

/// Whether any of `fields` contains `term` (case-insensitive substring). An
/// empty `term` always matches — the topbar search is a filter, not a
/// requirement, so a blank box shows everything.
pub(crate) fn matches_search(term: &str, fields: &[&str]) -> bool {
    term.is_empty() || fields.iter().any(|f| f.to_lowercase().contains(term))
}

/// Barra de progresso em blocos (█ cheio / ░ vazio) para um percent 0–100.
pub(crate) fn progress_bar(percent: u8, width: usize) -> String {
    let filled = (percent as usize * width / 100).min(width);
    format!("{}{}", "█".repeat(filled), "░".repeat(width - filled))
}

/// Linhas da seção "Executando agora" da aba Deploy Engine.
pub(crate) fn eng_active_json(active: &[shared::ActiveDeployInfo]) -> String {
    let rows: Vec<serde_json::Value> = active
        .iter()
        .map(|info| {
            let (label, _) = state_label_color(&info.state);
            serde_json::json!({
                "service": info.service_name,
                "project": info.project_name,
                "state_label": label,
                "state_kind": state_kind(&info.state),
                "percent": format!("{}%", info.percent),
                "bar": progress_bar(info.percent, 12),
                "total": fmt_secs(info.elapsed_secs),
                "phase": fmt_secs(info.current_state_secs),
                "service_id": info.service_id,
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Linhas da seção "Histórico 24h" da aba Deploy Engine.
pub(crate) fn eng_recent_json(recent: &[shared::ActiveDeployInfo]) -> String {
    let rows: Vec<serde_json::Value> = recent
        .iter()
        .map(|info| {
            let (label, _) = state_label_color(&info.state);
            let icon = match info.state {
                DeployState::Live => "✓",
                DeployState::Failed => "✕",
                _ => "○",
            };
            serde_json::json!({
                "icon": icon,
                "service": info.service_name,
                "project": info.project_name,
                "state_label": label,
                "state_kind": state_kind(&info.state),
                "duration": fmt_secs(info.elapsed_secs),
                "start": info.started_at.with_timezone(&Local).format("%H:%M:%S").to_string(),
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

pub(crate) fn deployments_json(list: &[DeploymentSummary], search: &str) -> String {
    let rows: Vec<serde_json::Value> = list
        .iter()
        .filter(|s| matches_search(search, &[&s.service_name, &s.project_name]))
        .map(|s| {
            let d = &s.deployment;
            let (label, _) = state_label_color(&d.state);
            serde_json::json!({
                "service": s.service_name,
                "project": s.project_name,
                "state_label": label,
                "state_kind": state_kind(&d.state),
                "duration": fmt_duration(d),
                "start": d.started_at.with_timezone(&Local).format("%H:%M:%S").to_string(),
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// One JSON card object per service, with live CPU/memory merged in. Shared by
/// the flat `services` key and the chunked `service_rows` grid.
pub(crate) fn service_card(svc: &Service, project: &str, m: Option<&ContainerMetricsPoint>) -> serde_json::Value {
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
pub(crate) fn services_json(all: &[(Service, String)], metrics: &HashMap<String, ContainerMetricsPoint>, search: &str) -> String {
    let rows: Vec<serde_json::Value> = all
        .iter()
        .filter(|(s, proj)| matches_search(search, &[&s.spec.name, proj]))
        .map(|(s, proj)| service_card(s, proj, metrics.get(&s.id)))
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Service cards chunked into rows of [`GRID_COLS`], each padded with invisible
/// filler cards so every row keeps the same column widths. glacier-ui has no
/// wrapping grid, so the layout is materialised here as `[{ "cards": [...] }]`
/// and rendered with a nested `<ForEach>`.
pub(crate) fn service_rows_json(all: &[(Service, String)], metrics: &HashMap<String, ContainerMetricsPoint>, search: &str) -> String {
    let filler = serde_json::json!({ "filler": "1" });
    let rows: Vec<serde_json::Value> = all
        .iter()
        .filter(|(s, proj)| matches_search(search, &[&s.spec.name, proj]))
        .collect::<Vec<_>>()
        .chunks(GRID_COLS)
        .map(|chunk| {
            let mut cards: Vec<serde_json::Value> = chunk
                .iter()
                .copied()
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
pub(crate) fn service_status_label_color(status: &ServiceStatus) -> (&'static str, &'static str) {
    match status {
        ServiceStatus::Running => ("Running", "#3FB950"),
        ServiceStatus::Deploying => ("Deploying", "#58A6FF"),
        ServiceStatus::Degraded => ("Degraded", "#D29922"),
        ServiceStatus::Stopping => ("Stopping", "#8B949E"),
        ServiceStatus::Stopped => ("Stopped", "#8B949E"),
        ServiceStatus::Error(_) => ("Error", "#F85149"),
    }
}

/// Token semântico de estado para o `StateCell` — a paleta (cor) vive no GSS
/// (`.state_<kind>`); aqui só classificamos o estado. Badges continuam usando
/// o hex de `service_status_label_color`/`state_label_color`.
pub(crate) fn service_status_kind(status: &ServiceStatus) -> &'static str {
    match status {
        ServiceStatus::Running => "ok",
        ServiceStatus::Deploying => "info",
        ServiceStatus::Degraded => "warn",
        ServiceStatus::Stopping | ServiceStatus::Stopped => "muted",
        ServiceStatus::Error(_) => "bad",
    }
}

/// `true` em uso → verde (`ok`); senão cinza (`muted`). Usado nas listas Docker.
pub(crate) fn in_use_kind(in_use: bool) -> &'static str {
    if in_use { "ok" } else { "muted" }
}

/// `(kind, detail, build_engine)` describing where a service is built from.
pub(crate) fn source_summary(source: &ServiceSource) -> (&'static str, String, String) {
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
pub(crate) fn healthcheck_summary(hc: &Healthcheck) -> String {
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

/// Ingress route rows derived from services that expose domains — one row per
/// domain route (a service may expose several). Returns `(json, count)`.
pub(crate) fn ingress_json(all: &[(Service, String)]) -> (String, usize) {
    let rows: Vec<serde_json::Value> = all
        .iter()
        .flat_map(|(s, proj)| {
            let port = s.spec.port;
            let name = s.spec.name.clone();
            let proj = proj.clone();
            s.spec
                .domain_routes()
                .into_iter()
                .filter(|r| !r.domain.trim().is_empty())
                .map(move |r| {
                    let scheme = if r.tls { "https" } else { "http" };
                    serde_json::json!({
                        "domain": r.domain,
                        "url": format!("{scheme}://{}", r.domain),
                        "service": name,
                        "project": proj,
                        "upstream": format!(":{}", r.container_port(port)),
                        "tls": if r.tls { "TLS" } else { "—" },
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let count = rows.len();
    (serde_json::Value::Array(rows).to_string(), count)
}

/// Docker container rows derived from services (one per managed service).
/// Sorted by name — the underlying `all` list is rebuilt from scratch every
/// poll tick, and without a stable order the table reshuffles on each
/// refresh.
pub(crate) fn docker_json(all: &[(Service, String)], search: &str) -> String {
    let mut all: Vec<_> = all.iter().collect();
    all.sort_by(|a, b| a.0.spec.name.cmp(&b.0.spec.name));
    let rows: Vec<serde_json::Value> = all
        .iter()
        .filter(|(s, proj)| matches_search(search, &[&s.spec.name, proj]))
        .map(|(s, proj)| {
            let (status_label, _) = service_status_label_color(&s.status);
            let container = s
                .live_container_id
                .as_deref()
                .map(|c| c.chars().take(12).collect::<String>())
                .unwrap_or_else(|| "—".into());
            serde_json::json!({
                "name": s.spec.name,
                "project": proj,
                "image": source_summary(&s.spec.source).1,
                "container": container,
                "status_label": status_label,
                "status_kind": service_status_kind(&s.status),
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Docker image rows for the Images sub-tab — every image on the host, not
/// just ones rustploy built/pulled. `in_use` reflects whether any container
/// (running or stopped) currently references it (see
/// `DockerImageInfo::containers`, computed daemon-side via `docker system df`).
pub(crate) fn docker_images_json(list: &[shared::DockerImageInfo], search: &str) -> String {
    let mut list: Vec<_> = list.iter().collect();
    list.sort_by(|a, b| a.tags.join(",").cmp(&b.tags.join(",")).then(a.id.cmp(&b.id)));
    let rows: Vec<serde_json::Value> = list
        .iter()
        .filter(|img| {
            let tags = img.tags.join(" ");
            matches_search(
                search,
                &[&tags, img.project.as_deref().unwrap_or(""), img.service.as_deref().unwrap_or("")],
            )
        })
        .map(|img| {
            let in_use = img.containers > 0;
            let tags = if img.tags.is_empty() { "<none>".to_string() } else { img.tags.join(", ") };
            serde_json::json!({
                "id": img.id.trim_start_matches("sha256:").chars().take(12).collect::<String>(),
                "tags": tags,
                "size": fmt_bytes(img.size_bytes),
                "created": img.created.with_timezone(&Local).format("%d/%m %H:%M").to_string(),
                "project": img.project.clone().unwrap_or_else(|| "—".into()),
                "service": img.service.clone().unwrap_or_else(|| "—".into()),
                "in_use_label": if in_use { "EM USO" } else { "SEM USO" },
                "in_use_kind": in_use_kind(in_use),
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Docker volume rows for the Volumes sub-tab.
pub(crate) fn docker_volumes_json(list: &[shared::DockerVolumeInfo], search: &str) -> String {
    let mut list: Vec<_> = list.iter().collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    let rows: Vec<serde_json::Value> = list
        .iter()
        .filter(|v| matches_search(search, &[&v.name, &v.driver]))
        .map(|v| {
            serde_json::json!({
                "name": v.name,
                "driver": v.driver,
                "mountpoint": v.mountpoint,
                "size": if v.size_bytes >= 0 { fmt_bytes(v.size_bytes as u64) } else { "—".to_string() },
                "in_use_label": if v.in_use { "EM USO" } else { "SEM USO" },
                "in_use_kind": in_use_kind(v.in_use),
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Docker network rows for the Networks sub-tab.
pub(crate) fn docker_networks_json(list: &[shared::DockerNetworkInfo], search: &str) -> String {
    let mut list: Vec<_> = list.iter().collect();
    list.sort_by(|a, b| a.name.cmp(&b.name));
    let rows: Vec<serde_json::Value> = list
        .iter()
        .filter(|n| matches_search(search, &[&n.name, n.project.as_deref().unwrap_or("")]))
        .map(|n| {
            serde_json::json!({
                "name": n.name,
                "driver": n.driver,
                "scope": n.scope,
                "project": n.project.clone().unwrap_or_else(|| "—".into()),
                "containers": n.container_count.to_string(),
                "in_use_label": if n.in_use { "EM USO" } else { "SEM USO" },
                "in_use_kind": in_use_kind(n.in_use),
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Per-container live metrics rows for the Monitoring screen.
pub(crate) fn monitoring_json(all: &[(Service, String)], metrics: &HashMap<String, ContainerMetricsPoint>) -> String {
    let rows: Vec<serde_json::Value> = all
        .iter()
        .filter_map(|(s, proj)| {
            let m = metrics.get(&s.id)?;
            Some(serde_json::json!({
                "name": s.spec.name,
                "project": proj,
                "cpu": format!("{:.1}%", m.cpu_percent),
                "mem": fmt_bytes(m.mem_used_bytes),
                "rx": fmt_bytes(m.net_rx_bytes),
                "tx": fmt_bytes(m.net_tx_bytes),
            }))
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Per-service deployment history rows for the Deployments tab.
pub(crate) fn deployments_detail_json(list: &[shared::Deployment]) -> String {
    let rows: Vec<serde_json::Value> = list
        .iter()
        .map(|d| {
            let (label, _) = state_label_color(&d.state);
            serde_json::json!({
                "id": d.id.chars().take(12).collect::<String>(),
                "id_full": d.id,
                "image": d.image,
                "state_label": label,
                "state_kind": state_kind(&d.state),
                "duration": fmt_duration(d),
                "start": d.started_at.with_timezone(&Local).format("%d/%m %H:%M:%S").to_string(),
                // The daemon refuses to delete a non-terminal deployment
                // (`DeployDelete` → "DEPLOY_ACTIVE") — hide the button instead
                // of round-tripping into a doomed request.
                "can_delete": if d.state.is_terminal() { "1" } else { "0" },
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Env vars rendered as a `.env` text blob (KEY=VALUE, secrets by reference).
/// Renders vars back to a `.env` blob, re-interleaving `comments` at their
/// anchored position: right before the var named by `before_key`, or at the
/// end for `before_key: None` — the inverse of `parse_dotenv_with_comments`.
pub(crate) fn env_dotenv_with_comments(vars: &[EnvVar], comments: &[EnvComment]) -> String {
    let mut lines = Vec::new();
    for v in vars {
        for c in comments.iter().filter(|c| c.before_key.as_deref() == Some(v.key.as_str())) {
            lines.push(c.text.clone());
        }
        let val = match &v.value {
            EnvVarValue::Plain(s) => s.clone(),
            EnvVarValue::Secret(name) => format!("<secret:{name}>"),
        };
        lines.push(format!("{}={}", v.key, val));
    }
    for c in comments.iter().filter(|c| c.before_key.is_none()) {
        lines.push(c.text.clone());
    }
    lines.join("\n")
}

/// Environment variables as JSON card rows (secrets shown by reference only).
pub(crate) fn env_json(vars: &[EnvVar]) -> String {
    let rows: Vec<serde_json::Value> = vars.iter().map(env_var_row).collect();
    serde_json::Value::Array(rows).to_string()
}

fn env_var_row(v: &EnvVar) -> serde_json::Value {
    let (value, kind) = match &v.value {
        EnvVarValue::Plain(s) => (s.clone(), "plain"),
        EnvVarValue::Secret(name) => (format!("secret:{name}"), "secret"),
    };
    serde_json::json!({ "key": v.key, "value": value, "kind": kind })
}

/// Same as [`env_json`], re-interleaving the `.env` editor's `# ...` comments
/// at their anchored position (right before the var named by `before_key`;
/// trailing/orphan ones at the end) — same ordering as
/// [`env_dotenv_with_comments`]. Comment rows carry `kind: "comment"` and a
/// synthetic `__c<idx>` key, where `idx` is the comment's index in
/// `spec.env_comments` — that's how `EnvOp::Reorder` maps a dragged comment
/// row back to the comment it re-anchors.
pub(crate) fn env_json_with_comments(vars: &[EnvVar], comments: &[EnvComment]) -> String {
    let mut rows: Vec<serde_json::Value> = Vec::new();
    let comment_row = |idx: usize, text: &str| {
        serde_json::json!({ "key": format!("__c{idx}"), "value": text, "kind": "comment" })
    };
    for v in vars {
        for (i, c) in comments
            .iter()
            .enumerate()
            .filter(|(_, c)| c.before_key.as_deref() == Some(v.key.as_str()))
        {
            rows.push(comment_row(i, &c.text));
        }
        rows.push(env_var_row(v));
    }
    for (i, c) in comments.iter().enumerate().filter(|(_, c)| c.before_key.is_none()) {
        rows.push(comment_row(i, &c.text));
    }
    serde_json::Value::Array(rows).to_string()
}

/// Recent log lines as JSON rows, colored by stream.
pub(crate) fn logs_json(logs: &[LogEntry]) -> String {
    logs_json_iter(logs.iter())
}

/// Same as [`logs_json`] over the live ring buffer.
pub(crate) fn logs_json_buf(buf: &VecDeque<LogEntry>) -> String {
    logs_json_iter(buf.iter())
}

/// Build log lines as JSON rows, colored by stream.
pub(crate) fn build_logs_json(lines: &[BuildLogLine]) -> String {
    build_logs_json_iter(lines.iter())
}

/// Same as [`build_logs_json`] over the live ring buffer.
pub(crate) fn build_logs_json_buf(buf: &VecDeque<BuildLogLine>) -> String {
    build_logs_json_iter(buf.iter())
}

fn build_logs_json_iter<'a>(it: impl Iterator<Item = &'a BuildLogLine>) -> String {
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

/// Joins log lines into a plain-text blob (`HH:MM:SS line`) for the selectable
/// `<TextArea>` view and the "copy all" clipboard action.
pub(crate) fn join_log_lines<'a>(
    it: impl Iterator<Item = (&'a chrono::DateTime<Utc>, &'a str)>,
) -> String {
    it.map(|(ts, line)| format!("{} {}", ts.with_timezone(&Local).format("%H:%M:%S"), line))
        .collect::<Vec<_>>()
        .join("\n")
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
pub(crate) fn fmt_bytes(b: u64) -> String {
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

/// One JSON card object per project, with service counts aggregated from the
/// already-fetched `all` (every service across every project). `can_delete`
/// mirrors the daemon's own rule (`project_delete` refuses non-empty
/// projects) so the Projects grid can hide the button instead of round-
/// tripping into a doomed request.
fn project_card(p: &Project, all: &[(Service, String)]) -> serde_json::Value {
    let (mut count, mut running) = (0usize, 0usize);
    for (s, _) in all.iter().filter(|(s, _)| s.spec.project_id == p.id) {
        count += 1;
        if matches!(s.status, ServiceStatus::Running) {
            running += 1;
        }
    }
    serde_json::json!({
        "filler": "0",
        "id": p.id,
        "name": p.name,
        "description": p.description.clone().unwrap_or_default(),
        "service_count": count.to_string(),
        "running_count": running.to_string(),
        "can_delete": if count == 0 { "1" } else { "0" },
    })
}

/// Flat array of project cards (used for the stat counts).
pub(crate) fn projects_json(list: &[Project], all: &[(Service, String)], search: &str) -> String {
    let rows: Vec<serde_json::Value> = list
        .iter()
        .filter(|p| matches_search(search, &[&p.name, p.description.as_deref().unwrap_or("")]))
        .map(|p| project_card(p, all))
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Project cards chunked into rows of [`GRID_COLS`] for the Projects grid,
/// mirroring [`service_rows_json`] (glacier-ui has no wrapping grid).
pub(crate) fn project_rows_json(list: &[Project], all: &[(Service, String)], search: &str) -> String {
    let filler = serde_json::json!({ "filler": "1" });
    let rows: Vec<serde_json::Value> = list
        .iter()
        .filter(|p| matches_search(search, &[&p.name, p.description.as_deref().unwrap_or("")]))
        .collect::<Vec<_>>()
        .chunks(GRID_COLS)
        .map(|chunk| {
            let mut cards: Vec<serde_json::Value> =
                chunk.iter().copied().map(|p| project_card(p, all)).collect();
            while cards.len() < GRID_COLS {
                cards.push(filler.clone());
            }
            serde_json::json!({ "cards": cards })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Uppercase status label plus the accent color used by the design.
pub(crate) fn state_label_color(state: &DeployState) -> (&'static str, &'static str) {
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

/// Token semântico de estado do deploy para o `StateCell` (paleta no GSS,
/// classe `.state_<kind>`). Espelha as cores de `state_label_color`.
pub(crate) fn state_kind(state: &DeployState) -> &'static str {
    match state {
        DeployState::Live => "ok",
        DeployState::Stopped => "muted",
        DeployState::Failed => "bad",
        _ => "info",
    }
}

pub(crate) fn fmt_duration(d: &shared::Deployment) -> String {
    let end = d.finished_at.unwrap_or_else(Utc::now);
    let secs = (end - d.started_at).num_seconds().max(0) as u64;
    fmt_secs(secs)
}

/// Formats a second count as `Ns` or `Mm Ns`, used for both the deployments
/// tables (recomputed on each poll) and the live deploy timer (recomputed
/// every second by `poll_stream`'s `sec_tick`).
pub(crate) fn fmt_secs(secs: u64) -> String {
    let m = secs / 60;
    let s = secs % 60;
    if m > 0 { format!("{m}m {s}s") } else { format!("{s}s") }
}

pub(crate) fn fmt_uptime(secs: u64) -> String {
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

/// Full internal connection URL for a service reachable at `rp_<safe>:<port>`
/// on the project network. Most kinds get a scheme prefix so the value is a
/// ready-to-paste connection string; Kafka is the exception — its clients take
/// a plain `host:port` bootstrap address, not a URL.
pub(crate) fn internal_url(db_kind: Option<&str>, safe_name: &str, port: u16) -> String {
    let host = format!("rp_{safe_name}:{port}");
    match internal_scheme(db_kind) {
        Some(scheme) => format!("{scheme}://{host}"),
        None => host,
    }
}

/// Scheme for a service's internal connection URL, keyed off `db_kind`.
/// Returns `None` for kinds that address by bare `host:port` (Kafka).
/// Database/broker services get their protocol scheme so the URL is directly
/// usable as a connection string; everything else is assumed to speak HTTP.
fn internal_scheme(db_kind: Option<&str>) -> Option<&'static str> {
    match db_kind.map(str::to_ascii_lowercase).as_deref() {
        Some("postgres") | Some("postgresql") => Some("postgres"),
        Some("mysql") | Some("mariadb") => Some("mysql"),
        Some("redis") => Some("redis"),
        Some("mongodb") | Some("mongo") => Some("mongodb"),
        Some("rabbitmq") => Some("amqp"),
        Some("nats") => Some("nats"),
        // Kafka: bootstrap servers are bare `host:port`, no URL scheme.
        Some("kafka") => None,
        _ => Some("http"),
    }
}

/// JSON list rendered by the Domains tab: one row per HTTP domain route with
/// its container port and TLS flag (legacy single-domain specs fold into one).
pub(crate) fn domains_json(spec: &shared::ServiceSpec) -> String {
    let rows: Vec<serde_json::Value> = spec
        .domain_routes()
        .iter()
        .map(|r| {
            let tls = r.tls;
            serde_json::json!({
                "domain": r.domain,
                "port": r.container_port(spec.port).to_string(),
                "tls": if tls { "true" } else { "false" },
                "tls_label": if tls { "TLS" } else { "—" },
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Public ingress URL for a service: `{https|http}://{domain}`, or `—` when no
/// domain is configured (the service isn't reachable from outside the box).
pub(crate) fn external_url(domain: Option<&str>, tls: bool) -> String {
    match domain.map(str::trim).filter(|d| !d.is_empty()) {
        Some(d) => format!("{}://{}", if tls { "https" } else { "http" }, d.trim_end_matches('/')),
        None => "—".to_string(),
    }
}

pub(crate) fn git_providers_json(list: &[GitProvider]) -> String {
    let rows: Vec<serde_json::Value> = list
        .iter()
        .map(|p| {
            let (account, connected) = match &p.account {
                Some(a) => (a.login.clone(), "true"),
                None => ("não conectado".to_string(), "false"),
            };
            // Label shown in the account <Select>: "name (@login)" when connected.
            let display = match &p.account {
                Some(a) => format!("{} (@{})", p.name, a.login),
                None => format!("{} — não conectado", p.name),
            };
            let auth_mode = match p.auth_mode {
                shared::GitAuthMode::OAuth => "OAuth2",
                shared::GitAuthMode::Pat => "PAT",
            };
            let account_lbl = match &p.account {
                Some(a) => format!("@{}", a.login),
                None => "(pendente — autorize no navegador)".to_string(),
            };
            serde_json::json!({
                "id": p.id,
                "name": p.name,
                "display": display,
                "base_url": p.base_url,
                "auth_mode": auth_mode,
                "account": account_lbl,
                "account_login": account,
                "connected": connected,
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

pub(crate) fn git_repos_json(list: &[GitRepo]) -> String {
    let rows: Vec<serde_json::Value> = list
        .iter()
        .map(|r| {
            serde_json::json!({
                "full_name": r.full_name,
                "clone_url": r.clone_url,
                "default_branch": r.default_branch,
                "private": if r.private { "private" } else { "public" },
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

pub(crate) fn git_branches_json(list: &[GitBranch]) -> String {
    let rows: Vec<serde_json::Value> = list
        .iter()
        .map(|b| serde_json::json!({ "name": b.name }))
        .collect();
    serde_json::Value::Array(rows).to_string()
}
