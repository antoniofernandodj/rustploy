//! Network bridge between the daemon (RWP) and the glacier-ui context.
//!
//! Every RPC-driven action is a method on a small per-domain struct
//! (`Services`, `Projects`, `Docker`, `GitProviders`, `Daemon`), each just a
//! [`RwpClient`] plus the operations that make sense on it — built fresh at
//! each call site (`Services::new(self.client.clone())`, cheap since
//! `RwpClient` is an `Arc` clone) instead of threading `addr`/`token` through
//! free functions. `view` holds every pure model → display-string/JSON
//! formatter (no RwpClient, no I/O). [`poll_stream`] is the odd one out: a
//! long-lived `Subscription` source that reads from every domain in one loop,
//! so it doesn't belong to any single domain struct.

mod daemon;
mod docker;
mod git;
mod projects;
mod services;
mod view;

pub use daemon::Daemon;
pub use docker::Docker;
pub use git::{oauth_redirect_uri, GitProviders};
pub use projects::{ProjectEnvOp, Projects};
pub use services::{EnvOp, Services, SpecOp};
pub use super::rwp::RwpClient;

use chrono::{DateTime, Utc};
use glacier_ui::{EffectOutcome, EngineMessage, ToastSpec};
use iced::futures::{SinkExt, Stream};
use shared::protocol::{BuildLogLine, LogEntry, LogStream};
use shared::{
    Command, ContainerMetricsPoint, DeployState, DeploymentSummary, Project, Response, RwpFrame,
    RwpReply, Service, SystemMetricsPoint,
};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// Max log lines kept per service in the live ring buffer.
const LOG_RING: usize = 200;
/// Max build-log lines kept per deployment (builds can be verbose).
const BUILD_RING: usize = 2000;

// ── Toasts ──────────────────────────────────────────────────────────────────
//
// A mutating action's outcome (delete/deploy/edit/…) is worth surfacing as a
// toast even if the user has since navigated away from the panel that shows
// its inline `*_msg` text. Since glacier-ui 0.7 a `ctx.perform` effect returns
// an [`EffectOutcome`] (data patch + optional toast), so the toast travels the
// engine's own channel — `EngineMessage::EffectOutcome`, which
// `GlacierUI::dispatch` both merges the patch and shows the toast for. No more
// reserved context keys, no interception in `app/mod.rs`.
//
// `poll_stream` (a long-lived subscription, not an `Effect`) emits the same
// `EngineMessage::EffectOutcome` directly for its own outcome toasts (connect
// failure, deploy completion) — see the `outcome!` macro there.

/// An [`EffectOutcome`] carrying `pairs` plus a success/error toast inferred
/// from one of this module's own outcome messages: `"erro…"` /
/// `"resposta inesperada…"` (see [`view::resp_msg`]) become an error toast,
/// anything else a success one.
fn outcome_toast(pairs: Vec<(String, String)>, msg: &str) -> EffectOutcome {
    EffectOutcome::data(pairs).with_toast(infer_toast(msg))
}

/// A success/error [`ToastSpec`] inferred from an outcome message (see
/// [`outcome_toast`]).
fn infer_toast(msg: &str) -> ToastSpec {
    if msg.starts_with("erro") || msg.starts_with("resposta inesperada") {
        ToastSpec::error(msg)
    } else {
        ToastSpec::success(msg)
    }
}

/// Identity of the deploy currently being watched, shared between the action
/// that starts it (`Services::start_deploy`) and the poll loop
/// ([`poll_stream`]) that ticks its elapsed time and detects completion.
/// `started_at` cached here means the 1Hz tick costs no RPC; `service_id` lets
/// the poll loop only patch `svc_deploy_*` while the *same* service's detail
/// panel is open.
#[derive(Clone, Default)]
pub struct DeployTrack {
    pub(crate) service_id: String,
    pub(crate) deployment_id: String,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) running: bool,
}

/// RAM snapshot of the last successful service/project detail fetch, so
/// reopening a detail view paints instantly (cache-aside) instead of showing
/// a spinner until the fetch lands. Written through by `Services::fetch_detail_cached`
/// / `Projects::fetch_services_cached`, read by `Root`'s `open_service` /
/// `open_project`. Shared with those async fetches via an `Arc<Mutex>` — same
/// pattern as [`SearchCache`], since a `ctx.perform` result never routes back
/// through `Component::update` for the component to stash it itself.
#[derive(Default)]
pub struct DetailCache {
    services: HashMap<String, Vec<(String, String)>>,
    projects: HashMap<String, Vec<(String, String)>>,
}

impl DetailCache {
    /// Cloned cached pairs for a service's detail view, if any.
    pub fn service(&self, id: &str) -> Option<Vec<(String, String)>> {
        self.services.get(id).cloned()
    }
    /// Cloned cached pairs for a project's service list, if any.
    pub fn project(&self, id: &str) -> Option<Vec<(String, String)>> {
        self.projects.get(id).cloned()
    }
    pub(crate) fn insert_service(&mut self, id: String, pairs: Vec<(String, String)>) {
        self.services.insert(id, pairs);
    }
    pub(crate) fn insert_project(&mut self, id: String, pairs: Vec<(String, String)>) {
        self.projects.insert(id, pairs);
    }
}

pub type DetailCacheHandle = Arc<Mutex<DetailCache>>;

/// Snapshot of the last-polled raw data that the topbar search filters over.
/// Shared (behind an `Arc<Mutex>`) between the poll loop — which refreshes it
/// every 2s tick — and `Root`'s `search_changed` handler, which rebuilds the
/// filtered lists from it on every keystroke so typing filters instantly
/// instead of waiting for the next poll (see [`search_pairs`]).
#[derive(Default)]
pub struct SearchCache {
    pub projects: Vec<Project>,
    pub services: Vec<(Service, String)>,
    pub metrics: HashMap<String, ContainerMetricsPoint>,
    pub deployments: Vec<DeploymentSummary>,
    pub docker_images: Vec<shared::DockerImageInfo>,
    pub docker_volumes: Vec<shared::DockerVolumeInfo>,
    pub docker_networks: Vec<shared::DockerNetworkInfo>,
}

impl SearchCache {
    /// True before the first poll has populated anything — the keystroke path
    /// skips rebuilding then, so it doesn't blank the lists before data lands.
    pub fn is_empty(&self) -> bool {
        self.projects.is_empty()
            && self.services.is_empty()
            && self.deployments.is_empty()
            && self.docker_images.is_empty()
            && self.docker_volumes.is_empty()
            && self.docker_networks.is_empty()
    }
}

/// Rebuilds every search-filtered context key from `cache` for `term` (the
/// only keys whose contents depend on the search box). Shared by the poll loop
/// and the instant `search_changed` handler so both filter identically; counts
/// (`*_count`) are intentionally left out — they track totals and are set by
/// the poll loop regardless of the term. `selected_project`, when set, also
/// re-filters the open `project_services` grid.
pub fn search_pairs(cache: &SearchCache, term: &str, selected_project: &str) -> Vec<(String, String)> {
    let term = term.trim().to_lowercase();
    let mut pairs = vec![
        ("deployments".into(), view::deployments_json(&cache.deployments, &term)),
        ("projects".into(), view::projects_json(&cache.projects, &cache.services, &term)),
        ("project_rows".into(), view::project_rows_json(&cache.projects, &cache.services, &term)),
        ("services".into(), view::services_json(&cache.services, &cache.metrics, &term)),
        ("service_rows".into(), view::service_rows_json(&cache.services, &cache.metrics, &term)),
        ("docker_rows".into(), view::docker_json(&cache.services, &term)),
        ("docker_images".into(), view::docker_images_json(&cache.docker_images, &term)),
        ("docker_volumes".into(), view::docker_volumes_json(&cache.docker_volumes, &term)),
        ("docker_networks".into(), view::docker_networks_json(&cache.docker_networks, &term)),
    ];
    if !selected_project.is_empty()
        && cache.projects.iter().any(|p| p.id == selected_project)
    {
        let proj_all: Vec<(Service, String)> = cache
            .services
            .iter()
            .filter(|(s, _)| s.spec.project_id == selected_project)
            .cloned()
            .collect();
        pairs.push(("project_services".into(), view::service_rows_json(&proj_all, &cache.metrics, &term)));
    }
    pairs
}

/// Long-lived polling + event stream feeding the context. Yields
/// `EngineMessage::ContextPatch` items.
///
/// `selected` mirrors the service open in the detail view; the live log stream
/// reads it to decide which `LogLine` events to surface as `svc_logs`.
/// `selected_deploy` does the same for `BuildLog` events → `dep_build_logs`.
pub fn poll_stream(
    client: RwpClient,
    selected: Arc<Mutex<String>>,
    selected_deploy: Arc<Mutex<String>>,
    deploy_track: Arc<Mutex<DeployTrack>>,
    search: Arc<Mutex<String>>,
    selected_project: Arc<Mutex<String>>,
    search_cache: Arc<Mutex<SearchCache>>,
) -> impl Stream<Item = EngineMessage> {
    iced::stream::channel(64, move |mut output: iced::futures::channel::mpsc::Sender<EngineMessage>| async move {
        macro_rules! patch {
            ($pairs:expr) => {
                let _ = output.send(EngineMessage::ContextPatch($pairs)).await;
            };
        }
        // Data patch + a toast, both applied by `GlacierUI::dispatch`'s
        // `EffectOutcome` arm — the same channel a `ctx.perform` effect uses.
        macro_rules! outcome {
            ($pairs:expr, $toast:expr) => {
                let _ = output
                    .send(EngineMessage::EffectOutcome(
                        EffectOutcome::data($pairs).with_toast($toast),
                    ))
                    .await;
            };
        }

        // The shared command connection (reused by every other `net::*`
        // action, and by the RPC polling below) + a dedicated event
        // connection (long-lived read loop, can't share a request/response
        // client).
        if let Err(e) = client.ensure_connected().await {
            outcome!(
                vec![
                    ("connected".into(), "false".into()),
                    ("screen".into(), "login".into()),
                    ("error".into(), e.to_string()),
                    ("status_line".into(), "falha na conexão".into()),
                ],
                ToastSpec::error(format!("Falha na conexão: {e}"))
            );
            return;
        }
        let mut evt = match super::rwp::connect(client.addr(), client.token()).await {
            Ok(s) => s,
            Err(_) => return,
        };
        let _ = super::rwp::write_frame(&mut evt, &RwpFrame::Subscribe { service_id: None }).await;

        patch!(vec![
            ("connected".into(), "true".into()),
            ("screen".into(), "shell".into()),
            ("error".into(), String::new()),
            ("status_line".into(), "conectado".into()),
        ]);

        // Daemon settings fetched once on connect so the editable fields are not
        // clobbered by polling while the user types.
        if let Ok(Response::DaemonSettings { webhook_base_url, acme_email }) =
            client.rpc(Command::GetDaemonSettings).await
        {
            let domain = webhook_base_url.unwrap_or_default();
            patch!(vec![
                ("ss_domain".into(), domain.clone()),
                ("ss_email".into(), acme_email.unwrap_or_default()),
                ("gp_redirect".into(), oauth_redirect_uri(&domain)),
            ]);
        }

        // Connected Git providers, fetched once on connect for the Settings → Git
        // list and the service General → Gitea account picker.
        if let Ok(Response::GitProviders(list)) =
            client.rpc(Command::GitProviderList).await
        {
            patch!(vec![
                ("gitea_providers".into(), view::git_providers_json(&list)),
                ("gitea_count".into(), list.len().to_string()),
            ]);
        }

        let mut poll = tokio::time::interval(std::time::Duration::from_secs(2));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        // Drives the live "1s, 2s, 3s…" deploy timer: cheap (no RPC), so it can
        // run at 1Hz independently of the heavier 2s status poll above.
        let mut sec_tick = tokio::time::interval(std::time::Duration::from_secs(1));
        sec_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        // Latest per-service container metrics, fed by the live event stream and
        // merged into the Projects grid cards on each poll tick.
        let mut metrics: HashMap<String, ContainerMetricsPoint> = HashMap::new();
        // Latest host metrics, fed by `Event::SystemMetrics`.
        let mut sysm: Option<SystemMetricsPoint> = None;

        // Live log ring buffer per service, fed by `Event::LogLine`. Only the
        // selected service's buffer is rendered (`svc_logs`).
        let mut logs: HashMap<String, VecDeque<LogEntry>> = HashMap::new();
        // Service whose buffer we last seeded; detects selection changes.
        let mut seeded: String = String::new();
        // Same, for deployment build logs (`Event::BuildLog` → `dep_build_logs`).
        let mut blogs: HashMap<String, VecDeque<BuildLogLine>> = HashMap::new();
        let mut deploy_seeded: String = String::new();

        loop {
            tokio::select! {
                // Live "1s, 2s, 3s…" tick for the deploy started from the open
                // service's detail panel. Purely local (no RPC): `started_at`
                // is cached in `deploy_track` from the `DeployStart` response.
                // Gated on `current == track.service_id` so navigating away
                // from the deploying service doesn't keep ticking its old timer.
                _ = sec_tick.tick() => {
                    let track = deploy_track.lock().map(|t| t.clone()).unwrap_or_default();
                    let current = selected.lock().map(|s| s.clone()).unwrap_or_default();
                    if track.running && track.service_id == current
                        && let Some(started) = track.started_at
                    {
                        let secs = (Utc::now() - started).num_seconds().max(0) as u64;
                        patch!(vec![("svc_deploy_elapsed".into(), view::fmt_secs(secs))]);
                    }
                }
                _ = poll.tick() => {
                    let mut pairs = Vec::new();
                    let term = search.lock().map(|s| s.trim().to_lowercase()).unwrap_or_default();
                    if let Ok(Response::DaemonStatus(d)) = client.rpc(Command::DaemonStatus).await {
                        pairs.push(("daemon_version".into(), d.version.clone()));
                        pairs.push(("daemon_uptime".into(), view::fmt_uptime(d.uptime_secs)));
                        pairs.push(("services_running".into(), d.services_running.to_string()));
                        pairs.push(("services_total".into(), d.services_total.to_string()));
                        pairs.push(("services_label".into(), format!("{}/{}", d.services_running, d.services_total)));
                    }
                    // Raw fetches for this tick — stashed so the search-filtered
                    // keys can be (re)built once, both here and instantly on
                    // keystroke (see `search_pairs`/`SearchCache`).
                    let mut deployments_raw: Vec<DeploymentSummary> = Vec::new();
                    let mut projects_raw: Vec<Project> = Vec::new();
                    let mut all: Vec<(Service, String)> = Vec::new();
                    let mut docker_images_raw: Vec<shared::DockerImageInfo> = Vec::new();
                    let mut docker_volumes_raw: Vec<shared::DockerVolumeInfo> = Vec::new();
                    let mut docker_networks_raw: Vec<shared::DockerNetworkInfo> = Vec::new();

                    if let Ok(Response::DeploymentSummaries(list)) =
                        client.rpc(Command::RecentDeployments { limit: 40 }).await
                    {
                        pairs.push(("deployments_count".into(), list.len().to_string()));
                        deployments_raw = list;
                    }
                    let pid = selected_project.lock().map(|s| s.clone()).unwrap_or_default();
                    if let Ok(Response::Projects(list)) = client.rpc(Command::ProjectList).await {
                        // Fan out one ServiceList per project, tagging each
                        // service with its project name for the grid cards.
                        for p in &list {
                            if let Ok(Response::Services(svcs)) = client.rpc(
                                Command::ServiceList { project_id: p.id.clone() },
                            ).await {
                                for s in svcs {
                                    all.push((s, p.name.clone()));
                                }
                            }
                        }

                        pairs.push(("projects_count".into(), list.len().to_string()));
                        pairs.push(("services_count".into(), all.len().to_string()));

                        // Ingress + Monitoring aren't filtered by the search box,
                        // so they stay built here (not in `search_pairs`).
                        let (ingress, ingress_count) = view::ingress_json(&all);
                        pairs.push(("ingress".into(), ingress));
                        pairs.push(("ingress_count".into(), ingress_count.to_string()));
                        pairs.push(("monitoring".into(), view::monitoring_json(&all, &metrics)));

                        // Open `project_services` header (name/description/can-delete)
                        // — also term-independent; the grid itself is in `search_pairs`.
                        if !pid.is_empty()
                            && let Some(proj) = list.iter().find(|p| p.id == pid)
                        {
                            let has_svcs = all.iter().any(|(s, _)| s.spec.project_id == pid);
                            pairs.push(("proj_name".into(), proj.name.clone()));
                            pairs.push(("proj_description".into(), proj.description.clone().unwrap_or_default()));
                            pairs.push(("proj_can_delete".into(), if has_svcs { "0" } else { "1" }.into()));
                            // Cura o spinner "Carregando dados…" da view de serviços:
                            // o one-shot `fetch_project_services` seta proj_loading=false,
                            // mas se aquele resultado se perde numa corrida com o poll o
                            // flag fica preso. Como aqui já temos os dados do projeto
                            // aberto, limpamos todo tick (mesmo padrão do data_loading).
                            pairs.push(("proj_loading".into(), "false".into()));
                        }
                        projects_raw = list;
                    }
                    // Docker tab: images/volumes/networks across the whole host
                    // (not just rustploy-managed services — see docker_inventory
                    // on the daemon side).
                    if let Ok(Response::DockerImages(list)) = client.rpc(Command::DockerImages).await {
                        pairs.push(("docker_images_count".into(), list.len().to_string()));
                        docker_images_raw = list;
                    }
                    if let Ok(Response::DockerVolumes(list)) = client.rpc(Command::DockerVolumes).await {
                        pairs.push(("docker_volumes_count".into(), list.len().to_string()));
                        docker_volumes_raw = list;
                    }
                    if let Ok(Response::DockerNetworks(list)) = client.rpc(Command::DockerNetworks).await {
                        pairs.push(("docker_networks_count".into(), list.len().to_string()));
                        docker_networks_raw = list;
                    }
                    // Deploy Engine: KPIs + deploys ativos + histórico 24h
                    // (não é filtrado pela busca — fica inline como o ingress).
                    if let Ok(Response::DeployEngineStatus(eng)) =
                        client.rpc(Command::DeployEngineStatus).await
                    {
                        pairs.push(("eng_active_count".into(), eng.active.len().to_string()));
                        pairs.push(("eng_success_24h".into(), eng.successful_24h.to_string()));
                        pairs.push(("eng_failed_24h".into(), eng.failed_24h.to_string()));
                        pairs.push(("eng_total_24h".into(), eng.total_24h.to_string()));
                        pairs.push(("eng_uptime".into(), view::fmt_uptime(eng.uptime_secs)));
                        pairs.push(("eng_active".into(), view::eng_active_json(&eng.active)));
                        pairs.push(("eng_recent".into(), view::eng_recent_json(&eng.recent)));
                        pairs.push(("eng_recent_count".into(), eng.recent.len().to_string()));
                    }

                    // Publish the raw snapshot for the instant keystroke path,
                    // then build the filtered lists for this tick from it.
                    {
                        let mut cache = match search_cache.lock() {
                            Ok(c) => c,
                            Err(p) => p.into_inner(),
                        };
                        cache.projects = projects_raw;
                        cache.services = all;
                        cache.metrics = metrics.clone();
                        cache.deployments = deployments_raw;
                        cache.docker_images = docker_images_raw;
                        cache.docker_volumes = docker_volumes_raw;
                        cache.docker_networks = docker_networks_raw;
                        pairs.extend(search_pairs(&cache, &term, &pid));
                    }
                    if let Some(s) = &sysm {
                        pairs.push(("sys_cpu".into(), format!("{:.0}%", s.cpu_percent)));
                        pairs.push(("sys_mem".into(), format!("{} / {}", view::fmt_bytes(s.mem_used_bytes), view::fmt_bytes(s.mem_total_bytes))));
                        pairs.push(("sys_disk".into(), format!("{} / {}", view::fmt_bytes(s.disk_used_bytes), view::fmt_bytes(s.disk_total_bytes))));
                        pairs.push(("sys_load".into(), format!("{:.2} {:.2} {:.2}", s.load_avg_1, s.load_avg_5, s.load_avg_15)));
                    }
                    pairs.push(("data_loading".into(), "false".into()));
                    patch!(pairs);

                    let current = selected.lock().map(|s| s.clone()).unwrap_or_default();

                    // Watch the in-flight deploy (if any) for completion: stop the
                    // 1Hz ticker and surface the final outcome (with total elapsed
                    // time) once it reaches a terminal state. The book-keeping
                    // (`deploy_track.running = false`) always runs so a finished
                    // deploy doesn't appear to "resume" if the user reopens its
                    // service later; the `svc_deploy_*`/`svc_action_*` patch only
                    // fires while that service's detail panel is still open.
                    let track = deploy_track.lock().map(|t| t.clone()).unwrap_or_default();
                    if track.running && !track.service_id.is_empty()
                        && let Ok(Response::Deployments(history)) = client.rpc(
                            Command::DeployHistory { service_id: track.service_id.clone(), limit: 1 },
                        ).await
                        && let Some(dep) = history.into_iter().next()
                        && dep.id == track.deployment_id
                        && dep.state.is_terminal()
                    {
                        let secs = dep.finished_at
                            .map(|f| (f - dep.started_at).num_seconds())
                            .unwrap_or(0)
                            .max(0) as u64;
                        if let Ok(mut t) = deploy_track.lock() {
                            t.running = false;
                        }
                        let live = dep.state == DeployState::Live;
                        let (msg, color) = if live {
                            (format!("deploy concluído em {}", view::fmt_secs(secs)), "#3FB950")
                        } else {
                            (format!("deploy falhou após {} · {}", view::fmt_secs(secs), dep.state.label()), "#F85149")
                        };
                        // Toasted unconditionally — the whole point is to
                        // notify even if the user has navigated away from
                        // this service's detail panel; the `svc_deploy_*`/
                        // `svc_action_*` patch below stays panel-gated since
                        // those keys only mean something while it's open.
                        let toast = if live { ToastSpec::success(msg.clone()) } else { ToastSpec::error(msg.clone()) };
                        let _ = output.send(EngineMessage::EffectOutcome(EffectOutcome::toast(toast))).await;
                        if track.service_id == current {
                            patch!(vec![
                                ("svc_deploy_running".into(), "false".into()),
                                ("svc_deploy_elapsed".into(), view::fmt_secs(secs)),
                                ("svc_action_msg".into(), msg),
                                ("svc_action_color".into(), color.into()),
                            ]);
                        }
                    }

                    // Seed the live buffer from the historical tail whenever the
                    // selected service changes, so live lines continue from it.
                    if current != seeded {
                        seeded = current.clone();
                        if !current.is_empty()
                            && let Ok(Response::Logs(tail)) = client.rpc(
                                Command::LogsGet { service_id: current.clone(), tail: LOG_RING },
                            ).await
                        {
                            let buf: VecDeque<LogEntry> = tail.into_iter().collect();
                            let snapshot = view::logs_json_buf(&buf);
                            let text = view::join_log_lines(buf.iter().map(|e| (&e.timestamp, e.line.as_str())));
                            let count = buf.len().to_string();
                            logs.insert(current.clone(), buf);
                            patch!(vec![
                                ("svc_logs".into(), snapshot),
                                ("svc_logs_count".into(), count),
                                ("svc_logs_text".into(), text),
                            ]);
                        }
                    }

                    // Same seed-on-change for the selected deployment's build log.
                    let cur_dep = selected_deploy.lock().map(|s| s.clone()).unwrap_or_default();
                    if cur_dep != deploy_seeded {
                        deploy_seeded = cur_dep.clone();
                        if !cur_dep.is_empty()
                            && let Ok(Response::BuildLogs(tail)) = client.rpc(
                                Command::GetBuildLogs { deployment_id: cur_dep.clone() },
                            ).await
                        {
                            let buf: VecDeque<BuildLogLine> = tail.into_iter().collect();
                            let snapshot = view::build_logs_json_buf(&buf);
                            let text = view::join_log_lines(buf.iter().map(|e| (&e.timestamp, e.line.as_str())));
                            let count = buf.len().to_string();
                            blogs.insert(cur_dep.clone(), buf);
                            patch!(vec![
                                ("dep_build_logs".into(), snapshot),
                                ("dep_build_count".into(), count),
                                ("dep_build_text".into(), text),
                            ]);
                        }
                    }
                }
                frame = super::rwp::read_frame::<RwpReply>(&mut evt) => match frame {
                    // Cache the freshest metrics; next poll re-renders the grid.
                    Ok(RwpReply::Event(shared::Event::ContainerMetrics(p))) => {
                        metrics.insert(p.service_id.clone(), p);
                    }
                    Ok(RwpReply::Event(shared::Event::SystemMetrics(p))) => {
                        sysm = Some(p);
                    }
                    // Append live log lines; re-render when it's the open service.
                    Ok(RwpReply::Event(shared::Event::LogLine { service_id, stream, line, timestamp, .. })) => {
                        let buf = logs.entry(service_id.clone()).or_default();
                        buf.push_back(LogEntry { stream, line, timestamp });
                        while buf.len() > LOG_RING {
                            buf.pop_front();
                        }
                        let current = selected.lock().map(|s| s.clone()).unwrap_or_default();
                        if service_id == current {
                            patch!(vec![
                                ("svc_logs".into(), view::logs_json_buf(buf)),
                                ("svc_logs_count".into(), buf.len().to_string()),
                                ("svc_logs_text".into(), view::join_log_lines(buf.iter().map(|e| (&e.timestamp, e.line.as_str())))),
                            ]);
                        }
                    }
                    // Append live build-log lines (no stream split on the wire →
                    // treated as stdout); re-render when it's the open deployment.
                    Ok(RwpReply::Event(shared::Event::BuildLog { deployment_id, line, timestamp, .. })) => {
                        let buf = blogs.entry(deployment_id.clone()).or_default();
                        buf.push_back(BuildLogLine { stream: LogStream::Stdout, line, timestamp });
                        while buf.len() > BUILD_RING {
                            buf.pop_front();
                        }
                        let cur_dep = selected_deploy.lock().map(|s| s.clone()).unwrap_or_default();
                        if deployment_id == cur_dep {
                            patch!(vec![
                                ("dep_build_logs".into(), view::build_logs_json_buf(buf)),
                                ("dep_build_count".into(), buf.len().to_string()),
                                ("dep_build_text".into(), view::join_log_lines(buf.iter().map(|e| (&e.timestamp, e.line.as_str())))),
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
