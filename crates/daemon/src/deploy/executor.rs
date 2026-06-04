use crate::{
    db::Db,
    docker::{DockerClient, containers, images, networks},
    event_bus::EventBus,
    ingress::IngressController,
    secrets::SecretsManager,
};
use anyhow::{Result, anyhow};
use bollard::models::HealthStatusEnum;
use chrono::Utc;
use shared::{
    DeployState, Deployment, EnvVarValue, Event, HealthcheckKind, Service, ServiceSource,
    ServiceStatus,
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
                message: format!(
                    "Falha crítica no deploy {}: {e}",
                    &deployment_id[..8.min(deployment_id.len())]
                ),
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
                    ServiceSource::Compose(c) => {
                        info!(
                            deployment_id = %dep.id,
                            compose_file = %c.content,
                            "step[ResolvingDeps]: fonte é Compose → irá para ComposingUp"
                        );
                        DeployState::ComposingUp
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
                    self.secrets.get_raw(&svc.spec.project_id, name).await.ok()
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
                let bus = self.bus.clone();
                let sid = svc.id.clone();
                let did = dep.id.clone();

                super::git::clone(
                    super::git::CloneOptions {
                        url: &git.url,
                        branch: &git.branch,
                        token: token.as_deref(),
                        dir: &dir,
                    },
                    |p| {
                        bus.publish(Event::DeployProgress {
                            deployment_id: did.clone(),
                            service_id: sid.clone(),
                            phase: "CloningRepo".into(),
                            percent: p.percent,
                            description: p.description,
                        });
                    },
                )
                .await?;

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
                    &self.db,
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
                let env = self.resolve_env(svc).await?;
                let replicas = svc.spec.replicas.max(1);
                let dep_short = self.short(&dep.id).to_string();

                if replicas == 1 {
                    // Single replica: caminho existente, healthcheck e swap tratados nos próximos estados
                    let cname =
                        containers::replica_staging_name(&svc.spec.name, &dep_short, 0);
                    info!(deployment_id = %dep.id, container_name = %cname, "step[Staging]: criando réplica única");
                    let id = containers::create_staging(
                        &self.docker.inner,
                        &svc.spec,
                        &image,
                        &svc.id,
                        &dep.id,
                        &network,
                        &env,
                        &cname,
                    )
                    .await?;
                    networks::connect_container(&self.docker.inner, &network, &id).await?;
                    containers::start(&self.docker.inner, &id).await?;
                    return Ok(DeployState::HealthcheckPolling);
                }

                // Multi-réplica: exige healthcheck configurado
                if svc.spec.healthcheck.kind == HealthcheckKind::None {
                    return Err(anyhow!(
                        "Deploy com múltiplas réplicas requer configuração de healthcheck"
                    ));
                }

                // Rolling update: uma réplica por vez — sobe → healthcheck → derruba antiga → promove
                info!(
                    deployment_id = %dep.id,
                    replicas = replicas,
                    "step[Staging/Rolling]: iniciando rolling update"
                );

                // Estado inicial: coleta IPs das réplicas live já existentes (None = primeiro deploy)
                let mut backends: Vec<Option<String>> = vec![None; replicas as usize];
                for i in 0..replicas {
                    let live = containers::replica_live_name(&svc.spec.name, i);
                    if let Ok(Some(cid)) =
                        containers::find_by_name(&self.docker.inner, &live).await
                    {
                        if let Ok(ip) =
                            containers::get_container_ip(&self.docker.inner, &cid, &network)
                                .await
                        {
                            backends[i as usize] = Some(format!("{ip}:{}", svc.spec.port));
                        }
                    }
                }

                for i in 0..replicas {
                    let staging =
                        containers::replica_staging_name(&svc.spec.name, &dep_short, i);
                    info!(
                        deployment_id = %dep.id,
                        replica = i,
                        container_name = %staging,
                        "step[Staging/Rolling]: criando nova réplica"
                    );

                    let staging_id = containers::create_staging(
                        &self.docker.inner,
                        &svc.spec,
                        &image,
                        &svc.id,
                        &dep.id,
                        &network,
                        &env,
                        &staging,
                    )
                    .await?;
                    networks::connect_container(&self.docker.inner, &network, &staging_id)
                        .await?;
                    containers::start(&self.docker.inner, &staging_id).await?;

                    let ip = containers::get_container_ip(
                        &self.docker.inner,
                        &staging_id,
                        &network,
                    )
                    .await?;
                    info!(
                        deployment_id = %dep.id,
                        replica = i,
                        ip = %ip,
                        "step[Staging/Rolling]: verificando healthcheck da nova réplica"
                    );
                    // Falha aqui → RollingBack remove todos os stagings pendentes
                    self.poll_healthcheck(&ip, &staging_id, svc, dep).await?;

                    // Derruba a réplica live antiga (se existir)
                    let live_name = containers::replica_live_name(&svc.spec.name, i);
                    if let Ok(Some(old_cid)) =
                        containers::find_by_name(&self.docker.inner, &live_name).await
                    {
                        info!(
                            deployment_id = %dep.id,
                            replica = i,
                            old_container = %old_cid,
                            "step[Staging/Rolling]: parando réplica anterior"
                        );
                        let _ = containers::stop_graceful(&self.docker.inner, &old_cid, 30)
                            .await;
                        let _ = containers::remove(&self.docker.inner, &old_cid).await;
                    }

                    // Promove staging → live
                    containers::rename(&self.docker.inner, &staging_id, &live_name).await?;
                    info!(
                        deployment_id = %dep.id,
                        replica = i,
                        new_name = %live_name,
                        "step[Staging/Rolling]: réplica promovida"
                    );

                    // Atualiza ingress com backends ativos até agora
                    backends[i as usize] = Some(format!("{ip}:{}", svc.spec.port));
                    let active: Vec<String> =
                        backends.iter().flatten().cloned().collect();
                    if let Some(domain) = &svc.spec.domain {
                        self.ingress.upsert_route(domain, active.clone(), &svc.id);
                    }
                    if let Some(host_port) = svc.spec.host_port {
                        self.ingress.upsert_port_route(host_port, active);
                    }

                    self.bus.publish(Event::DeployProgress {
                        deployment_id: dep.id.clone(),
                        service_id: svc.id.clone(),
                        phase: "RollingUpdate".into(),
                        percent: (((i + 1) as f32 / replicas as f32) * 100.0) as u8,
                        description: format!("replica {}/{replicas} ok", i + 1),
                    });
                }

                // Todas as réplicas substituídas; Promoting cuida do status no banco
                Ok(DeployState::Promoting)
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
                let replicas = svc.spec.replicas.max(1);
                let dep_short = self.short(&dep.id).to_string();
                let net = self.network_name(&svc.spec.project_id);

                // Coleta IPs de todas as réplicas para registrar no ingress
                let mut backends: Vec<String> = Vec::with_capacity(replicas as usize);
                for i in 0..replicas {
                    let staging =
                        containers::replica_staging_name(&svc.spec.name, &dep_short, i);
                    info!(
                        deployment_id = %dep.id,
                        replica = i,
                        container_name = %staging,
                        "step[SwappingIn]: resolvendo IP da réplica de staging"
                    );
                    let staging_id = containers::find_by_name(&self.docker.inner, &staging)
                        .await?
                        .ok_or_else(|| anyhow!("staging container not found: {staging}"))?;
                    let ip =
                        containers::get_container_ip(&self.docker.inner, &staging_id, &net)
                            .await?;
                    backends.push(format!("{ip}:{}", svc.spec.port));
                }

                if let Some(domain) = &svc.spec.domain {
                    info!(
                        deployment_id = %dep.id,
                        domain = %domain,
                        backends = ?backends,
                        "step[SwappingIn]: atualizando rota de domínio no ingress"
                    );
                    self.ingress.upsert_route(domain, backends.clone(), &svc.id);
                }
                if let Some(host_port) = svc.spec.host_port {
                    info!(
                        deployment_id = %dep.id,
                        host_port,
                        backends = ?backends,
                        "step[SwappingIn]: atualizando rota de porta no ingress"
                    );
                    self.ingress.upsert_port_route(host_port, backends.clone());
                }
                if svc.spec.domain.is_none() && svc.spec.host_port.is_none() {
                    info!(
                        deployment_id = %dep.id,
                        "step[SwappingIn]: sem domínio nem porta externa, ingress não atualizado"
                    );
                }

                // Para todas as instâncias antigas (suporte a replicas), excluindo as do deploy atual.
                match containers::find_old_containers(&self.docker.inner, &svc.id, &dep.id).await {
                    Ok(old_ids) if !old_ids.is_empty() => {
                        info!(
                            deployment_id = %dep.id,
                            count = old_ids.len(),
                            "step[SwappingIn]: parando instâncias live antigas"
                        );
                        for old in &old_ids {
                            let _ = containers::stop_graceful(&self.docker.inner, old, 30).await;
                        }
                        info!(deployment_id = %dep.id, "step[SwappingIn]: instâncias antigas paradas");
                    }
                    Ok(_) => {
                        info!(deployment_id = %dep.id, "step[SwappingIn]: nenhuma instância live anterior");
                    }
                    Err(e) => {
                        warn!(deployment_id = %dep.id, error = %e, "step[SwappingIn]: erro ao buscar containers antigos (ignorado)");
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
                let replicas = svc.spec.replicas.max(1);
                let dep_short = self.short(&dep.id).to_string();
                info!(
                    deployment_id = %dep.id,
                    replicas = replicas,
                    "step[Promoting]: promovendo staging → live"
                );

                // Remove todos os containers antigos (já parados no SwappingIn).
                match containers::find_old_containers(&self.docker.inner, &svc.id, &dep.id).await {
                    Ok(old_ids) => {
                        for old in &old_ids {
                            let _ = containers::remove(&self.docker.inner, old).await;
                        }
                        if !old_ids.is_empty() {
                            info!(deployment_id = %dep.id, count = old_ids.len(), "step[Promoting]: containers antigos removidos");
                        }
                    }
                    Err(e) => {
                        warn!(deployment_id = %dep.id, error = %e, "step[Promoting]: erro ao remover containers antigos (ignorado)");
                    }
                }

                // Renomeia cada réplica de staging → live.
                let mut primary_id = String::new();
                for i in 0..replicas {
                    let staging = containers::replica_staging_name(&svc.spec.name, &dep_short, i);
                    let live = containers::replica_live_name(&svc.spec.name, i);
                    let sid = match containers::find_by_name(&self.docker.inner, &staging).await? {
                        Some(id) => id,
                        None => {
                            warn!(deployment_id = %dep.id, replica = i, container_name = %staging, "step[Promoting]: réplica de staging não encontrada, pulando");
                            continue;
                        }
                    };
                    info!(
                        deployment_id = %dep.id,
                        replica = i,
                        container_id = %sid,
                        new_name = %live,
                        "step[Promoting]: renomeando réplica"
                    );
                    containers::rename(&self.docker.inner, &sid, &live).await?;
                    if i == 0 {
                        primary_id = sid;
                    }
                }

                info!(
                    deployment_id = %dep.id,
                    service_id = %svc.id,
                    container_id = %primary_id,
                    "step[Promoting]: atualizando status do serviço para Running"
                );
                crate::db::services::update_status(
                    &self.db,
                    &svc.id,
                    &ServiceStatus::Running,
                    if primary_id.is_empty() { None } else { Some(primary_id.as_str()) },
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

                // Transiciona qualquer deployment anterior em Live para Pruning
                // para evitar múltiplos registros Live para o mesmo serviço.
                if let Ok(history) =
                    crate::db::deployments::list_for_service(&self.db, &svc.id, 20).await
                {
                    for prev in history
                        .iter()
                        .filter(|d| d.id != dep.id && d.state == DeployState::Live)
                    {
                        let _ = crate::db::deployments::transition(
                            &self.db,
                            &prev.id,
                            &DeployState::Live,
                            DeployState::Pruning,
                            Some("superseded by newer deployment".into()),
                        )
                        .await;
                        self.bus.publish(Event::DeployStateChanged {
                            deployment_id: prev.id.clone(),
                            service_id: svc.id.clone(),
                            state: DeployState::Pruning,
                            timestamp: Utc::now(),
                            message: Some("superseded".into()),
                        });
                    }
                }

                Ok(DeployState::Live)
            }

            DeployState::RollingBack => {
                if let ServiceSource::Compose(compose) = &svc.spec.source {
                    let project_name = format!("rp_{}", &svc.spec.name);
                    info!(
                        deployment_id = %dep.id,
                        project = %project_name,
                        "step[RollingBack]: derrubando compose stack"
                    );
                    let network_name = self.network_name(&svc.spec.project_id);
                    let _ = crate::docker::compose::compose_down(
                        &compose.content,
                        &project_name,
                        &network_name,
                    )
                    .await;
                    let err_status = ServiceStatus::Error("deploy failed".into());
                    let _ =
                        crate::db::services::update_status(&self.db, &svc.id, &err_status, None)
                            .await;
                    self.bus.publish(Event::ServiceStatusChanged {
                        service_id: svc.id.clone(),
                        status: err_status,
                    });
                    return Ok(DeployState::Failed);
                }

                // Remove todos os containers de staging deste deployment.
                let replicas = svc.spec.replicas.max(1);
                let dep_short = self.short(&dep.id).to_string();
                info!(
                    deployment_id = %dep.id,
                    replicas = replicas,
                    "step[RollingBack]: removendo containers de staging"
                );
                for i in 0..replicas {
                    let staging = containers::replica_staging_name(&svc.spec.name, &dep_short, i);
                    if let Ok(Some(id)) = containers::find_by_name(&self.docker.inner, &staging).await {
                        let _ = containers::remove(&self.docker.inner, &id).await;
                        info!(deployment_id = %dep.id, replica = i, container_id = %id, "step[RollingBack]: staging removido");
                    }
                }

                // Restaura todos os backends live anteriores para o ingress
                let live_replicas = svc.spec.replicas.max(1);
                let net = self.network_name(&svc.spec.project_id);
                let mut live_backends: Vec<String> = Vec::new();
                for i in 0..live_replicas {
                    let live = containers::replica_live_name(&svc.spec.name, i);
                    if let Ok(Some(cid)) =
                        containers::find_by_name(&self.docker.inner, &live).await
                    {
                        if let Ok(ip) =
                            containers::get_container_ip(&self.docker.inner, &cid, &net).await
                        {
                            live_backends.push(format!("{ip}:{}", svc.spec.port));
                        }
                    }
                }
                if !live_backends.is_empty() {
                    if let Some(domain) = &svc.spec.domain {
                        info!(
                            deployment_id = %dep.id,
                            backends = ?live_backends,
                            "step[RollingBack]: restaurando rota de domínio para lives anteriores"
                        );
                        self.ingress.upsert_route(domain, live_backends.clone(), &svc.id);
                    }
                    if let Some(host_port) = svc.spec.host_port {
                        info!(
                            deployment_id = %dep.id,
                            host_port,
                            "step[RollingBack]: restaurando rota de porta para lives anteriores"
                        );
                        self.ingress.upsert_port_route(host_port, live_backends);
                    }
                } else {
                    info!(deployment_id = %dep.id, "step[RollingBack]: nenhum live anterior para restaurar");
                    if let Some(host_port) = svc.spec.host_port {
                        self.ingress.remove_port_route(host_port);
                    }
                }

                let err_status = ServiceStatus::Error("deploy failed".into());
                info!(
                    deployment_id = %dep.id,
                    service_id = %svc.id,
                    "step[RollingBack]: atualizando serviço para Error"
                );
                crate::db::services::update_status(&self.db, &svc.id, &err_status, None).await?;
                self.bus.publish(Event::ServiceStatusChanged {
                    service_id: svc.id.clone(),
                    status: err_status,
                });
                info!(deployment_id = %dep.id, "step[RollingBack]: rollback concluído, estado = Failed");
                let _ = std::fs::remove_dir_all(self.clone_dir(&dep.id));
                Ok(DeployState::Failed)
            }

            DeployState::ComposingUp => {
                let ServiceSource::Compose(compose) = &svc.spec.source else {
                    return Err(anyhow!("expected Compose source in ComposingUp"));
                };
                let project_name = format!("rp_{}", &svc.spec.name);
                info!(
                    deployment_id = %dep.id,
                    content_bytes = compose.content.len(),
                    project = %project_name,
                    "step[ComposingUp]: executando docker compose up"
                );
                let network_name = self.network_name(&svc.spec.project_id);
                crate::docker::compose::compose_up(
                    &compose.content,
                    &project_name,
                    &svc.id,
                    &dep.id,
                    &network_name,
                    &self.bus,
                    &self.db,
                )
                .await?;

                // Compose ingress: tenta encontrar o container principal para roteamento
                let main_container = format!("{}-{}", project_name, svc.spec.name);
                if let Ok(Some(cid)) = containers::find_by_prefix(&self.docker.inner, &main_container).await {
                    if let Ok(ip) = containers::get_container_ip(&self.docker.inner, &cid, &network_name).await {
                        let backend = format!("{ip}:{}", svc.spec.port);
                        if let Some(domain) = &svc.spec.domain {
                            info!(deployment_id = %dep.id, domain, backend, "ComposingUp: registrando rota de domínio");
                            self.ingress.upsert_route(domain, vec![backend.clone()], &svc.id);
                        }
                        if let Some(host_port) = svc.spec.host_port {
                            info!(deployment_id = %dep.id, host_port, backend, "ComposingUp: registrando rota de porta");
                            self.ingress.upsert_port_route(host_port, vec![backend.clone()]);
                        }
                    }
                }

                info!(
                    deployment_id = %dep.id,
                    project = %project_name,
                    "step[ComposingUp]: compose up concluído, promovendo serviço"
                );
                crate::db::services::update_status(
                    &self.db,
                    &svc.id,
                    &ServiceStatus::Running,
                    None,
                )
                .await?;
                self.bus.publish(Event::ServiceStatusChanged {
                    service_id: svc.id.clone(),
                    status: ServiceStatus::Running,
                });
                Ok(DeployState::Live)
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
                let exit_code = inspect.state.as_ref().and_then(|s| s.exit_code);
                error!(
                    deployment_id = %dep.id,
                    container_id = %container_id,
                    exit_code = ?exit_code,
                    "healthcheck: container parou inesperadamente"
                );
                return Err(anyhow!("container stopped during healthcheck"));
            }

            let ok = match &hc.kind {
                HealthcheckKind::None => return Ok(()),
                HealthcheckKind::Http {
                    path,
                    expected_status,
                } => {
                    let url = format!("http://{ip}:{}{path}", svc.spec.port);
                    debug!(deployment_id = %dep.id, url = %url, expected = expected_status, "healthcheck: HTTP check");
                    crate::health::check_http(&url, *expected_status, timeout).await
                }
                HealthcheckKind::Tcp => {
                    let addr = format!("{ip}:{}", svc.spec.port);
                    debug!(deployment_id = %dep.id, addr = %addr, "healthcheck: TCP check");
                    crate::health::check_tcp(&addr, timeout).await
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
            ServiceSource::Compose(c) => format!("compose:{}", c.content),
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
                    self.secrets.get_raw(&svc.spec.project_id, name).await.unwrap_or_default()
                }
            };
            env_map.insert(ev.key.clone(), value);
        }

        for ev in &svc.spec.env_vars {
            let value = match &ev.value {
                EnvVarValue::Plain(v) => v.clone(),
                EnvVarValue::Secret(name) => {
                    debug!(service_id = %svc.id, secret = %name, "resolve_env: desencriptando secret do serviço");
                    self.secrets.get_raw(&svc.spec.project_id, name).await.unwrap_or_default()
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
