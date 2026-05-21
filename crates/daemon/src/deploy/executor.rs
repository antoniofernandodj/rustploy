use crate::{
    db::Db,
    docker::{containers, images, networks, DockerClient},
    event_bus::EventBus,
    ingress::IngressController,
    secrets::SecretsManager,
};
use anyhow::{anyhow, Result};
use bollard::models::HealthStatusEnum;
use chrono::Utc;
use shared::{
    DeployState, Deployment, EnvVarValue, Event, GitSource, HealthcheckKind, Service,
    ServiceSource, ServiceStatus,
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{error, info, warn};

pub struct DeployExecutor {
    pub db: Arc<Db>,
    pub docker: Arc<DockerClient>,
    pub ingress: Arc<IngressController>,
    pub bus: Arc<EventBus>,
    pub secrets: Arc<SecretsManager>,
    pub db_path: PathBuf,
    pub drain_secs: u64,
}

impl DeployExecutor {
    pub async fn run(self: Arc<Self>, deployment_id: String) {
        if let Err(e) = self.execute(&deployment_id).await {
            error!(deployment_id, error = %e, "deploy executor failed");
        }
    }

    async fn execute(&self, deployment_id: &str) -> Result<()> {
        loop {
            let deployment = self.load_deployment(deployment_id).await?;
            if deployment.state.is_terminal() {
                break;
            }

            let service = self
                .load_service(&deployment.service_id)
                .await?
                .ok_or_else(|| anyhow!("service not found: {}", deployment.service_id))?;

            let result = self.step(&deployment, &service).await;

            match result {
                Ok(next_state) => {
                    self.transition(deployment_id, &deployment.state, next_state, None)
                        .await?;
                }
                Err(e) => {
                    warn!(deployment_id, error = %e, "step failed, rolling back");
                    self.transition(
                        deployment_id,
                        &deployment.state,
                        DeployState::RollingBack,
                        Some(e.to_string()),
                    )
                    .await?;
                }
            }
        }
        Ok(())
    }

    async fn step(&self, dep: &Deployment, svc: &Service) -> Result<DeployState> {
        match &dep.state {
            DeployState::Pending => {
                self.ensure_network(&svc.spec.project_id).await?;
                Ok(DeployState::ResolvingDeps)
            }

            DeployState::ResolvingDeps => match &svc.spec.source {
                ServiceSource::Registry { .. } => Ok(DeployState::PullingImage),
                ServiceSource::Git(_) => Ok(DeployState::CloningRepo),
            },

            DeployState::PullingImage => {
                let image = self.image_for(dep, svc);
                images::pull(&self.docker.inner, &image, &svc.id, &dep.id, &self.bus).await?;
                Ok(DeployState::Staging)
            }

            DeployState::CloningRepo => {
                let ServiceSource::Git(git) = &svc.spec.source else {
                    return Err(anyhow!("expected Git source"));
                };
                // Resolve token before entering spawn_blocking (async op)
                let token = if let Some(name) = &git.credentials {
                    self.secrets.get_raw(name).await.ok()
                } else {
                    None
                };
                let dir = self.clone_dir(&dep.id);
                let git_clone = git.clone();
                let bus = self.bus.clone();
                let sid = svc.id.clone();
                let did = dep.id.clone();

                // git2 is !Send, run in blocking thread
                tokio::task::spawn_blocking(move || {
                    clone_repo_sync(&dir, &git_clone, token.as_deref(), &bus, &sid, &did)
                })
                .await??;

                Ok(DeployState::BuildingImage)
            }

            DeployState::BuildingImage => {
                let ServiceSource::Git(git) = &svc.spec.source else {
                    return Err(anyhow!("expected Git source"));
                };
                let tag = format!("rp_{}:{}", svc.spec.name, self.short(&dep.id));
                let clone_dir = self.clone_dir(&dep.id);
                let context = clone_dir.join(&git.build_context);
                images::build(
                    &self.docker.inner,
                    &context,
                    &git.dockerfile_path,
                    &tag,
                    &svc.id,
                    &dep.id,
                    &self.bus,
                )
                .await?;
                Ok(DeployState::Staging)
            }

            DeployState::Staging => {
                let image = self.image_for(dep, svc);
                let network = self.network_name(&svc.spec.project_id);
                let env = self.resolve_env(svc).await?;
                let id = containers::create_staging(
                    &self.docker.inner,
                    &svc.spec,
                    &image,
                    &svc.id,
                    &dep.id,
                    &network,
                    &env,
                )
                .await?;
                containers::start(&self.docker.inner, &id).await?;
                Ok(DeployState::HealthcheckPolling)
            }

            DeployState::HealthcheckPolling => {
                let staging = containers::staging_name(&svc.spec.name, self.short(&dep.id));
                let cid = containers::find_by_name(&self.docker.inner, &staging)
                    .await?
                    .ok_or_else(|| anyhow!("staging container not found"))?;
                let net = self.network_name(&svc.spec.project_id);
                let ip = containers::get_container_ip(&self.docker.inner, &cid, &net).await?;
                self.poll_healthcheck(&ip, &cid, svc, dep).await?;
                Ok(DeployState::SwappingIn)
            }

            DeployState::SwappingIn => {
                let staging = containers::staging_name(&svc.spec.name, self.short(&dep.id));
                let staging_id = containers::find_by_name(&self.docker.inner, &staging)
                    .await?
                    .ok_or_else(|| anyhow!("staging container not found"))?;
                let net = self.network_name(&svc.spec.project_id);
                let ip =
                    containers::get_container_ip(&self.docker.inner, &staging_id, &net).await?;
                self.ingress
                    .upsert_route(&svc.spec.domain, &format!("{ip}:{}", svc.spec.port), &svc.id);

                let live = containers::live_name(&svc.spec.name);
                if let Ok(Some(old)) = containers::find_by_name(&self.docker.inner, &live).await {
                    let _ = containers::stop_graceful(&self.docker.inner, &old, 30).await;
                }
                Ok(DeployState::Draining)
            }

            DeployState::Draining => {
                sleep(Duration::from_secs(self.drain_secs)).await;
                Ok(DeployState::Promoting)
            }

            DeployState::Promoting => {
                let staging = containers::staging_name(&svc.spec.name, self.short(&dep.id));
                let staging_id = containers::find_by_name(&self.docker.inner, &staging)
                    .await?
                    .ok_or_else(|| anyhow!("staging container not found"))?;
                let live = containers::live_name(&svc.spec.name);

                if let Ok(Some(old)) = containers::find_by_name(&self.docker.inner, &live).await {
                    let _ = containers::remove(&self.docker.inner, &old).await;
                }
                containers::rename(&self.docker.inner, &staging_id, &live).await?;

                crate::db::services::update_status(
                    &self.db,
                    &svc.id,
                    &ServiceStatus::Running,
                    Some(&staging_id),
                )
                .await?;
                self.bus.publish(Event::ServiceStatusChanged {
                    service_id: svc.id.clone(),
                    status: ServiceStatus::Running,
                });

                let _ = std::fs::remove_dir_all(self.clone_dir(&dep.id));
                Ok(DeployState::Live)
            }

            DeployState::RollingBack => {
                let staging = containers::staging_name(&svc.spec.name, self.short(&dep.id));
                if let Ok(Some(id)) = containers::find_by_name(&self.docker.inner, &staging).await {
                    let _ = containers::remove(&self.docker.inner, &id).await;
                }

                let live = containers::live_name(&svc.spec.name);
                if let Ok(Some(old)) = containers::find_by_name(&self.docker.inner, &live).await {
                    let net = self.network_name(&svc.spec.project_id);
                    if let Ok(ip) =
                        containers::get_container_ip(&self.docker.inner, &old, &net).await
                    {
                        self.ingress.upsert_route(
                            &svc.spec.domain,
                            &format!("{ip}:{}", svc.spec.port),
                            &svc.id,
                        );
                    }
                }

                crate::db::services::update_status(
                    &self.db,
                    &svc.id,
                    &ServiceStatus::Error("deploy failed".into()),
                    None,
                )
                .await?;
                self.bus.publish(Event::ServiceStatusChanged {
                    service_id: svc.id.clone(),
                    status: ServiceStatus::Error("deploy failed".into()),
                });
                let _ = std::fs::remove_dir_all(self.clone_dir(&dep.id));
                Ok(DeployState::Failed)
            }

            other => Err(anyhow!("unhandled state: {:?}", other)),
        }
    }

    async fn poll_healthcheck(
        &self,
        ip: &str,
        container_id: &str,
        svc: &Service,
        dep: &Deployment,
    ) -> Result<()> {
        let hc = &svc.spec.healthcheck;
        sleep(Duration::from_secs(hc.start_period_secs as u64)).await;

        let interval = Duration::from_secs(hc.interval_secs as u64);
        let timeout = Duration::from_secs(hc.timeout_secs as u64);
        let max = hc.retries;

        for attempt in 0..max {
            let inspect = containers::inspect(&self.docker.inner, container_id).await?;
            if !inspect
                .state
                .as_ref()
                .and_then(|s| s.running)
                .unwrap_or(false)
            {
                return Err(anyhow!("container stopped during healthcheck"));
            }

            let ok = match &hc.kind {
                HealthcheckKind::Http { path, expected_status } => {
                    check_http(&format!("http://{ip}:{}{path}", svc.spec.port), *expected_status, timeout).await
                }
                HealthcheckKind::Tcp => {
                    check_tcp(&format!("{ip}:{}", svc.spec.port), timeout).await
                }
                HealthcheckKind::DockerNative => inspect
                    .state
                    .as_ref()
                    .and_then(|s| s.health.as_ref())
                    .and_then(|h| h.status.as_ref())
                    == Some(&HealthStatusEnum::HEALTHY),
            };

            if ok {
                info!(attempt, "healthcheck passed");
                return Ok(());
            }

            self.bus.publish(Event::DeployProgress {
                deployment_id: dep.id.clone(),
                service_id: svc.id.clone(),
                phase: "HealthcheckPolling".into(),
                percent: ((attempt as f32 / max as f32) * 100.0) as u8,
                description: format!("attempt {}/{max}", attempt + 1),
            });

            sleep(interval).await;
        }

        Err(anyhow!("healthcheck failed after {max} retries"))
    }

    fn clone_dir(&self, deployment_id: &str) -> PathBuf {
        self.db_path.join("builds").join(deployment_id)
    }

    fn short<'a>(&self, id: &'a str) -> &'a str {
        &id[..8.min(id.len())]
    }

    fn image_for(&self, dep: &Deployment, svc: &Service) -> String {
        match &svc.spec.source {
            ServiceSource::Registry { image } => image.clone(),
            ServiceSource::Git(_) => format!("rp_{}:{}", svc.spec.name, self.short(&dep.id)),
        }
    }

    fn network_name(&self, project_id: &str) -> String {
        networks::project_network_name(&project_id[..8.min(project_id.len())])
    }

    async fn ensure_network(&self, project_id: &str) -> Result<String> {
        networks::ensure_project_network(&self.docker.inner, project_id).await
    }

    async fn resolve_env(&self, svc: &Service) -> Result<Vec<(String, String)>> {
        let mut out = Vec::new();
        for ev in &svc.spec.env_vars {
            let value = match &ev.value {
                EnvVarValue::Plain(v) => v.clone(),
                EnvVarValue::Secret(name) => self.secrets.get_raw(name).await.unwrap_or_default(),
            };
            out.push((ev.key.clone(), value));
        }
        Ok(out)
    }

    async fn load_deployment(&self, id: &str) -> Result<Deployment> {
        crate::db::deployments::get(&self.db, id)
            .await?
            .ok_or_else(|| anyhow!("deployment not found: {id}"))
    }

    async fn load_service(&self, id: &str) -> Result<Option<Service>> {
        crate::db::services::get(&self.db, id).await
    }

    async fn transition(
        &self,
        deployment_id: &str,
        from: &DeployState,
        to: DeployState,
        message: Option<String>,
    ) -> Result<()> {
        info!(deployment_id, from = from.label(), to = to.label(), "state transition");
        let dep = crate::db::deployments::transition(
            &self.db,
            deployment_id,
            from,
            to.clone(),
            message.clone(),
        )
        .await?;

        self.bus.publish(Event::DeployStateChanged {
            deployment_id: deployment_id.to_string(),
            service_id: dep.service_id.clone(),
            state: to,
            timestamp: Utc::now(),
            message,
        });
        Ok(())
    }
}

/// Synchronous git clone — runs in spawn_blocking because git2 is !Send.
fn clone_repo_sync(
    dir: &PathBuf,
    git: &GitSource,
    token: Option<&str>,
    bus: &Arc<EventBus>,
    service_id: &str,
    deployment_id: &str,
) -> Result<()> {
    use git2::{build::RepoBuilder, FetchOptions, RemoteCallbacks};

    std::fs::create_dir_all(dir)?;
    info!(url = git.url, branch = git.branch, "cloning repository");

    let mut callbacks = RemoteCallbacks::new();
    if let Some(tok) = token {
        let tok = tok.to_string();
        callbacks.credentials(move |_url, username, _allowed| {
            git2::Cred::userpass_plaintext(username.unwrap_or("git"), &tok)
        });
    }

    let sid = service_id.to_string();
    let did = deployment_id.to_string();
    let bus_clone = bus.clone();
    callbacks.transfer_progress(move |stats| {
        let pct = if stats.total_objects() > 0 {
            (stats.received_objects() * 100 / stats.total_objects()) as u8
        } else {
            0
        };
        bus_clone.publish(Event::DeployProgress {
            deployment_id: did.clone(),
            service_id: sid.clone(),
            phase: "CloningRepo".into(),
            percent: pct,
            description: format!(
                "objects: {}/{}",
                stats.received_objects(),
                stats.total_objects()
            ),
        });
        true
    });

    let mut fo = FetchOptions::new();
    fo.remote_callbacks(callbacks);

    let mut builder = RepoBuilder::new();
    builder.branch(&git.branch);
    builder.fetch_options(fo);
    builder.clone(&git.url, dir)?;

    info!("repository cloned");
    Ok(())
}

async fn check_http(url: &str, expected: u16, timeout: Duration) -> bool {
    let Ok(client) = reqwest::Client::builder()
        .timeout(timeout)
        .danger_accept_invalid_certs(true)
        .build()
    else {
        return false;
    };
    match client.get(url).send().await {
        Ok(r) => r.status().as_u16() == expected,
        Err(_) => false,
    }
}

async fn check_tcp(addr: &str, timeout: Duration) -> bool {
    tokio::time::timeout(timeout, tokio::net::TcpStream::connect(addr))
        .await
        .map(|r| r.is_ok())
        .unwrap_or(false)
}
