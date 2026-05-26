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
use tracing::{debug, error, info, warn};

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
        info!(deployment_id = %deployment_id, "executor: iniciando");
        if let Err(e) = self.execute(&deployment_id).await {
            error!(deployment_id = %deployment_id, error = %e, "executor: falha fatal no deploy");
            // Publica o erro para o cliente ver (erros no próprio loop de controle,
            // ex.: falha ao ler do banco — distintos das falhas de step que já fazem rollback)
            self.bus.publish(Event::Error {
                code: "ExecutorFatal".into(),
                message: format!("Falha crítica no deploy {}: {e}", &deployment_id[..8.min(deployment_id.len())]),
            });
        }
        info!(deployment_id = %deployment_id, "executor: encerrado");
    }

    async fn execute(&self, deployment_id: &str) -> Result<()> {
        loop {
            let deployment = self.load_deployment(deployment_id).await?;
            info!(
                deployment_id = %deployment_id,
                state = deployment.state.label(),
                "executor: estado atual"
            );

            if deployment.state.is_terminal() {
                info!(
                    deployment_id = %deployment_id,
                    state = deployment.state.label(),
                    "executor: estado terminal, saindo do loop"
                );
                break;
            }

            let service = self
                .load_service(&deployment.service_id)
                .await?
                .ok_or_else(|| anyhow!("service not found: {}", deployment.service_id))?;

            info!(
                deployment_id = %deployment_id,
                service_id = %service.id,
                service_name = %service.spec.name,
                state = deployment.state.label(),
                "executor: executando step"
            );

            let result = self.step(&deployment, &service).await;

            match result {
                Ok(next_state) => {
                    info!(
                        deployment_id = %deployment_id,
                        from = deployment.state.label(),
                        to = next_state.label(),
                        "executor: transição de estado"
                    );
                    self.transition(deployment_id, &deployment.state, next_state, None)
                        .await?;
                }
                Err(e) => {
                    warn!(
                        deployment_id = %deployment_id,
                        state = deployment.state.label(),
                        error = %e,
                        "executor: step falhou, iniciando rollback"
                    );
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
                info!(
                    deployment_id = %dep.id,
                    project_id = %svc.spec.project_id,
                    "step[Pending]: garantindo rede Docker do projeto"
                );
                let net = self.ensure_network(&svc.spec.project_id).await?;
                info!(
                    deployment_id = %dep.id,
                    network = %net,
                    "step[Pending]: rede pronta"
                );
                Ok(DeployState::ResolvingDeps)
            }

            DeployState::ResolvingDeps => {
                let next = match &svc.spec.source {
                    ServiceSource::Registry { image } => {
                        info!(
                            deployment_id = %dep.id,
                            image = %image,
                            "step[ResolvingDeps]: fonte é Registry → irá para PullingImage"
                        );
                        DeployState::PullingImage
                    }
                    ServiceSource::Git(g) => {
                        info!(
                            deployment_id = %dep.id,
                            url = %g.url,
                            branch = %g.branch,
                            "step[ResolvingDeps]: fonte é Git → irá para CloningRepo"
                        );
                        DeployState::CloningRepo
                    }
                };
                Ok(next)
            }

            DeployState::PullingImage => {
                let image = self.image_for(dep, svc);
                info!(
                    deployment_id = %dep.id,
                    image = %image,
                    "step[PullingImage]: iniciando pull"
                );
                images::pull(&self.docker.inner, &image, &svc.id, &dep.id, &self.bus).await?;
                info!(
                    deployment_id = %dep.id,
                    image = %image,
                    "step[PullingImage]: pull concluído"
                );
                Ok(DeployState::Staging)
            }

            DeployState::CloningRepo => {
                let ServiceSource::Git(git) = &svc.spec.source else {
                    return Err(anyhow!("expected Git source"));
                };
                info!(
                    deployment_id = %dep.id,
                    url = %git.url,
                    branch = %git.branch,
                    "step[CloningRepo]: resolvendo credenciais"
                );
                let token = if let Some(name) = &git.credentials {
                    info!(deployment_id = %dep.id, secret = %name, "step[CloningRepo]: buscando token do secret");
                    self.secrets.get_raw(name).await.ok()
                } else {
                    info!(deployment_id = %dep.id, "step[CloningRepo]: sem credenciais configuradas");
                    None
                };
                let dir = self.clone_dir(&dep.id);
                info!(
                    deployment_id = %dep.id,
                    dir = %dir.display(),
                    "step[CloningRepo]: clonando para diretório"
                );
                let git_clone = git.clone();
                let bus = self.bus.clone();
                let sid = svc.id.clone();
                let did = dep.id.clone();

                tokio::task::spawn_blocking(move || {
                    clone_repo_sync(&dir, &git_clone, token.as_deref(), &bus, &sid, &did)
                })
                .await??;

                info!(deployment_id = %dep.id, "step[CloningRepo]: clone concluído");
                Ok(DeployState::BuildingImage)
            }

            DeployState::BuildingImage => {
                let ServiceSource::Git(git) = &svc.spec.source else {
                    return Err(anyhow!("expected Git source"));
                };
                let tag = format!("rp_{}:{}", svc.spec.name, self.short(&dep.id));
                let clone_dir = self.clone_dir(&dep.id);
                let context = clone_dir.join(&git.build_context);
                info!(
                    deployment_id = %dep.id,
                    tag = %tag,
                    dockerfile = %git.dockerfile_path,
                    context = %context.display(),
                    "step[BuildingImage]: iniciando build Docker"
                );
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
                info!(
                    deployment_id = %dep.id,
                    tag = %tag,
                    "step[BuildingImage]: build concluído"
                );
                Ok(DeployState::Staging)
            }

            DeployState::Staging => {
                let image = self.image_for(dep, svc);
                let network = self.network_name(&svc.spec.project_id);
                info!(
                    deployment_id = %dep.id,
                    service_name = %svc.spec.name,
                    image = %image,
                    network = %network,
                    "step[Staging]: resolvendo env vars"
                );
                let env = self.resolve_env(svc).await?;
                info!(
                    deployment_id = %dep.id,
                    env_keys = ?env.iter().map(|(k, _)| k.as_str()).collect::<Vec<_>>(),
                    "step[Staging]: env vars resolvidas"
                );
                info!(
                    deployment_id = %dep.id,
                    "step[Staging]: criando container de staging"
                );
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
                info!(
                    deployment_id = %dep.id,
                    container_id = %id,
                    network = %network,
                    "step[Staging]: conectando container à rede do projeto"
                );
                // Conecta explicitamente via `network connect` ANTES do start.
                // Usar network_mode=rede_user_defined no create não preenche
                // IPAddress no inspect; o flow create→connect→start é confiável.
                networks::connect_container(&self.docker.inner, &network, &id).await?;
                info!(
                    deployment_id = %dep.id,
                    container_id = %id,
                    "step[Staging]: container conectado, dando start"
                );
                containers::start(&self.docker.inner, &id).await?;
                info!(
                    deployment_id = %dep.id,
                    container_id = %id,
                    "step[Staging]: container iniciado"
                );
                Ok(DeployState::HealthcheckPolling)
            }

            DeployState::HealthcheckPolling => {
                let staging = containers::staging_name(&svc.spec.name, self.short(&dep.id));
                info!(
                    deployment_id = %dep.id,
                    container_name = %staging,
                    "step[HealthcheckPolling]: buscando container de staging"
                );
                let cid = containers::find_by_name(&self.docker.inner, &staging)
                    .await?
                    .ok_or_else(|| anyhow!("staging container not found"))?;
                let net = self.network_name(&svc.spec.project_id);
                info!(
                    deployment_id = %dep.id,
                    container_id = %cid,
                    network = %net,
                    "step[HealthcheckPolling]: obtendo IP do container"
                );
                let ip = containers::get_container_ip(&self.docker.inner, &cid, &net).await?;
                info!(
                    deployment_id = %dep.id,
                    container_id = %cid,
                    ip = %ip,
                    port = svc.spec.port,
                    healthcheck = ?svc.spec.healthcheck.kind,
                    "step[HealthcheckPolling]: iniciando polling de healthcheck"
                );
                self.poll_healthcheck(&ip, &cid, svc, dep).await?;
                info!(
                    deployment_id = %dep.id,
                    ip = %ip,
                    "step[HealthcheckPolling]: healthcheck passou"
                );
                Ok(DeployState::SwappingIn)
            }

            DeployState::SwappingIn => {
                let staging = containers::staging_name(&svc.spec.name, self.short(&dep.id));
                info!(
                    deployment_id = %dep.id,
                    container_name = %staging,
                    "step[SwappingIn]: buscando container de staging para swap"
                );
                let staging_id = containers::find_by_name(&self.docker.inner, &staging)
                    .await?
                    .ok_or_else(|| anyhow!("staging container not found"))?;
                let net = self.network_name(&svc.spec.project_id);
                let ip =
                    containers::get_container_ip(&self.docker.inner, &staging_id, &net).await?;
                if let Some(domain) = &svc.spec.domain {
                    info!(
                        deployment_id = %dep.id,
                        container_id = %staging_id,
                        ip = %ip,
                        domain = %domain,
                        upstream = format!("{ip}:{}", svc.spec.port),
                        "step[SwappingIn]: atualizando rota no ingress"
                    );
                    self.ingress
                        .upsert_route(domain, &format!("{ip}:{}", svc.spec.port), &svc.id);
                } else {
                    info!(
                        deployment_id = %dep.id,
                        container_id = %staging_id,
                        ip = %ip,
                        "step[SwappingIn]: sem domínio configurado, ingress não atualizado"
                    );
                }

                let live = containers::live_name(&svc.spec.name);
                match containers::find_by_name(&self.docker.inner, &live).await {
                    Ok(Some(old)) => {
                        info!(
                            deployment_id = %dep.id,
                            old_container = %old,
                            "step[SwappingIn]: parando container live antigo"
                        );
                        let _ = containers::stop_graceful(&self.docker.inner, &old, 30).await;
                        info!(deployment_id = %dep.id, "step[SwappingIn]: container antigo parado");
                    }
                    Ok(None) => {
                        info!(deployment_id = %dep.id, "step[SwappingIn]: nenhum container live anterior");
                    }
                    Err(e) => {
                        warn!(deployment_id = %dep.id, error = %e, "step[SwappingIn]: erro ao buscar live (ignorado)");
                    }
                }
                Ok(DeployState::Draining)
            }

            DeployState::Draining => {
                info!(
                    deployment_id = %dep.id,
                    drain_secs = self.drain_secs,
                    "step[Draining]: aguardando drain de conexões"
                );
                sleep(Duration::from_secs(self.drain_secs)).await;
                info!(deployment_id = %dep.id, "step[Draining]: drain concluído");
                Ok(DeployState::Promoting)
            }

            DeployState::Promoting => {
                let staging = containers::staging_name(&svc.spec.name, self.short(&dep.id));
                let live = containers::live_name(&svc.spec.name);
                info!(
                    deployment_id = %dep.id,
                    staging_name = %staging,
                    live_name = %live,
                    "step[Promoting]: promovendo staging → live"
                );
                let staging_id = containers::find_by_name(&self.docker.inner, &staging)
                    .await?
                    .ok_or_else(|| anyhow!("staging container not found"))?;

                if let Ok(Some(old)) = containers::find_by_name(&self.docker.inner, &live).await {
                    info!(
                        deployment_id = %dep.id,
                        old_container = %old,
                        "step[Promoting]: removendo container live antigo"
                    );
                    let _ = containers::remove(&self.docker.inner, &old).await;
                    info!(deployment_id = %dep.id, "step[Promoting]: container antigo removido");
                }

                info!(
                    deployment_id = %dep.id,
                    container_id = %staging_id,
                    new_name = %live,
                    "step[Promoting]: renomeando staging → live"
                );
                containers::rename(&self.docker.inner, &staging_id, &live).await?;
                info!(deployment_id = %dep.id, live_name = %live, "step[Promoting]: rename concluído");

                info!(
                    deployment_id = %dep.id,
                    service_id = %svc.id,
                    container_id = %staging_id,
                    "step[Promoting]: atualizando status do serviço para Running"
                );
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
                info!(
                    deployment_id = %dep.id,
                    service_id = %svc.id,
                    "step[Promoting]: serviço promovido para Running ✓"
                );

                let build_dir = self.clone_dir(&dep.id);
                if build_dir.exists() {
                    let _ = std::fs::remove_dir_all(&build_dir);
                    debug!(deployment_id = %dep.id, dir = %build_dir.display(), "step[Promoting]: diretório de build removido");
                }
                Ok(DeployState::Live)
            }

            DeployState::RollingBack => {
                let staging = containers::staging_name(&svc.spec.name, self.short(&dep.id));
                info!(
                    deployment_id = %dep.id,
                    container_name = %staging,
                    "step[RollingBack]: removendo container de staging"
                );
                if let Ok(Some(id)) = containers::find_by_name(&self.docker.inner, &staging).await {
                    let _ = containers::remove(&self.docker.inner, &id).await;
                    info!(deployment_id = %dep.id, container_id = %id, "step[RollingBack]: staging removido");
                } else {
                    info!(deployment_id = %dep.id, "step[RollingBack]: nenhum staging encontrado para remover");
                }

                let live = containers::live_name(&svc.spec.name);
                match containers::find_by_name(&self.docker.inner, &live).await {
                    Ok(Some(old)) => {
                        let net = self.network_name(&svc.spec.project_id);
                        if let Ok(ip) =
                            containers::get_container_ip(&self.docker.inner, &old, &net).await
                        {
                            if let Some(domain) = &svc.spec.domain {
                                info!(
                                    deployment_id = %dep.id,
                                    old_live = %old,
                                    ip = %ip,
                                    "step[RollingBack]: restaurando rota ingress para live anterior"
                                );
                                self.ingress.upsert_route(
                                    domain,
                                    &format!("{ip}:{}", svc.spec.port),
                                    &svc.id,
                                );
                            }
                        }
                    }
                    _ => {
                        info!(deployment_id = %dep.id, "step[RollingBack]: nenhum live anterior para restaurar");
                    }
                }

                let err_status = ServiceStatus::Error("deploy failed".into());
                info!(
                    deployment_id = %dep.id,
                    service_id = %svc.id,
                    "step[RollingBack]: atualizando serviço para Error"
                );
                crate::db::services::update_status(
                    &self.db,
                    &svc.id,
                    &err_status,
                    None,
                )
                .await?;
                self.bus.publish(Event::ServiceStatusChanged {
                    service_id: svc.id.clone(),
                    status: err_status,
                });
                info!(deployment_id = %dep.id, "step[RollingBack]: rollback concluído, estado = Failed");
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
        info!(
            deployment_id = %dep.id,
            kind = ?hc.kind,
            start_period = hc.start_period_secs,
            interval = hc.interval_secs,
            timeout = hc.timeout_secs,
            retries = hc.retries,
            "healthcheck: aguardando start_period antes do primeiro check"
        );
        sleep(Duration::from_secs(hc.start_period_secs as u64)).await;

        let interval = Duration::from_secs(hc.interval_secs as u64);
        let timeout = Duration::from_secs(hc.timeout_secs as u64);
        let max = hc.retries;

        for attempt in 0..max {
            info!(
                deployment_id = %dep.id,
                attempt = attempt + 1,
                max = max,
                "healthcheck: tentativa"
            );

            let inspect = containers::inspect(&self.docker.inner, container_id).await?;
            let running = inspect
                .state
                .as_ref()
                .and_then(|s| s.running)
                .unwrap_or(false);

            if !running {
                let exit_code = inspect
                    .state
                    .as_ref()
                    .and_then(|s| s.exit_code);
                error!(
                    deployment_id = %dep.id,
                    container_id = %container_id,
                    exit_code = ?exit_code,
                    "healthcheck: container parou inesperadamente"
                );
                return Err(anyhow!("container stopped during healthcheck"));
            }

            let ok = match &hc.kind {
                HealthcheckKind::Http { path, expected_status } => {
                    let url = format!("http://{ip}:{}{path}", svc.spec.port);
                    debug!(deployment_id = %dep.id, url = %url, expected = expected_status, "healthcheck: HTTP check");
                    check_http(&url, *expected_status, timeout).await
                }
                HealthcheckKind::Tcp => {
                    let addr = format!("{ip}:{}", svc.spec.port);
                    debug!(deployment_id = %dep.id, addr = %addr, "healthcheck: TCP check");
                    check_tcp(&addr, timeout).await
                }
                HealthcheckKind::DockerNative => {
                    let status = inspect
                        .state
                        .as_ref()
                        .and_then(|s| s.health.as_ref())
                        .and_then(|h| h.status.as_ref());
                    debug!(deployment_id = %dep.id, health_status = ?status, "healthcheck: DockerNative check");
                    // None  → imagem sem HEALTHCHECK; container rodando = ok
                    // HEALTHY → passou
                    // STARTING → ainda aquecendo, aguardar
                    // UNHEALTHY → falha explícita
                    match status {
                        None => true,
                        Some(s) => *s == HealthStatusEnum::HEALTHY,
                    }
                }
            };

            if ok {
                info!(
                    deployment_id = %dep.id,
                    attempt = attempt + 1,
                    "healthcheck: passou ✓"
                );
                return Ok(());
            }

            warn!(
                deployment_id = %dep.id,
                attempt = attempt + 1,
                max = max,
                "healthcheck: falhou nesta tentativa, aguardando próxima"
            );

            self.bus.publish(Event::DeployProgress {
                deployment_id: dep.id.clone(),
                service_id: svc.id.clone(),
                phase: "HealthcheckPolling".into(),
                percent: ((attempt as f32 / max as f32) * 100.0) as u8,
                description: format!("attempt {}/{max}", attempt + 1),
            });

            sleep(interval).await;
        }

        error!(
            deployment_id = %dep.id,
            max = max,
            "healthcheck: esgotou todas as tentativas"
        );
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
        // Env vars do projeto herdadas por todos os serviços (base)
        let project_env = if let Ok(Some(project)) =
            crate::db::projects::get(&self.db, &svc.spec.project_id).await
        {
            project.env_vars
        } else {
            vec![]
        };

        // Mapa com precedência: projeto primeiro, service sobrescreve
        use std::collections::HashMap;
        let mut env_map: HashMap<String, String> = HashMap::new();

        for ev in &project_env {
            let value = match &ev.value {
                EnvVarValue::Plain(v) => v.clone(),
                EnvVarValue::Secret(name) => {
                    debug!(service_id = %svc.id, secret = %name, "resolve_env: desencriptando secret do projeto");
                    self.secrets.get_raw(name).await.unwrap_or_default()
                }
            };
            env_map.insert(ev.key.clone(), value);
        }

        for ev in &svc.spec.env_vars {
            let value = match &ev.value {
                EnvVarValue::Plain(v) => v.clone(),
                EnvVarValue::Secret(name) => {
                    debug!(service_id = %svc.id, secret = %name, "resolve_env: desencriptando secret do serviço");
                    self.secrets.get_raw(name).await.unwrap_or_default()
                }
            };
            // Service override tem precedência sobre o projeto
            env_map.insert(ev.key.clone(), value);
        }

        debug!(
            service_id = %svc.id,
            project_vars = project_env.len(),
            service_vars = svc.spec.env_vars.len(),
            total = env_map.len(),
            "resolve_env: vars resolvidas (projeto + serviço)"
        );

        Ok(env_map.into_iter().collect())
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
        info!(
            deployment_id = %deployment_id,
            from = from.label(),
            to = to.label(),
            message = ?message,
            "executor: gravando transição no banco"
        );
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
            state: to.clone(),
            timestamp: Utc::now(),
            message,
        });
        info!(
            deployment_id = %deployment_id,
            state = to.label(),
            "executor: evento DeployStateChanged publicado"
        );
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

    // Limpa diretório de clone anterior para evitar falha em retry
    // (git2 recusa clonar em diretório não-vazio)
    if dir.exists() {
        std::fs::remove_dir_all(dir)
            .map_err(|e| anyhow::anyhow!("falha ao limpar clone dir anterior '{}': {e}", dir.display()))?;
    }
    std::fs::create_dir_all(dir)?;
    info!(url = git.url, branch = git.branch, dir = %dir.display(), "clone_repo: clonando repositório");

    let mut callbacks = RemoteCallbacks::new();
    if let Some(tok) = token {
        let tok = tok.to_string();
        info!("clone_repo: usando credenciais de token");
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
        debug!(
            deployment_id = %did,
            received = stats.received_objects(),
            total = stats.total_objects(),
            pct = pct,
            "clone_repo: progresso"
        );
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

    // Para URLs file:// o git2 não cria refs/remotes/origin/* de forma confiável
    // (usa clone local por hardlink, sem fetch normal). Convertemos para caminho
    // absoluto para forçar o comportamento de clone local do git2.
    let effective_url = if git.url.starts_with("file://") {
        git.url
            .strip_prefix("file://")
            .unwrap_or(&git.url)
            .to_owned()
    } else {
        git.url.clone()
    };

    // NÃO usar builder.branch(): o git2 resolve o branch via
    // refs/remotes/origin/<branch>, que pode não existir em clones locais ou
    // em remotes que ainda não popularam esse ref. Fazemos o checkout
    // manualmente após o clone.
    let mut builder = RepoBuilder::new();
    builder.fetch_options(fo);
    let repo = builder.clone(&effective_url, dir)
        .map_err(|e| anyhow::anyhow!("falha ao clonar '{}': {e}", git.url))?;

    info!(url = git.url, branch = git.branch, "clone_repo: clone concluído, resolvendo branch");

    // Coleta branches disponíveis para erro útil (ignora falhas de listagem)
    let available_branches: Vec<String> = repo
        .branches(None)
        .map(|iter| {
            iter.filter_map(|b| b.ok())
                .filter_map(|(b, _)| b.name().ok().flatten().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    info!(
        url = git.url,
        branch = git.branch,
        available = ?available_branches,
        "clone_repo: branches disponíveis no clone"
    );

    // Tenta resolver o branch:
    //   1. origin/<branch>  — remote-tracking ref (clones de rede)
    //   2. <branch>         — ref local ou tag (clones locais / file://)
    let obj = repo
        .revparse_single(&format!("origin/{}", git.branch))
        .or_else(|_| repo.revparse_single(&git.branch))
        .map_err(|_| {
            let hint = if available_branches.is_empty() {
                "nenhum branch encontrado no repositório".to_owned()
            } else {
                format!("branches disponíveis: {}", available_branches.join(", "))
            };
            anyhow::anyhow!(
                "branch '{}' não encontrado no repositório — {}",
                git.branch,
                hint
            )
        })?;

    let mut co_opts = git2::build::CheckoutBuilder::new();
    co_opts.force();
    repo.checkout_tree(&obj, Some(&mut co_opts))
        .map_err(|e| anyhow::anyhow!("falha ao fazer checkout do branch '{}': {e}", git.branch))?;

    // Aponta HEAD para o branch local
    let commit = obj.peel_to_commit()
        .map_err(|e| anyhow::anyhow!("falha ao resolver commit do branch '{}': {e}", git.branch))?;
    let head_ref = format!("refs/heads/{}", git.branch);
    repo.reference(&head_ref, commit.id(), true, &format!("checkout: {}", git.branch))
        .map_err(|e| anyhow::anyhow!("falha ao criar ref local '{}': {e}", head_ref))?;
    repo.set_head(&head_ref)
        .map_err(|e| anyhow::anyhow!("falha ao definir HEAD para '{}': {e}", head_ref))?;

    info!(url = git.url, branch = git.branch, commit = %commit.id(), "clone_repo: checkout concluído");
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
        Ok(r) => {
            let got = r.status().as_u16();
            got == expected
        }
        Err(e) => {
            debug!(url = %url, error = %e, "check_http: falhou");
            false
        }
    }
}

async fn check_tcp(addr: &str, timeout: Duration) -> bool {
    match tokio::time::timeout(timeout, tokio::net::TcpStream::connect(addr)).await {
        Ok(Ok(_)) => true,
        Ok(Err(e)) => {
            debug!(addr = %addr, error = %e, "check_tcp: conexão recusada");
            false
        }
        Err(_) => {
            debug!(addr = %addr, "check_tcp: timeout");
            false
        }
    }
}
