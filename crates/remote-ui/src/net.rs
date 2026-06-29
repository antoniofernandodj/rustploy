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
use shared::protocol::{BuildLogLine, LogEntry, LogStream};
use shared::{
    Command, ContainerMetricsPoint, DeploymentSummary, DeployState, EnvVar, EnvVarValue, Event,
    GitBranch, GitProvider, GitRepo, GitSource, Healthcheck, HealthcheckKind, Project, Response,
    RwpFrame, RwpReply, Service, ServiceSource, ServiceStatus, SystemMetricsPoint, looks_like_git_url,
};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// How many service cards sit side by side in the Projects grid.
const GRID_COLS: usize = 3;
/// Max log lines kept per service in the live ring buffer.
const LOG_RING: usize = 200;
/// Max build-log lines kept per deployment (builds can be verbose).
const BUILD_RING: usize = 2000;

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

/// Editable General-tab fields extracted from a `ServiceSource`.
#[derive(Default)]
struct GeneralFields {
    repo_url: String,
    branch: String,
    username: String,
    credentials: String,
    build_path: String,
    watch_paths: String,
    submodules: bool,
    dockerfile: String,
    context_path: String,
    build_stage: String,
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

    // Recent stdout/stderr for the LIVE OUTPUT panel (one-shot tail; the live
    // stream takes over via `poll_stream`).
    let logs = match rwp::rpc(&mut conn, Command::LogsGet { service_id: service_id.into(), tail: 200 }).await {
        Ok(Response::Logs(l)) => l,
        _ => Vec::new(),
    };

    // Recent deployments for the Deployments tab.
    let deployments = match rwp::rpc(
        &mut conn,
        Command::DeployHistory { service_id: service_id.into(), limit: 30 },
    ).await {
        Ok(Response::Deployments(d)) => d,
        _ => Vec::new(),
    };

    let (status_label, status_color) = service_status_label_color(&svc.status);
    let (source_kind, source_detail, build_engine) = source_summary(&svc.spec.source);
    let spec = &svc.spec;
    let run_args = if spec.run_args.is_empty() { "—".to_string() } else { spec.run_args.join(" ") };

    // General (source) editable fields.
    let g = match &spec.source {
        ServiceSource::Git(g) => GeneralFields {
            repo_url: g.url.clone(),
            branch: g.branch.clone(),
            username: g.username.clone().unwrap_or_default(),
            credentials: g.credentials.clone().unwrap_or_default(),
            build_path: g.root_path.clone(),
            watch_paths: g.watch_paths.join(", "),
            submodules: g.submodules,
            dockerfile: g.dockerfile_path.clone(),
            context_path: g.build_context.clone(),
            build_stage: g.build_stage.clone().unwrap_or_default(),
        },
        ServiceSource::Registry { image } => GeneralFields {
            repo_url: image.clone(),
            ..GeneralFields::default()
        },
        ServiceSource::Compose(_) => GeneralFields::default(),
    };

    // Editable form fields (Domains / Healthcheck / Advanced) — empty when unset.
    // Source provider binding drives the General sub-tab (Git vs Gitea) and the
    // provider id carried through a Gitea-bound save.
    let (prov_tab, gen_provider_id, bound_url) = match &spec.source {
        ServiceSource::Git(g) if g.provider_id.is_some() => {
            ("gitea", g.provider_id.clone().unwrap_or_default(), g.url.clone())
        }
        _ => ("git", String::new(), String::new()),
    };

    // For a Gitea-bound service, pre-load the provider's repos (and the bound
    // repo's branches) so the picker lists are populated on first paint instead
    // of only after a manual click.
    let mut gitea_extra: Vec<(String, String)> = Vec::new();
    if !gen_provider_id.is_empty()
        && let Ok(Response::GitRepos(repos)) = rwp::rpc(
            &mut conn,
            Command::GitRepoList { provider_id: gen_provider_id.clone() },
        ).await
    {
        let repo_full = repos
            .iter()
            .find(|r| r.clone_url == bound_url)
            .map(|r| r.full_name.clone());
        gitea_extra.push(("gitea_repos".into(), git_repos_json(&repos)));
        gitea_extra.push(("gitea_msg".into(), format!("{} repositório(s)", repos.len())));
        if let Some(full) = repo_full {
            gitea_extra.push(("gitea_repo".into(), full.clone()));
            if let Ok(Response::GitBranches(brs)) = rwp::rpc(
                &mut conn,
                Command::GitBranchList { provider_id: gen_provider_id.clone(), repo_full_name: full },
            ).await {
                gitea_extra.push(("gitea_branches".into(), git_branches_json(&brs)));
            }
        }
    }

    let hc = &spec.healthcheck;
    let (hc_kind, hc_path, hc_status) = match &hc.kind {
        HealthcheckKind::None => ("none", String::new(), String::new()),
        HealthcheckKind::Tcp => ("tcp", String::new(), String::new()),
        HealthcheckKind::DockerNative => ("docker", String::new(), String::new()),
        HealthcheckKind::Http { path, expected_status } => ("http", path.clone(), expected_status.to_string()),
    };

    let mut pairs = vec![
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
        ("svc_run_command".into(), spec.run_command.clone().unwrap_or_else(|| "—".into())),
        ("svc_run_args".into(), run_args),
        ("svc_env".into(), env_json(&spec.env_vars)),
        ("svc_env_count".into(), spec.env_vars.len().to_string()),
        ("svc_env_text".into(), env_dotenv(&spec.env_vars)),
        // Pristine copy so the editor's Cancel can discard edits offline.
        ("svc_env_text_orig".into(), env_dotenv(&spec.env_vars)),
        ("svc_logs".into(), logs_json(&logs)),
        ("svc_logs_count".into(), logs.len().to_string()),
        ("svc_logs_text".into(), join_log_lines(logs.iter().map(|e| (&e.timestamp, e.line.as_str())))),
        ("svc_deployments".into(), deployments_detail_json(&deployments)),
        ("svc_deployments_count".into(), deployments.len().to_string()),
        // Editable fields.
        ("f_domain".into(), spec.domain.clone().unwrap_or_default()),
        ("f_host_port".into(), spec.host_port.map(|p| p.to_string()).unwrap_or_default()),
        ("f_tls".into(), if spec.tls_enabled { "true" } else { "false" }.into()),
        ("f_hc_kind".into(), hc_kind.into()),
        ("f_hc_path".into(), hc_path),
        ("f_hc_status".into(), hc_status),
        ("f_hc_interval".into(), hc.interval_secs.to_string()),
        ("f_hc_timeout".into(), hc.timeout_secs.to_string()),
        ("f_hc_retries".into(), hc.retries.to_string()),
        ("f_hc_start".into(), hc.start_period_secs.to_string()),
        ("f_replicas".into(), spec.replicas.to_string()),
        ("f_run_command".into(), spec.run_command.clone().unwrap_or_default()),
        // General (source) fields.
        ("f_repo_url".into(), g.repo_url),
        ("f_branch".into(), g.branch),
        ("f_username".into(), g.username),
        ("f_credentials".into(), g.credentials),
        ("f_build_path".into(), g.build_path),
        ("f_watch_paths".into(), g.watch_paths),
        ("f_submodules".into(), if g.submodules { "true" } else { "false" }.into()),
        ("f_dockerfile".into(), g.dockerfile),
        ("f_context_path".into(), g.context_path),
        ("f_build_stage".into(), g.build_stage),
        ("f_gen_port".into(), spec.port.to_string()),
        // Provider sub-tab state (Git vs connected Gitea).
        ("prov_tab".into(), prov_tab.into()),
        ("gitea_provider_id".into(), gen_provider_id),
    ];
    pairs.extend(gitea_extra);
    Ok(pairs)
}

/// Stops every running service across all projects (topbar "Stop All").
pub async fn stop_all(addr: String, token: Option<String>) -> Vec<(String, String)> {
    let msg = match stop_all_inner(&addr, token.as_deref()).await {
        Ok(n) => format!("{n} serviço(s) parado(s)"),
        Err(e) => format!("erro: {e}"),
    };
    vec![("status_line".into(), msg)]
}

async fn stop_all_inner(addr: &str, token: Option<&str>) -> anyhow::Result<usize> {
    let mut conn = rwp::connect(addr, token).await?;
    let projects = match rwp::rpc(&mut conn, Command::ProjectList).await? {
        Response::Projects(p) => p,
        other => anyhow::bail!("resposta inesperada: {other:?}"),
    };
    let mut stopped = 0usize;
    for p in &projects {
        if let Ok(Response::Services(svcs)) =
            rwp::rpc(&mut conn, Command::ServiceList { project_id: p.id.clone() }).await
        {
            for s in svcs {
                if s.status == ServiceStatus::Running || s.status == ServiceStatus::Degraded {
                    let _ = rwp::rpc(&mut conn, Command::ServiceStop { service_id: s.id }).await;
                    stopped += 1;
                }
            }
        }
    }
    Ok(stopped)
}

/// Persists the daemon settings (Settings screen). Empty strings clear a field.
pub async fn save_settings(
    addr: String,
    token: Option<String>,
    domain: String,
    email: String,
) -> Vec<(String, String)> {
    let opt = |s: String| if s.trim().is_empty() { None } else { Some(s) };
    let cmd = Command::SetDaemonSettings {
        webhook_base_url: opt(domain),
        acme_email: opt(email),
    };
    let msg = match run_command(addr, token, cmd).await {
        Ok(Response::Ok) => "configurações salvas".to_string(),
        Ok(other) => format!("{other:?}"),
        Err(e) => format!("erro: {e}"),
    };
    vec![("settings_msg".into(), msg)]
}

/// A form-driven edit to a service spec (Domains / Healthcheck / Advanced tabs).
/// Fields arrive as strings from the UI and are parsed here.
pub enum SpecOp {
    Domains { domain: String, host_port: String, tls: bool },
    Healthcheck {
        kind: String,
        http_path: String,
        expected_status: String,
        interval: String,
        timeout: String,
        retries: String,
        start_period: String,
    },
    Advanced { replicas: String, run_command: String },
    General {
        repo_url: String,
        branch: String,
        username: String,
        credentials: String,
        build_path: String,
        watch_paths: String,
        submodules: bool,
        dockerfile: String,
        context_path: String,
        build_stage: String,
        port: String,
        /// When non-empty, binds the source to this connected Git provider
        /// (set by the Gitea picker); empty keeps whatever was there.
        provider_id: String,
    },
}

/// Applies a [`SpecOp`] to the service (fetch fresh spec → mutate → update),
/// then re-fetches the detail so the panel reflects the change.
pub async fn run_spec_op(
    addr: String,
    token: Option<String>,
    service_id: String,
    op: SpecOp,
) -> Vec<(String, String)> {
    let msg = match apply_spec_op(&addr, token.as_deref(), &service_id, op).await {
        Ok(_) => "salvo".to_string(),
        Err(e) => format!("erro: {e}"),
    };
    let mut pairs = fetch_service_detail(addr, token, service_id).await;
    pairs.push(("svc_action_msg".into(), msg));
    pairs
}

async fn apply_spec_op(
    addr: &str,
    token: Option<&str>,
    service_id: &str,
    op: SpecOp,
) -> anyhow::Result<()> {
    let mut conn = rwp::connect(addr, token).await?;
    let svc = match rwp::rpc(&mut conn, Command::ServiceGet { id: service_id.into() }).await? {
        Response::Service(s) => s,
        other => anyhow::bail!("resposta inesperada para ServiceGet: {other:?}"),
    };
    let mut spec = svc.spec;
    let trimmed = |s: String| {
        let t = s.trim().to_string();
        if t.is_empty() { None } else { Some(t) }
    };
    match op {
        SpecOp::Domains { domain, host_port, tls } => {
            spec.domain = trimmed(domain);
            spec.host_port = host_port.trim().parse::<u16>().ok();
            spec.tls_enabled = tls;
        }
        SpecOp::Healthcheck { kind, http_path, expected_status, interval, timeout, retries, start_period } => {
            let cur = &spec.healthcheck;
            let num = |s: String, d: u32| s.trim().parse::<u32>().unwrap_or(d);
            spec.healthcheck = Healthcheck {
                kind: match kind.as_str() {
                    "none" => HealthcheckKind::None,
                    "http" => HealthcheckKind::Http {
                        path: if http_path.trim().is_empty() { "/".into() } else { http_path.trim().into() },
                        expected_status: expected_status.trim().parse::<u16>().unwrap_or(200),
                    },
                    "docker" => HealthcheckKind::DockerNative,
                    _ => HealthcheckKind::Tcp,
                },
                interval_secs: num(interval, cur.interval_secs),
                timeout_secs: num(timeout, cur.timeout_secs),
                retries: num(retries, cur.retries),
                start_period_secs: num(start_period, cur.start_period_secs),
            };
        }
        SpecOp::Advanced { replicas, run_command } => {
            spec.replicas = replicas.trim().parse::<u32>().unwrap_or(1).max(1);
            spec.run_command = trimmed(run_command);
        }
        SpecOp::General {
            repo_url, branch, username, credentials, build_path, watch_paths,
            submodules, dockerfile, context_path, build_stage, port, provider_id,
        } => {
            if let Ok(p) = port.trim().parse::<u16>() {
                spec.port = p;
            }
            let non_empty = |s: String, d: &str| {
                let t = s.trim().to_string();
                if t.is_empty() { d.to_string() } else { t }
            };
            // The Gitea sub-tab binds a provider id; the Git sub-tab sends an
            // empty id, detaching the source from any provider (raw URL).
            let provider_id = if provider_id.trim().is_empty() {
                None
            } else {
                Some(provider_id.trim().to_string())
            };
            let git = ServiceSource::Git(GitSource {
                url: repo_url.trim().to_string(),
                branch: non_empty(branch, "main"),
                root_path: non_empty(build_path, "."),
                watch_paths: watch_paths
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                submodules,
                dockerfile_path: non_empty(dockerfile, "Dockerfile"),
                build_context: non_empty(context_path, "."),
                build_stage: trimmed(build_stage),
                credentials: trimmed(credentials),
                username: trimmed(username),
                provider_id,
            });
            // Registry stays a registry unless the URL clearly points at a repo;
            // a Git source rebuilds from the form. Compose is left untouched.
            spec.source = if matches!(spec.source, ServiceSource::Compose(_)) {
                spec.source.clone()
            } else if matches!(spec.source, ServiceSource::Git(_)) || looks_like_git_url(repo_url.trim()) {
                git
            } else {
                ServiceSource::Registry { image: repo_url.trim().to_string() }
            };
        }
    }
    match rwp::rpc(&mut conn, Command::ServiceUpdate { id: service_id.into(), spec }).await? {
        Response::Ok | Response::Service(_) => Ok(()),
        Response::Err { code, message } => anyhow::bail!("{code}: {message}"),
        other => anyhow::bail!("resposta inesperada para ServiceUpdate: {other:?}"),
    }
}

/// An edit to a service's environment variables.
pub enum EnvOp {
    /// Add or replace `key` with a plain value.
    Set { key: String, value: String },
    /// Remove `key`.
    Delete { key: String },
    /// Replace ALL variables with the parsed contents of a `.env` blob.
    ImportDotenv(String),
}

/// Parses a `.env` blob (`KEY=VALUE`, `#` comments, blanks ignored). A value of
/// the form `<secret:NAME>` round-trips back to a secret reference.
fn parse_dotenv(text: &str) -> Vec<EnvVar> {
    text.lines()
        .filter_map(|line| {
            let l = line.trim();
            if l.is_empty() || l.starts_with('#') {
                return None;
            }
            let (k, v) = l.split_once('=')?;
            let key = k.trim().to_string();
            if key.is_empty() {
                return None;
            }
            let v = v.trim();
            let value = match v.strip_prefix("<secret:").and_then(|s| s.strip_suffix('>')) {
                Some(name) => EnvVarValue::Secret(name.to_string()),
                None => EnvVarValue::Plain(v.to_string()),
            };
            Some(EnvVar { key, value })
        })
        .collect()
}

/// Applies an [`EnvOp`] to the service (fetch fresh spec → mutate → update),
/// then re-fetches the detail so the panel reflects the change.
pub async fn run_env_op(
    addr: String,
    token: Option<String>,
    service_id: String,
    op: EnvOp,
) -> Vec<(String, String)> {
    let msg = match apply_env_op(&addr, token.as_deref(), &service_id, op).await {
        Ok(_) => "env atualizado".to_string(),
        Err(e) => format!("erro: {e}"),
    };
    let mut pairs = fetch_service_detail(addr, token, service_id).await;
    pairs.push(("svc_action_msg".into(), msg));
    pairs
}

async fn apply_env_op(
    addr: &str,
    token: Option<&str>,
    service_id: &str,
    op: EnvOp,
) -> anyhow::Result<()> {
    let mut conn = rwp::connect(addr, token).await?;
    let svc = match rwp::rpc(&mut conn, Command::ServiceGet { id: service_id.into() }).await? {
        Response::Service(s) => s,
        other => anyhow::bail!("resposta inesperada para ServiceGet: {other:?}"),
    };
    let mut spec = svc.spec;
    match op {
        EnvOp::Set { key, value } => {
            spec.env_vars.retain(|v| v.key != key);
            spec.env_vars.push(EnvVar { key, value: EnvVarValue::Plain(value) });
        }
        EnvOp::Delete { key } => spec.env_vars.retain(|v| v.key != key),
        EnvOp::ImportDotenv(text) => spec.env_vars = parse_dotenv(&text),
    }
    match rwp::rpc(&mut conn, Command::ServiceUpdate { id: service_id.into(), spec }).await? {
        Response::Ok | Response::Service(_) => Ok(()),
        Response::Err { code, message } => anyhow::bail!("{code}: {message}"),
        other => anyhow::bail!("resposta inesperada para ServiceUpdate: {other:?}"),
    }
}

/// Fetches the build log of a single deployment for the Deployments tab.
pub async fn fetch_build_logs(
    addr: String,
    token: Option<String>,
    deployment_id: String,
) -> Vec<(String, String)> {
    let lines = match run_command(addr, token, Command::GetBuildLogs { deployment_id: deployment_id.clone() }).await {
        Ok(Response::BuildLogs(l)) => l,
        _ => Vec::new(),
    };
    vec![
        ("dep_selected".into(), deployment_id),
        ("dep_build_logs".into(), build_logs_json(&lines)),
        ("dep_build_count".into(), lines.len().to_string()),
        ("dep_build_text".into(), join_log_lines(lines.iter().map(|e| (&e.timestamp, e.line.as_str())))),
    ]
}

// ── Gitea picker (General tab) ──────────────────────────────────────────────

/// Renders an unexpected/`Err` response into a one-line, human-readable message.
fn resp_msg(r: &Response) -> String {
    match r {
        Response::Err { code, message } => format!("erro: {code}: {message}"),
        other => format!("resposta inesperada: {other:?}"),
    }
}

/// Lists the connected Git providers for the General-tab picker.
pub async fn fetch_git_providers(addr: String, token: Option<String>) -> Vec<(String, String)> {
    match run_command(addr, token, Command::GitProviderList).await {
        Ok(Response::GitProviders(list)) => {
            let msg = if list.is_empty() {
                "nenhum provider conectado — configure em Settings".to_string()
            } else {
                String::new()
            };
            vec![
                ("gitea_providers".into(), git_providers_json(&list)),
                ("gitea_count".into(), list.len().to_string()),
                ("gitea_msg".into(), msg),
            ]
        }
        Ok(other) => vec![("gitea_msg".into(), resp_msg(&other))],
        Err(e) => vec![("gitea_msg".into(), format!("erro: {e}"))],
    }
}

/// Lists the repositories of a provider; resets the repo/branch selection.
pub async fn fetch_git_repos(
    addr: String,
    token: Option<String>,
    provider_id: String,
) -> Vec<(String, String)> {
    let pid = provider_id.clone();
    match run_command(addr, token, Command::GitRepoList { provider_id }).await {
        Ok(Response::GitRepos(list)) => vec![
            ("gitea_provider_id".into(), pid),
            ("gitea_repos".into(), git_repos_json(&list)),
            ("gitea_branches".into(), "[]".into()),
            ("gitea_repo".into(), String::new()),
            ("gitea_msg".into(), format!("{} repositório(s)", list.len())),
        ],
        Ok(other) => vec![("gitea_msg".into(), resp_msg(&other))],
        Err(e) => vec![("gitea_msg".into(), format!("erro: {e}"))],
    }
}

/// Lists the branches of a repository for the branch picker.
pub async fn fetch_git_branches(
    addr: String,
    token: Option<String>,
    provider_id: String,
    repo_full_name: String,
) -> Vec<(String, String)> {
    match run_command(addr, token, Command::GitBranchList { provider_id, repo_full_name }).await {
        Ok(Response::GitBranches(list)) => vec![
            ("gitea_branches".into(), git_branches_json(&list)),
            ("gitea_msg".into(), format!("{} branch(es)", list.len())),
        ],
        Ok(other) => vec![("gitea_msg".into(), resp_msg(&other))],
        Err(e) => vec![("gitea_msg".into(), format!("erro: {e}"))],
    }
}

/// Re-fetches the provider list and returns the context pairs (`gitea_*`) plus
/// `gp_msg`. Shared by connect/delete/refresh so the list stays in one place.
async fn providers_refresh_pairs(
    conn: &mut rwp::RwpStream,
    msg: String,
) -> Vec<(String, String)> {
    let mut pairs = vec![("gp_msg".into(), msg)];
    if let Ok(Response::GitProviders(list)) = rwp::rpc(conn, Command::GitProviderList).await {
        pairs.push(("gitea_providers".into(), git_providers_json(&list)));
        pairs.push(("gitea_count".into(), list.len().to_string()));
    }
    pairs
}

/// Registers a new Gitea provider (Settings → Git). On OAuth it then starts the
/// authorization flow and opens the browser; on PAT the account is usable at
/// once. Clears the form fields and refreshes the connected list.
#[allow(clippy::too_many_arguments)]
pub async fn git_provider_connect(
    addr: String,
    token: Option<String>,
    name: String,
    base_url: String,
    mode: String,
    client_id: String,
    client_secret: String,
    pat: String,
) -> Vec<(String, String)> {
    if base_url.trim().is_empty() {
        return vec![("gp_msg".into(), "informe a Base URL do Gitea".into())];
    }
    let name = if name.trim().is_empty() { "Gitea".to_string() } else { name.trim().to_string() };
    let is_oauth = mode != "pat";

    let cmd = if is_oauth {
        if client_id.trim().is_empty() || client_secret.trim().is_empty() {
            return vec![("gp_msg".into(), "Client ID e Client Secret são obrigatórios".into())];
        }
        Command::GitProviderCreate {
            kind: shared::GitProviderKind::Gitea,
            name,
            base_url: base_url.trim().to_string(),
            auth_mode: shared::GitAuthMode::OAuth,
            oauth_client_id: Some(client_id.trim().to_string()),
            oauth_client_secret: Some(client_secret.clone()),
            pat: None,
        }
    } else {
        if pat.trim().is_empty() {
            return vec![("gp_msg".into(), "informe o Personal Access Token".into())];
        }
        Command::GitProviderCreate {
            kind: shared::GitProviderKind::Gitea,
            name,
            base_url: base_url.trim().to_string(),
            auth_mode: shared::GitAuthMode::Pat,
            oauth_client_id: None,
            oauth_client_secret: None,
            pat: Some(pat.clone()),
        }
    };

    let mut conn = match rwp::connect(&addr, token.as_deref()).await {
        Ok(c) => c,
        Err(e) => return vec![("gp_msg".into(), format!("erro: {e}"))],
    };

    let provider_id = match rwp::rpc(&mut conn, cmd).await {
        Ok(Response::GitProviderInfo(p)) => p.id,
        Ok(other) => return vec![("gp_msg".into(), resp_msg(&other))],
        Err(e) => return vec![("gp_msg".into(), format!("erro: {e}"))],
    };

    // OAuth needs a browser round-trip; PAT is immediately usable.
    let msg = if is_oauth {
        match rwp::rpc(&mut conn, Command::GitOAuthStart { provider_id }).await {
            Ok(Response::OAuthUrl(url)) => {
                if open_in_browser(&url) {
                    "navegador aberto — autorize e clique em Atualizar lista".to_string()
                } else {
                    format!("abra para autorizar: {url}")
                }
            }
            Ok(other) => resp_msg(&other),
            Err(e) => format!("erro: {e}"),
        }
    } else {
        "conta Gitea conectada ✓".to_string()
    };

    let mut pairs = providers_refresh_pairs(&mut conn, msg).await;
    // Clear the connect form.
    for k in ["gp_name", "gp_base_url", "gp_client_id", "gp_client_secret", "gp_pat"] {
        pairs.push((k.into(), String::new()));
    }
    pairs
}

/// Removes a connected provider and refreshes the list.
pub async fn git_provider_delete(
    addr: String,
    token: Option<String>,
    id: String,
) -> Vec<(String, String)> {
    let mut conn = match rwp::connect(&addr, token.as_deref()).await {
        Ok(c) => c,
        Err(e) => return vec![("gp_msg".into(), format!("erro: {e}"))],
    };
    let msg = match rwp::rpc(&mut conn, Command::GitProviderDelete { id }).await {
        Ok(Response::Ok) => "provider removido".to_string(),
        Ok(other) => resp_msg(&other),
        Err(e) => format!("erro: {e}"),
    };
    providers_refresh_pairs(&mut conn, msg).await
}

/// One-shot provider list refresh for the "Atualizar lista" button.
pub async fn git_providers_only(addr: String, token: Option<String>) -> Vec<(String, String)> {
    let mut conn = match rwp::connect(&addr, token.as_deref()).await {
        Ok(c) => c,
        Err(e) => return vec![("gp_msg".into(), format!("erro: {e}"))],
    };
    providers_refresh_pairs(&mut conn, String::new()).await
}

/// The Gitea OAuth callback URI the user must register in their Gitea app:
/// `{domain}/oauth/gitea/callback` (matches the daemon's webhook server).
/// Empty domain yields a hint placeholder.
pub fn oauth_redirect_uri(domain: &str) -> String {
    let d = domain.trim().trim_end_matches('/');
    if d.is_empty() {
        "<configure o domínio em Web Server>/oauth/gitea/callback".to_string()
    } else {
        format!("{d}/oauth/gitea/callback")
    }
}

/// Best-effort: opens `url` in the user's default browser (`xdg-open`).
fn open_in_browser(url: &str) -> bool {
    std::process::Command::new("xdg-open")
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

fn git_providers_json(list: &[GitProvider]) -> String {
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

fn git_repos_json(list: &[GitRepo]) -> String {
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

fn git_branches_json(list: &[GitBranch]) -> String {
    let rows: Vec<serde_json::Value> = list
        .iter()
        .map(|b| serde_json::json!({ "name": b.name }))
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Long-lived polling + event stream feeding the context. Yields
/// `EngineMessage::ContextPatch` items.
///
/// `selected` mirrors the service open in the detail view; the live log stream
/// reads it to decide which `LogLine` events to surface as `svc_logs`.
/// `selected_deploy` does the same for `BuildLog` events → `dep_build_logs`.
pub fn poll_stream(
    addr: String,
    token: Option<String>,
    selected: Arc<Mutex<String>>,
    selected_deploy: Arc<Mutex<String>>,
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

        // Daemon settings fetched once on connect so the editable fields are not
        // clobbered by polling while the user types.
        if let Ok(Response::DaemonSettings { webhook_base_url, acme_email }) =
            rwp::rpc(&mut cmd, Command::GetDaemonSettings).await
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
            rwp::rpc(&mut cmd, Command::GitProviderList).await
        {
            patch!(vec![
                ("gitea_providers".into(), git_providers_json(&list)),
                ("gitea_count".into(), list.len().to_string()),
            ]);
        }

        let mut poll = tokio::time::interval(std::time::Duration::from_secs(2));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

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

                        // Derived home screens (Ingress routes, Docker, Monitoring).
                        let (ingress, ingress_count) = ingress_json(&all);
                        pairs.push(("ingress".into(), ingress));
                        pairs.push(("ingress_count".into(), ingress_count.to_string()));
                        pairs.push(("docker_rows".into(), docker_json(&all)));
                        pairs.push(("monitoring".into(), monitoring_json(&all, &metrics)));
                    }
                    if let Some(s) = &sysm {
                        pairs.push(("sys_cpu".into(), format!("{:.0}%", s.cpu_percent)));
                        pairs.push(("sys_mem".into(), format!("{} / {}", fmt_bytes(s.mem_used_bytes), fmt_bytes(s.mem_total_bytes))));
                        pairs.push(("sys_disk".into(), format!("{} / {}", fmt_bytes(s.disk_used_bytes), fmt_bytes(s.disk_total_bytes))));
                        pairs.push(("sys_load".into(), format!("{:.2} {:.2} {:.2}", s.load_avg_1, s.load_avg_5, s.load_avg_15)));
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
                            let text = join_log_lines(buf.iter().map(|e| (&e.timestamp, e.line.as_str())));
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
                            && let Ok(Response::BuildLogs(tail)) = rwp::rpc(
                                &mut cmd,
                                Command::GetBuildLogs { deployment_id: cur_dep.clone() },
                            ).await
                        {
                            let buf: VecDeque<BuildLogLine> = tail.into_iter().collect();
                            let snapshot = build_logs_json_buf(&buf);
                            let text = join_log_lines(buf.iter().map(|e| (&e.timestamp, e.line.as_str())));
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
                frame = rwp::read_frame::<RwpReply>(&mut evt) => match frame {
                    // Cache the freshest metrics; next poll re-renders the grid.
                    Ok(RwpReply::Event(Event::ContainerMetrics(p))) => {
                        metrics.insert(p.service_id.clone(), p);
                    }
                    Ok(RwpReply::Event(Event::SystemMetrics(p))) => {
                        sysm = Some(p);
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
                                ("svc_logs_text".into(), join_log_lines(buf.iter().map(|e| (&e.timestamp, e.line.as_str())))),
                            ]);
                        }
                    }
                    // Append live build-log lines (no stream split on the wire →
                    // treated as stdout); re-render when it's the open deployment.
                    Ok(RwpReply::Event(Event::BuildLog { deployment_id, line, timestamp, .. })) => {
                        let buf = blogs.entry(deployment_id.clone()).or_default();
                        buf.push_back(BuildLogLine { stream: LogStream::Stdout, line, timestamp });
                        while buf.len() > BUILD_RING {
                            buf.pop_front();
                        }
                        let cur_dep = selected_deploy.lock().map(|s| s.clone()).unwrap_or_default();
                        if deployment_id == cur_dep {
                            patch!(vec![
                                ("dep_build_logs".into(), build_logs_json_buf(buf)),
                                ("dep_build_count".into(), buf.len().to_string()),
                                ("dep_build_text".into(), join_log_lines(buf.iter().map(|e| (&e.timestamp, e.line.as_str())))),
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

/// Ingress route rows derived from services that expose a domain.
/// Returns `(json, count)`.
fn ingress_json(all: &[(Service, String)]) -> (String, usize) {
    let rows: Vec<serde_json::Value> = all
        .iter()
        .filter_map(|(s, proj)| {
            let domain = s.spec.domain.clone()?;
            if domain.trim().is_empty() {
                return None;
            }
            let scheme = if s.spec.tls_enabled { "https" } else { "http" };
            Some(serde_json::json!({
                "domain": domain,
                "url": format!("{scheme}://{domain}"),
                "service": s.spec.name,
                "project": proj,
                "upstream": format!(":{}", s.spec.port),
                "tls": if s.spec.tls_enabled { "TLS" } else { "—" },
            }))
        })
        .collect();
    let count = rows.len();
    (serde_json::Value::Array(rows).to_string(), count)
}

/// Docker container rows derived from services (one per managed service).
fn docker_json(all: &[(Service, String)]) -> String {
    let rows: Vec<serde_json::Value> = all
        .iter()
        .map(|(s, proj)| {
            let (status_label, status_color) = service_status_label_color(&s.status);
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
                "status_color": status_color,
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Per-container live metrics rows for the Monitoring screen.
fn monitoring_json(all: &[(Service, String)], metrics: &HashMap<String, ContainerMetricsPoint>) -> String {
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
fn deployments_detail_json(list: &[shared::Deployment]) -> String {
    let rows: Vec<serde_json::Value> = list
        .iter()
        .map(|d| {
            let (label, color) = state_label_color(&d.state);
            serde_json::json!({
                "id": d.id.chars().take(12).collect::<String>(),
                "id_full": d.id,
                "image": d.image,
                "state_label": label,
                "state_color": color,
                "duration": fmt_duration(d),
                "start": d.started_at.with_timezone(&Local).format("%d/%m %H:%M:%S").to_string(),
            })
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// Env vars rendered as a `.env` text blob (KEY=VALUE, secrets by reference).
fn env_dotenv(vars: &[EnvVar]) -> String {
    vars.iter()
        .map(|v| {
            let val = match &v.value {
                EnvVarValue::Plain(s) => s.clone(),
                EnvVarValue::Secret(name) => format!("<secret:{name}>"),
            };
            format!("{}={}", v.key, val)
        })
        .collect::<Vec<_>>()
        .join("\n")
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

/// Build log lines as JSON rows, colored by stream.
fn build_logs_json(lines: &[BuildLogLine]) -> String {
    build_logs_json_iter(lines.iter())
}

/// Same as [`build_logs_json`] over the live ring buffer.
fn build_logs_json_buf(buf: &VecDeque<BuildLogLine>) -> String {
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
fn join_log_lines<'a>(
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
