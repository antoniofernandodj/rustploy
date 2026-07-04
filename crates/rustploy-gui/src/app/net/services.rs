//! Service-scoped RPCs: everything that acts on (or reads the detail of) a
//! single `Service`. Each method borrows the [`RwpClient`] the struct was
//! built with instead of opening its own connection.

use super::view;
use super::{outcome_toast, DeployTrack, DetailCacheHandle, RwpClient};
use glacier_ui::{EffectOutcome, ToastSpec};
use shared::{
    Command, EnvComment, EnvVar, EnvVarValue, Healthcheck, HealthcheckKind, Response, ServiceSource,
    looks_like_git_url,
};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

pub struct Services {
    client: RwpClient,
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

/// A form-driven edit to a service spec (Domains / Healthcheck / Advanced tabs).
/// Fields arrive as strings from the UI and are parsed here.
pub enum SpecOp {
    /// Adiciona (ou atualiza, por domínio) uma rota HTTP: domínio, porta de
    /// container opcional (vazio = porta padrão do serviço) e TLS.
    DomainAdd { domain: String, port: String, tls: bool },
    /// Remove a rota HTTP do domínio informado.
    DomainRemove { domain: String },
    /// Porta TCP crua exposta no host (passthrough). Vazio = sem porta.
    HostPort { host_port: String },
    /// Substitui o YAML do compose de um serviço Compose.
    Compose { content: String },
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

/// An edit to a service's environment variables.
pub enum EnvOp {
    /// Add or replace `key` with a plain value.
    Set { key: String, value: String },
    /// Remove `key`.
    Delete { key: String },
    /// Replace ALL variables with the parsed contents of a `.env` blob.
    ImportDotenv(String),
    /// Reorder the Environment tab's rows to match this sequence of keys
    /// (drag-and-drop). The list interleaves var keys and comment rows
    /// (`__c<idx>` = index into `env_comments`, see `env_json_with_comments`):
    /// vars are reordered to match, and each comment is re-anchored to the
    /// first real var that follows it in the new order (`before_key: None`
    /// when it lands after every var). Items not mentioned keep their place at
    /// the end — a defensive fallback, since `keys` should always cover every
    /// row.
    Reorder(Vec<String>),
}

impl Services {
    pub fn new(client: RwpClient) -> Self {
        Self { client }
    }

    /// Runs a lifecycle command against `service_id`, then re-fetches its
    /// detail so the panel reflects the new state. Surfaces a one-line
    /// outcome in `svc_action_msg`.
    pub async fn run_action(self, cmd: Command, service_id: String) -> EffectOutcome {
        let msg = match self.client.rpc(cmd).await {
            Ok(Response::Ok) => "ação concluída".to_string(),
            Ok(Response::Deployment(d)) => format!("deploy iniciado · {}", d.state.label()),
            Ok(other) => format!("{other:?}"),
            Err(e) => format!("erro: {e}"),
        };
        let mut pairs = self.fetch_detail(service_id).await;
        pairs.push(("svc_action_msg".into(), msg.clone()));
        outcome_toast(pairs, &msg)
    }

    /// Starts a deploy (`Command::DeployStart`) for `service_id` and arms
    /// `deploy_shared` so `poll_stream` takes over: ticking `svc_deploy_elapsed`
    /// once a second while it runs, and surfacing the final outcome (success/
    /// failure, with total elapsed time) once the deployment reaches a
    /// terminal state.
    pub async fn start_deploy(
        self,
        service_id: String,
        deploy_shared: Arc<Mutex<DeployTrack>>,
    ) -> EffectOutcome {
        let resp = self
            .client
            .rpc(Command::DeployStart { service_id: service_id.clone() })
            .await;

        let mut pairs = Vec::new();
        let toast;
        match &resp {
            Ok(Response::Deployment(d)) => {
                if let Ok(mut t) = deploy_shared.lock() {
                    *t = DeployTrack {
                        service_id: service_id.clone(),
                        deployment_id: d.id.clone(),
                        started_at: Some(d.started_at),
                        running: true,
                    };
                }
                pairs.push(("svc_deploy_running".into(), "true".into()));
                pairs.push(("svc_deploy_elapsed".into(), "0s".into()));
                pairs.push(("svc_action_msg".into(), format!("deploy iniciado · {}", d.state.label())));
                pairs.push(("svc_action_color".into(), "#58A6FF".into()));
                // The terminal outcome (success/failure) is toasted separately by
                // `poll_stream`, once the deployment actually finishes — this one
                // just confirms the request was accepted.
                toast = ToastSpec::info("Deploy iniciado.");
            }
            Ok(other) => {
                let msg = view::resp_msg(other);
                pairs.push(("svc_action_msg".into(), msg.clone()));
                pairs.push(("svc_action_color".into(), "#F85149".into()));
                toast = ToastSpec::error(format!("Falha ao iniciar deploy: {msg}"));
            }
            Err(e) => {
                pairs.push(("svc_action_msg".into(), format!("erro: {e}")));
                pairs.push(("svc_action_color".into(), "#F85149".into()));
                toast = ToastSpec::error(format!("Falha ao iniciar deploy: {e}"));
            }
        }
        pairs.extend(self.fetch_detail(service_id).await);
        EffectOutcome::data(pairs).with_toast(toast)
    }

    /// [`Self::fetch_detail`] plus a write-through into `cache` on success, so
    /// a later reopen of this service paints from RAM instantly. Used only by
    /// the open-detail path; chained callers (env/spec edits, deploy) keep
    /// calling `fetch_detail` directly — the next open refreshes the cache
    /// anyway.
    pub async fn fetch_detail_cached(
        self,
        cache: DetailCacheHandle,
        service_id: String,
    ) -> EffectOutcome {
        let pairs = self.fetch_detail(service_id.clone()).await;
        // Only cache a successful load — the error path carries no `svc_id`.
        if pairs.iter().any(|(k, _)| k == "svc_id")
            && let Ok(mut c) = cache.lock()
        {
            c.insert_service(service_id, pairs.clone());
        }
        EffectOutcome::data(pairs)
    }

    /// One-shot fetch of everything the Service detail screen needs: the full
    /// `Service` spec/status, its project name and the most recent container
    /// logs. Returns context pairs (`svc_*`) merged by a `ctx.perform` effect.
    pub async fn fetch_detail(self, service_id: String) -> Vec<(String, String)> {
        match self.fetch_detail_inner(&service_id).await {
            Ok(pairs) => pairs,
            Err(e) => vec![
                ("svc_loading".into(), "false".into()),
                ("svc_error".into(), e.to_string()),
            ],
        }
    }

    async fn fetch_detail_inner(&self, service_id: &str) -> anyhow::Result<Vec<(String, String)>> {
        let client = &self.client;
        let svc = match client.rpc(Command::ServiceGet { id: service_id.into() }).await? {
            Response::Service(s) => s,
            other => anyhow::bail!("resposta inesperada para ServiceGet: {other:?}"),
        };

        // Resolve the project name (ServiceGet only carries the id).
        let mut project_name = svc.spec.project_id.clone();
        if let Ok(Response::Projects(list)) = client.rpc(Command::ProjectList).await
            && let Some(p) = list.iter().find(|p| p.id == svc.spec.project_id)
        {
            project_name = p.name.clone();
        }

        // Recent stdout/stderr for the LIVE OUTPUT panel (one-shot tail; the live
        // stream takes over via `poll_stream`).
        let logs = match client.rpc(Command::LogsGet { service_id: service_id.into(), tail: 200 }).await {
            Ok(Response::Logs(l)) => l,
            _ => Vec::new(),
        };

        // Recent deployments for the Deployments tab.
        let deployments = match client.rpc(
            Command::DeployHistory { service_id: service_id.into(), limit: 30 },
        ).await {
            Ok(Response::Deployments(d)) => d,
            _ => Vec::new(),
        };

        let (status_label, status_color) = view::service_status_label_color(&svc.status);
        let (source_kind, source_detail, build_engine) = view::source_summary(&svc.spec.source);
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

        // Compose services edit their YAML directly in the General tab instead of
        // the Git/Registry provider form.
        let compose_content = match &spec.source {
            ServiceSource::Compose(c) => c.content.clone(),
            _ => String::new(),
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

        // Connected providers for the CONTA GITEA picker, folded into this one-shot
        // load (reusing the open connection) so `svc_loading` only clears once every
        // picker list is ready — the selects never paint empty before their data.
        let mut gitea_extra: Vec<(String, String)> = Vec::new();
        if let Ok(Response::GitProviders(provs)) =
            client.rpc(Command::GitProviderList).await
        {
            gitea_extra.push(("gitea_providers".into(), view::git_providers_json(&provs)));
            gitea_extra.push(("gitea_count".into(), provs.len().to_string()));
        }

        // For a Gitea-bound service, pre-load the provider's repos (and the bound
        // repo's branches) so the picker lists are populated on first paint instead
        // of only after a manual click.
        if !gen_provider_id.is_empty()
            && let Ok(Response::GitRepos(repos)) = client.rpc(
                Command::GitRepoList { provider_id: gen_provider_id.clone() },
            ).await
        {
            let repo_full = repos
                .iter()
                .find(|r| r.clone_url == bound_url)
                .map(|r| r.full_name.clone());
            gitea_extra.push(("gitea_repos".into(), view::git_repos_json(&repos)));
            gitea_extra.push(("gitea_msg".into(), format!("{} repositório(s)", repos.len())));
            if let Some(full) = repo_full {
                gitea_extra.push(("gitea_repo".into(), full.clone()));
                if let Ok(Response::GitBranches(brs)) = client.rpc(
                    Command::GitBranchList { provider_id: gen_provider_id.clone(), repo_full_name: full },
                ).await {
                    gitea_extra.push(("gitea_branches".into(), view::git_branches_json(&brs)));
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

        // Primary (first) domain route drives the single-value Connection fields.
        let primary_route = spec.domain_routes().into_iter().next();

        let mut pairs = vec![
            ("svc_loading".into(), "false".into()),
            ("svc_error".into(), String::new()),
            ("svc_id".into(), svc.id.clone()),
            ("svc_name".into(), spec.name.clone()),
            ("svc_project".into(), project_name),
            ("svc_project_id".into(), spec.project_id.clone()),
            ("svc_status_label".into(), status_label.into()),
            ("svc_status_color".into(), status_color.into()),
            ("svc_source_kind".into(), source_kind.into()),
            ("svc_source_detail".into(), source_detail),
            // YAML do compose (vazio p/ Git/Registry) + cópia pristina p/ o Cancel.
            ("svc_compose".into(), compose_content.clone()),
            ("svc_compose_orig".into(), compose_content),
            ("svc_build".into(), build_engine),
            ("svc_port".into(), spec.port.to_string()),
            ("svc_host_port".into(), spec.host_port.map(|p| p.to_string()).unwrap_or_else(|| "—".into())),
            // Connection tab shows the primary (first) domain route; the Domains tab
            // lists them all (svc_domains). Legacy specs fold into a single route.
            ("svc_domain".into(), primary_route.as_ref().map(|r| r.domain.clone()).unwrap_or_else(|| "—".into())),
            ("svc_tls".into(), if primary_route.as_ref().map(|r| r.tls).unwrap_or(false) { "enabled" } else { "disabled" }.into()),
            ("svc_replicas".into(), spec.replicas.to_string()),
            // JSON list rendered by the Domains tab (domain + container port + TLS).
            ("svc_domains".into(), view::domains_json(&spec)),
            ("svc_domains_count".into(), spec.domain_routes().len().to_string()),
            // Public URL served by the ingress, from the primary domain route.
            // `—` when there's no domain (the service isn't exposed outside the box).
            ("svc_external_url".into(), view::external_url(primary_route.as_ref().map(|r| r.domain.as_str()), primary_route.as_ref().map(|r| r.tls).unwrap_or(false))),
            // Internal connection URL, resolvable by any other service in the same
            // project (they share the `rp_net_<project_id>` bridge network — the
            // container/alias is `rp_<safe_name>` for both Application and Compose
            // services). The scheme comes from `db_kind` (postgres://, amqp://,
            // nats://, …) so it's a complete, copy-pasteable URL; plain web
            // services fall back to `http://`, and Kafka is host:port with no
            // scheme (the bootstrap-servers format). Paste into another service's
            // env vars (e.g. `API_URL=…`).
            ("svc_internal_url".into(), view::internal_url(spec.db_kind.as_deref(), &spec.safe_name(), spec.port)),
            ("svc_db_kind".into(), spec.db_kind.clone().unwrap_or_else(|| "—".into())),
            ("svc_hc".into(), view::healthcheck_summary(&spec.healthcheck)),
            ("svc_run_command".into(), spec.run_command.clone().unwrap_or_else(|| "—".into())),
            ("svc_run_args".into(), run_args),
            ("svc_env".into(), view::env_json_with_comments(&spec.env_vars, &spec.env_comments)),
            ("svc_env_count".into(), spec.env_vars.len().to_string()),
            ("svc_env_text".into(), view::env_dotenv_with_comments(&spec.env_vars, &spec.env_comments)),
            // Pristine copy so the editor's Cancel can discard edits offline.
            ("svc_env_text_orig".into(), view::env_dotenv_with_comments(&spec.env_vars, &spec.env_comments)),
            ("svc_logs".into(), view::logs_json(&logs)),
            ("svc_logs_count".into(), logs.len().to_string()),
            ("svc_logs_text".into(), view::join_log_lines(logs.iter().map(|e| (&e.timestamp, e.line.as_str())))),
            ("svc_deployments".into(), view::deployments_detail_json(&deployments)),
            ("svc_deployments_count".into(), deployments.len().to_string()),
            // Editable fields. The domain add-form starts blank (f_domain/f_port/
            // f_tls); f_host_port keeps the current raw-TCP port for its own form.
            ("f_domain".into(), String::new()),
            ("f_port".into(), String::new()),
            ("f_tls".into(), "false".into()),
            ("f_host_port".into(), spec.host_port.map(|p| p.to_string()).unwrap_or_default()),
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

    /// Cria um serviço a partir do wizard "Novo serviço". No sucesso volta para
    /// a lista de serviços do projeto (a `view` é patchada junto) já
    /// re-fetchada; no erro permanece no wizard com a mensagem em `ns_msg`.
    pub async fn create(self, spec: shared::ServiceSpec) -> EffectOutcome {
        let project_id = spec.project_id.clone();
        let name = spec.name.clone();
        match self.client.rpc(Command::ServiceCreate(spec)).await {
            Ok(Response::Service(_)) => {
                let mut pairs = super::projects::Projects::new(self.client.clone())
                    .fetch_services(project_id)
                    .await;
                pairs.push(("view".into(), "project_services".into()));
                pairs.push(("proj_tab".into(), "services".into()));
                pairs.push(("ns_step".into(), String::new()));
                pairs.push(("ns_msg".into(), String::new()));
                EffectOutcome::data(pairs).with_toast(ToastSpec::success(format!("Serviço \"{name}\" criado.")))
            }
            Ok(other) => {
                let msg = view::resp_msg(&other);
                EffectOutcome::data(vec![("ns_msg".into(), msg.clone())]).with_toast(ToastSpec::error(msg))
            }
            Err(e) => {
                let msg = format!("erro: {e}");
                EffectOutcome::data(vec![("ns_msg".into(), msg.clone())]).with_toast(ToastSpec::error(msg))
            }
        }
    }

    /// Deletes a deployment's history record and its build logs (Deployments
    /// tab, "Remover"). The daemon refuses non-terminal deployments on its own
    /// (`DEPLOY_ACTIVE`), so no client-side re-validation beyond hiding the
    /// button (`can_delete`) is needed. Refetches the service detail
    /// afterwards so `svc_deployments` drops the removed row.
    pub async fn delete_deployment(self, service_id: String, deployment_id: String) -> EffectOutcome {
        let msg = match self.client.rpc(Command::DeployDelete { deployment_id }).await {
            Ok(Response::Ok) => "deployment removido".to_string(),
            Ok(other) => view::resp_msg(&other),
            Err(e) => format!("erro: {e}"),
        };
        let mut pairs = self.fetch_detail(service_id).await;
        pairs.push(("svc_action_msg".into(), msg.clone()));
        outcome_toast(pairs, &msg)
    }

    /// Fetches the build log of a single deployment for the Deployments tab.
    pub async fn fetch_build_logs(self, deployment_id: String) -> EffectOutcome {
        let lines = match self.client.rpc(Command::GetBuildLogs { deployment_id: deployment_id.clone() }).await {
            Ok(Response::BuildLogs(l)) => l,
            _ => Vec::new(),
        };
        EffectOutcome::data(vec![
            ("dep_selected".into(), deployment_id),
            ("dep_build_logs".into(), view::build_logs_json(&lines)),
            ("dep_build_count".into(), lines.len().to_string()),
            ("dep_build_text".into(), view::join_log_lines(lines.iter().map(|e| (&e.timestamp, e.line.as_str())))),
        ])
    }

    /// Applies a [`SpecOp`] to the service (fetch fresh spec → mutate →
    /// update), then re-fetches the detail so the panel reflects the change.
    pub async fn run_spec_op(self, service_id: String, op: SpecOp) -> EffectOutcome {
        let msg = match self.apply_spec_op(&service_id, op).await {
            Ok(_) => "salvo".to_string(),
            Err(e) => format!("erro: {e}"),
        };
        let mut pairs = self.fetch_detail(service_id).await;
        pairs.push(("svc_action_msg".into(), msg.clone()));
        outcome_toast(pairs, &msg)
    }

    async fn apply_spec_op(&self, service_id: &str, op: SpecOp) -> anyhow::Result<()> {
        let client = &self.client;
        let svc = match client.rpc(Command::ServiceGet { id: service_id.into() }).await? {
            Response::Service(s) => s,
            other => anyhow::bail!("resposta inesperada para ServiceGet: {other:?}"),
        };
        let mut spec = svc.spec;
        let trimmed = |s: String| {
            let t = s.trim().to_string();
            if t.is_empty() { None } else { Some(t) }
        };
        match op {
            SpecOp::DomainAdd { domain, port, tls } => {
                let Some(domain) = trimmed(domain) else {
                    anyhow::bail!("domínio vazio");
                };
                // Move o domínio legado para a lista antes de mexer, para operar numa
                // única fonte de verdade.
                spec.materialize_domains();
                let route = shared::DomainRoute {
                    domain: domain.clone(),
                    port: port.trim().parse::<u16>().ok(),
                    tls,
                };
                // Upsert por domínio: substitui se já existir, senão adiciona.
                if let Some(existing) = spec.domains.iter_mut().find(|r| r.domain == domain) {
                    *existing = route;
                } else {
                    spec.domains.push(route);
                }
            }
            SpecOp::DomainRemove { domain } => {
                spec.materialize_domains();
                spec.domains.retain(|r| r.domain != domain);
            }
            SpecOp::HostPort { host_port } => {
                spec.host_port = host_port.trim().parse::<u16>().ok();
            }
            SpecOp::Compose { content } => {
                // Só faz sentido para serviços já Compose; ignora silenciosamente
                // caso contrário (a UI só expõe o editor para Compose).
                if matches!(spec.source, shared::ServiceSource::Compose(_)) {
                    spec.source = shared::ServiceSource::Compose(shared::ComposeSource { content });
                }
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
                let git = ServiceSource::Git(shared::GitSource {
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
        match client.rpc(Command::ServiceUpdate { id: service_id.into(), spec }).await? {
            Response::Ok | Response::Service(_) => Ok(()),
            Response::Err { code, message } => anyhow::bail!("{code}: {message}"),
            other => anyhow::bail!("resposta inesperada para ServiceUpdate: {other:?}"),
        }
    }

    /// Applies an [`EnvOp`] to the service (fetch fresh spec → mutate →
    /// update), then re-fetches the detail so the panel reflects the change.
    pub async fn run_env_op(self, service_id: String, op: EnvOp) -> EffectOutcome {
        let msg = match self.apply_env_op(&service_id, op).await {
            Ok(_) => "env atualizado".to_string(),
            Err(e) => format!("erro: {e}"),
        };
        let mut pairs = self.fetch_detail(service_id).await;
        pairs.push(("svc_action_msg".into(), msg.clone()));
        outcome_toast(pairs, &msg)
    }

    async fn apply_env_op(&self, service_id: &str, op: EnvOp) -> anyhow::Result<()> {
        let client = &self.client;
        let svc = match client.rpc(Command::ServiceGet { id: service_id.into() }).await? {
            Response::Service(s) => s,
            other => anyhow::bail!("resposta inesperada para ServiceGet: {other:?}"),
        };
        let mut spec = svc.spec;
        match op {
            EnvOp::Set { key, value } => {
                spec.env_vars.retain(|v| v.key != key);
                spec.env_vars.push(EnvVar { key, value: EnvVarValue::Plain(value) });
            }
            EnvOp::Delete { key } => {
                spec.env_vars.retain(|v| v.key != key);
                // A comment about a var that no longer exists is meaningless.
                spec.env_comments.retain(|c| c.before_key.as_deref() != Some(key.as_str()));
            }
            EnvOp::ImportDotenv(text) => {
                let (vars, comments) = parse_dotenv_with_comments(&text);
                spec.env_vars = vars;
                spec.env_comments = comments;
            }
            EnvOp::Reorder(keys) => {
                let vars = std::mem::take(&mut spec.env_vars);
                let comments = std::mem::take(&mut spec.env_comments);
                let (new_vars, new_comments) = reorder_env(vars, comments, &keys);
                spec.env_vars = new_vars;
                spec.env_comments = new_comments;
            }
        }
        match client.rpc(Command::ServiceUpdate { id: service_id.into(), spec }).await? {
            Response::Ok | Response::Service(_) => Ok(()),
            Response::Err { code, message } => anyhow::bail!("{code}: {message}"),
            other => anyhow::bail!("resposta inesperada para ServiceUpdate: {other:?}"),
        }
    }
}

/// Parses a `.env` blob (`KEY=VALUE`, `#` comments, blanks ignored) into its
/// vars and, separately, its `#` comment lines — each anchored to the var
/// that immediately follows it (`before_key`), or `None` for a trailing/orphan
/// comment at the end of the text with no var after it. A value of the form
/// `<secret:NAME>` round-trips back to a secret reference.
fn parse_dotenv_with_comments(text: &str) -> (Vec<EnvVar>, Vec<EnvComment>) {
    let mut vars = Vec::new();
    let mut comments = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        if l.starts_with('#') {
            pending.push(l.to_string());
            continue;
        }
        let Some((k, v)) = l.split_once('=') else { continue };
        let key = k.trim().to_string();
        if key.is_empty() {
            continue;
        }
        for text in pending.drain(..) {
            comments.push(EnvComment { text, before_key: Some(key.clone()) });
        }
        let v = v.trim();
        let value = match v.strip_prefix("<secret:").and_then(|s| s.strip_suffix('>')) {
            Some(name) => EnvVarValue::Secret(name.to_string()),
            None => EnvVarValue::Plain(v.to_string()),
        };
        vars.push(EnvVar { key, value });
    }
    for text in pending {
        comments.push(EnvComment { text, before_key: None });
    }
    (vars, comments)
}

/// Applies a drag-and-drop `keys` order (var keys + `__c<idx>` comment rows,
/// see `env_json_with_comments`) to `vars`/`comments`: vars are reordered to
/// match, and each comment is re-anchored to the first real var that follows
/// it in the new order (`before_key: None` when it lands after every var).
/// Items not mentioned in `keys` are kept — vars appended at the end, comments
/// with their original anchor — a defensive fallback, since `keys` should
/// always cover every row.
fn reorder_env(
    vars: Vec<EnvVar>,
    comments: Vec<EnvComment>,
    keys: &[String],
) -> (Vec<EnvVar>, Vec<EnvComment>) {
    let var_keys: HashSet<String> = vars.iter().map(|v| v.key.clone()).collect();
    let mut by_key: HashMap<String, EnvVar> =
        vars.into_iter().map(|v| (v.key.clone(), v)).collect();
    let mut seen_comment = vec![false; comments.len()];
    let mut new_vars: Vec<EnvVar> = Vec::new();
    let mut new_comments: Vec<EnvComment> = Vec::new();
    for (pos, k) in keys.iter().enumerate() {
        if let Some(ci) = k.strip_prefix("__c").and_then(|s| s.parse::<usize>().ok()) {
            // Linha de comentário: a nova âncora é a primeira var real que
            // vem depois dela na nova ordem.
            if let Some(c) = comments.get(ci) {
                seen_comment[ci] = true;
                let anchor = keys[pos + 1..]
                    .iter()
                    .find(|kk| var_keys.contains(kk.as_str()))
                    .cloned();
                new_comments.push(EnvComment { text: c.text.clone(), before_key: anchor });
            }
        } else if let Some(v) = by_key.remove(k) {
            new_vars.push(v);
        }
    }
    new_vars.extend(by_key.into_values());
    for (ci, c) in comments.into_iter().enumerate() {
        if !seen_comment[ci] {
            new_comments.push(c);
        }
    }
    (new_vars, new_comments)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(k: &str) -> EnvVar {
        EnvVar { key: k.into(), value: EnvVarValue::Plain(format!("v_{k}")) }
    }
    fn comment(text: &str, before: Option<&str>) -> EnvComment {
        EnvComment { text: text.into(), before_key: before.map(String::from) }
    }
    fn keys(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// Comentário final (sem âncora) arrastado para cima da única var
    /// reancora nela — e o JSON renderizado o mostra acima da var.
    #[test]
    fn reorder_comment_above_var_reanchors() {
        let (vars, comments) = reorder_env(
            vec![var("teste")],
            vec![comment("# ola", None)],
            &keys(&["__c0", "teste"]),
        );
        assert_eq!(vars.len(), 1);
        assert_eq!(comments[0].before_key.as_deref(), Some("teste"));
        let rows: Vec<serde_json::Value> =
            serde_json::from_str(&view::env_json_with_comments(&vars, &comments)).unwrap();
        assert_eq!(rows[0]["kind"], "comment");
        assert_eq!(rows[1]["key"], "teste");
    }

    /// Comentário arrastado para depois de todas as vars vira comentário
    /// de fim de arquivo.
    #[test]
    fn reorder_comment_to_end_becomes_trailing() {
        let (_, comments) = reorder_env(
            vec![var("a"), var("b")],
            vec![comment("# c", Some("a"))],
            &keys(&["a", "b", "__c0"]),
        );
        assert_eq!(comments[0].before_key, None);
    }

    /// Reordenar vars por cima de um comentário reancora o comentário na
    /// var que ficou depois dele.
    #[test]
    fn reorder_vars_reanchors_comment_between_them() {
        let (vars, comments) = reorder_env(
            vec![var("a"), var("b")],
            vec![comment("# c", Some("a"))],
            &keys(&["b", "__c0", "a"]),
        );
        assert_eq!(vars[0].key, "b");
        assert_eq!(vars[1].key, "a");
        assert_eq!(comments[0].before_key.as_deref(), Some("a"));
    }
}
