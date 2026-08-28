use crate::{
    db::Db,
    docker,
    docker::{DockerClient, containers, images, networks},
    event_bus::EventBus,
    ingress::{IngressController, TlsManager},
    secrets::SecretsManager,
};
use anyhow::{Result, anyhow};
use bollard::models::HealthStatusEnum;
use chrono::Utc;
use shared::{
    compose_project_name,
    DeployState, Deployment, Event, HealthcheckKind, RustployConfig, Service, ServiceSource,
    ServiceStatus,
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else if ty.is_file() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

pub struct DeployExecutor {
    pub db: Arc<Db>,
    pub docker: Arc<DockerClient>,
    pub ingress: Arc<IngressController>,
    pub bus: Arc<EventBus>,
    pub secrets: Arc<SecretsManager>,
    pub tls: Arc<TlsManager>,
    pub db_path: PathBuf,
    pub drain_secs: u64,
    pub registry_internal_token: Option<Arc<str>>,
}

/// Reconhece se `image` aponta para o registry Docker embutido do próprio
/// rustployd, seja por loopback (`127.0.0.1:<port>`/`localhost:<port>`, caso
/// do deploy executor puxando no mesmo host) ou pelo domínio público
/// configurado (sem porta — o acesso externo passa pelo ingress, não fala
/// direto com a porta 5100).
fn is_embedded_registry_image(image: &str, port: u16, domain: Option<&str>) -> bool {
    if image.starts_with(&format!("127.0.0.1:{port}/")) {
        return true;
    }
    if image.starts_with(&format!("localhost:{port}/")) {
        return true;
    }
    if let Some(d) = domain {
        if image.starts_with(&format!("{d}/")) {
            return true;
        }
    }
    false
}

/// Quebra a causa de um step falho nas linhas que vão para o `build_log`.
///
/// Multi-linha vira VÁRIAS entradas, e não um registro só com `\n` dentro: o
/// renderizador de log do cliente (`fmt/service_detail.luau`) trata cada
/// registro como uma linha, então um `\n` embutido sairia espremido numa linha
/// só. Erro de `docker build` é multi-linha com frequência.
///
/// A primeira linha leva o marco `==>` (mesma convenção do resto do log) e cita
/// o estado em que o deploy quebrou — que hoje também se perdia: o log dizia
/// que falhou, nunca *em que etapa*. As continuações entram indentadas, para o
/// olho separar uma causa de várias linhas de vários passos seguidos.
fn failure_log_lines(state_label: &str, err: &str) -> Vec<String> {
    let mut linhas = err.lines().map(str::trim_end).filter(|l| !l.trim().is_empty());

    let Some(primeira) = linhas.next() else {
        // `anyhow!("")` é improvável, mas uma linha dizendo que não há mensagem
        // ainda é melhor que o log terminar mudo — que é o bug todo.
        return vec![format!("==> Erro em [{state_label}]: falha sem mensagem")];
    };

    let mut out = vec![format!("==> Erro em [{state_label}]: {primeira}")];
    out.extend(linhas.map(|l| format!("    {l}")));
    out
}

/// Mensagem de "Dockerfile não encontrado", especializada por fonte.
///
/// No caso `Git` o erro mais comum não é "esqueci de criar o Dockerfile", é
/// "criei mas não commitei/não fiz push" — por isso a mensagem cita o branch
/// clonado: aponta para essa hipótese sem precisar afirmá-la. O `build_context`
/// só aparece quando não é `.`, porque é ele que muda o significado do caminho
/// (o Dockerfile é procurado DENTRO do contexto).
fn missing_dockerfile_msg(source: &ServiceSource, dockerfile_path: &str) -> String {
    let contexto = |ctx: &str| {
        if ctx == "." || ctx.is_empty() {
            String::new()
        } else {
            format!(" (build context: {ctx})")
        }
    };

    match source {
        ServiceSource::Git(git) => format!(
            "Dockerfile não encontrado no repositório: {}{} — branch {}. \
             Confira se ele foi commitado e enviado, e se o caminho configurado \
             no serviço está certo.",
            dockerfile_path,
            contexto(&git.build_context),
            git.branch,
        ),
        ServiceSource::Archive(archive) => format!(
            "Dockerfile não encontrado no zip: {}{}",
            dockerfile_path,
            contexto(&archive.build_context),
        ),
        _ => format!("Dockerfile não encontrado: {dockerfile_path}"),
    }
}

/// Texto que vai para `ServiceStatus::Error` quando um deploy cai em rollback.
///
/// Era a string fixa `"deploy failed"` — o mesmo problema do log mudo, num
/// outro lugar: o chip de status do serviço afirmava que algo falhou sem nunca
/// dizer o quê. A causa está no `states_log` do próprio deployment (a transição
/// que ENTROU em `RollingBack` carrega a mensagem gravada pelo `execute()`), e
/// aqui ela já foi persistida: o laço recarrega o deployment do banco a cada
/// iteração, então o `dep` deste step é posterior à transição.
///
/// Só a primeira linha, e truncada: este campo é renderizado como chip/linha
/// única na UI, não como log.
fn rollback_cause(dep: &Deployment) -> String {
    dep.states_log
        .iter()
        .rev()
        .find(|t| t.to == DeployState::RollingBack)
        .and_then(|t| t.message.as_deref())
        .and_then(|m| m.lines().find(|l| !l.trim().is_empty()))
        .map(|primeira| truncate_chars(primeira.trim(), 160))
        .unwrap_or_else(|| "deploy failed".into())
}

/// Trunca por CARACTERE (não por byte): cortar um `&str` com índice de byte no
/// meio de um multibyte entra em pânico, e as mensagens aqui são em português.
fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cortado: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", cortado.trim_end())
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
                    {
                        let s = deployment_id.find('_').map(|i| &deployment_id[i + 1..]).unwrap_or(&deployment_id);
                        &s[..8.min(s.len())]
                    }
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
                    // A causa REAL da falha vai para o `build_log` — o único
                    // canal que a tela de log do serviço lê. Ela já era gravada
                    // em outros três (o `warn!` acima, o `states_log` do
                    // deployment e o `Event::DeployStateChanged` do SSE), e
                    // nenhum deles chega ao painel de log: o usuário via o
                    // deploy morrer num "==> Deploy falhou" mudo enquanto o
                    // motivo ("Cannot locate specified Dockerfile", healthcheck
                    // que não passou, pull negado…) existia no banco.
                    //
                    // ANTES do `transition` de propósito: assim a linha do
                    // motivo cai imediatamente acima do
                    // "==> Deploy falhou — iniciando rollback" que o
                    // `step[RollingBack]` grava em seguida, e o log se lê na
                    // ordem em que as coisas aconteceram.
                    for line in failure_log_lines(deployment.state.label(), &e.to_string()) {
                        self.log_step(deployment_id, &service.id, &line).await;
                    }
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
                self.log_step(&dep.id, &svc.id, "==> Iniciando deploy").await;
                let net = self.ensure_network(&svc.spec.project_id).await?;
                info!(
                    deployment_id = %dep.id,
                    network = %net,
                    "step[Pending]: rede pronta"
                );
                Ok(DeployState::PreDeployCheck)
            }

            DeployState::PreDeployCheck => {
                // Fila inteira roda dentro deste ÚNICO step (sem sub-estado por
                // índice): o loop de `execute()` não impõe timeout por step, e
                // manter a fila num só `PreDeployCheck` evita precisar persistir
                // "em qual item da fila estamos" (sem mudança de schema/wire).
                let checks = svc.spec.pre_deploy_checks();
                if checks.is_empty() {
                    return Ok(DeployState::ResolvingDeps);
                }
                let total = checks.len();

                for (idx, job_id) in checks.iter().enumerate() {
                    let job = crate::db::job::get(&self.db, job_id)
                        .await?
                        .ok_or_else(|| anyhow!("job de pré-deploy check não encontrado: {job_id}"))?;

                    info!(
                        deployment_id = %dep.id,
                        job_id = %job.id,
                        job_name = %job.name,
                        step = idx + 1,
                        total,
                        "step[PreDeployCheck]: rodando job de pré-deploy"
                    );
                    self.log_step(
                        &dep.id,
                        &svc.id,
                        &format!("==> Pré-deploy check {}/{}: {}", idx + 1, total, job.name),
                    )
                    .await;

                    let run = crate::db::job_run::create(&self.db, &job.id).await?;
                    self.bus.publish(Event::JobRunStateChanged {
                        job_id: job.id.clone(),
                        job_run_id: run.id.clone(),
                        running: true,
                        success: None,
                    });

                    let runner = crate::jobs::runner::JobRunner {
                        db: self.db.clone(),
                        docker: self.docker.clone(),
                        bus: self.bus.clone(),
                        secrets: self.secrets.clone(),
                        db_path: self.db_path.clone(),
                        registry_internal_token: self.registry_internal_token.clone(),
                    };
                    let result = runner
                        .run_inner_mirrored(&job, &run.id, Some((dep.id.clone(), svc.id.clone())), None)
                        .await;

                    let (exit_code, success) = match &result {
                        Ok(code) => (*code, *code == 0),
                        Err(e) => {
                            warn!(
                                deployment_id = %dep.id,
                                job_id = %job.id,
                                error = %e,
                                "step[PreDeployCheck]: falha ao executar job"
                            );
                            let _ = crate::db::job_run::finish(&self.db, &run.id, -1).await;
                            (-1, false)
                        }
                    };

                    self.bus.publish(Event::JobRunStateChanged {
                        job_id: job.id.clone(),
                        job_run_id: run.id.clone(),
                        running: false,
                        success: Some(success),
                    });

                    if !success {
                        // Primeira falha interrompe a fila inteira — os checks
                        // seguintes não rodam.
                        return Err(anyhow!(
                            "pré-deploy check {}/{} ({}) falhou (exit code {exit_code})",
                            idx + 1,
                            total,
                            job.name
                        ));
                    }
                    self.log_step(
                        &dep.id,
                        &svc.id,
                        &format!("--> Pré-deploy check {}/{} OK", idx + 1, total),
                    )
                    .await;
                }

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
                        self.log_step(&dep.id, &svc.id, &format!("--> Pulling image: {image}")).await;
                        DeployState::PullingImage
                    }
                    ServiceSource::Git(g) => {
                        info!(
                            deployment_id = %dep.id,
                            url = %g.url,
                            branch = %g.branch,
                            "step[ResolvingDeps]: fonte é Git → irá para CloningRepo"
                        );
                        self.log_step(&dep.id, &svc.id, &format!("--> Clonando repositório: {} ({})", g.url, g.branch)).await;
                        DeployState::CloningRepo
                    }
                    ServiceSource::Archive(a) => {
                        info!(
                            deployment_id = %dep.id,
                            archive_id = %a.archive_id,
                            "step[ResolvingDeps]: fonte é Archive → irá para BuildingImage"
                        );
                        self.log_step(&dep.id, &svc.id, "--> Preparando zip enviado").await;
                        DeployState::BuildingImage
                    }
                    ServiceSource::Compose(c) => {
                        info!(
                            deployment_id = %dep.id,
                            compose_file = %c.content,
                            "step[ResolvingDeps]: fonte é Compose → irá para ComposingUp"
                        );
                        self.log_step(&dep.id, &svc.id, "--> Executando docker compose up").await;
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
                let creds = self.registry_credentials_for(&image).await;
                images::pull(&self.docker.inner, &image, &svc.id, &dep.id, &self.bus, &self.db, creds).await?;
                self.log_step(&dep.id, &svc.id, &format!("--> Pull concluído: {image}")).await;
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
                let (token, clone_username) = super::git::resolve_clone_credentials(
                    &self.db,
                    &self.secrets,
                    git.provider_id.as_deref(),
                    git.credentials.as_deref(),
                    git.username.as_deref(),
                    &svc.spec.project_id,
                )
                .await;

                let dir = self.clone_dir(&dep.id);
                let bus = self.bus.clone();
                let sid = svc.id.clone();
                let did = dep.id.clone();
                
                info!(
                    deployment_id = %dep.id,
                    dir = %dir.display(),
                    git_url = &git.url,
                    "step[CloningRepo]: clonando para diretório"
                );

                super::git::clone(
                    super::git::CloneOptions {
                        url: &git.url,
                        branch: &git.branch,
                        token: token.as_deref(),
                        username: clone_username.as_deref(),
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

                self.log_step(&dep.id, &svc.id, "--> Clone concluído").await;
                info!(deployment_id = %dep.id, "step[CloningRepo]: clone concluído");
                Ok(DeployState::BuildingImage)
            }

            DeployState::BuildingImage => {
                let (context, dockerfile_path) = match &svc.spec.source {
                    ServiceSource::Git(git) => {
                        let clone_dir = self.clone_dir(&dep.id);
                        (clone_dir.join(&git.build_context), git.dockerfile_path.clone())
                    }
                    ServiceSource::Archive(archive) => {
                        let src = self.archive_dir(&svc.id, &archive.archive_id);
                        let dst = self.clone_dir(&dep.id);
                        if dst.exists() {
                            let _ = std::fs::remove_dir_all(&dst);
                        }
                        copy_dir_all(&src, &dst)?;
                        (dst.join(&archive.build_context), archive.dockerfile_path.clone())
                    }
                    _ => return Err(anyhow!("expected Git or Archive source")),
                };

                // Checagem ANTES de entregar ao Docker, para os dois ramos. Era
                // só do `Archive`: uma fonte `Git` sem Dockerfile commitado ia
                // direto para o `images::build` e voltava um lacônico
                // "docker build error: Cannot locate specified Dockerfile" —
                // que, até a correção acima, nem chegava ao log do usuário.
                //
                // O caminho é relativo ao CONTEXTO, não à raiz do clone/zip: é
                // exatamente o que `create_tar_gz` usa como nome dentro do tar
                // que manda pro Docker. A checagem antiga do `Archive` olhava a
                // raiz, então errava quando o `build_context` não era ".".
                if !context.join(&dockerfile_path).is_file() {
                    return Err(anyhow!(
                        "{}",
                        missing_dockerfile_msg(&svc.spec.source, &dockerfile_path)
                    ));
                }

                let tag = format!("rp_{}:{}", svc.spec.safe_name(), self.short(&dep.id));
                info!(
                    deployment_id = %dep.id,
                    tag = %tag,
                    dockerfile = %dockerfile_path,
                    context = %context.display(),
                    "step[BuildingImage]: iniciando build Docker"
                );
                self.log_step(&dep.id, &svc.id, &format!("--> Build Docker: {} ({})", tag, dockerfile_path)).await;
                images::build(
                    &self.docker.inner,
                    &self.db,
                    &context,
                    &dockerfile_path,
                    &tag,
                    &svc.id,
                    &dep.id,
                    &self.bus,
                )
                .await?;
                self.log_step(&dep.id, &svc.id, "--> Build concluído").await;
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
                    self.log_step(&dep.id, &svc.id, "--> Criando container de staging").await;
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
                let mut ips: Vec<Option<String>> = vec![None; replicas as usize];
                for i in 0..replicas {
                    let live = containers::replica_live_name(&svc.spec.name, i);
                    if let Ok(Some(cid)) =
                        containers::find_by_name(&self.docker.inner, &live).await
                    {
                        if let Ok(ip) =
                            containers::get_container_ip(&self.docker.inner, &cid, &network)
                                .await
                        {
                            ips[i as usize] = Some(ip);
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

                    // Atualiza ingress com os IPs ativos até agora
                    ips[i as usize] = Some(ip);
                    let active: Vec<String> = ips.iter().flatten().cloned().collect();
                    self.ingress.register_domains(&svc.spec, &active, &svc.id);
                    if let Some(host_port) = svc.spec.host_port {
                        let backends: Vec<String> =
                            active.iter().map(|ip| format!("{ip}:{}", svc.spec.port)).collect();
                        self.ingress.upsert_port_route(host_port, backends);
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
                self.log_step(&dep.id, &svc.id, &format!("--> Healthcheck: aguardando {ip}:{}", svc.spec.port)).await;
                self.poll_healthcheck(&ip, &cid, svc, dep).await?;
                self.log_step(&dep.id, &svc.id, "--> Healthcheck OK").await;
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

                // Coleta os IPs de todas as réplicas; cada rota de domínio
                // depois compõe `ip:porta` com a sua própria porta de container.
                let mut ips: Vec<String> = Vec::with_capacity(replicas as usize);
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
                    ips.push(ip);
                }

                if !svc.spec.domain_routes().is_empty() {
                    info!(
                        deployment_id = %dep.id,
                        domains = ?svc.spec.domain_routes().iter().map(|r| &r.domain).collect::<Vec<_>>(),
                        ips = ?ips,
                        "step[SwappingIn]: atualizando rotas de domínio no ingress"
                    );
                    self.ingress.register_domains(&svc.spec, &ips, &svc.id);
                }
                if let Some(host_port) = svc.spec.host_port {
                    let backends: Vec<String> =
                        ips.iter().map(|ip| format!("{ip}:{}", svc.spec.port)).collect();
                    info!(
                        deployment_id = %dep.id,
                        host_port,
                        backends = ?backends,
                        "step[SwappingIn]: atualizando rota de porta no ingress"
                    );
                    self.ingress.upsert_port_route(host_port, backends);
                    self.ensure_firewall(&dep.id, &svc.id, host_port).await;
                }
                if svc.spec.domain_routes().is_empty() && svc.spec.host_port.is_none() {
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
                self.log_step(&dep.id, &svc.id, "==> Deploy concluído — serviço Running ✓").await;
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

                // Provisiona certificado TLS em background (não bloqueia o
                // pipeline) para cada domínio com TLS habilitado.
                for route in svc.spec.domain_routes().into_iter().filter(|r| r.tls) {
                    let tls = self.tls.clone();
                    let domain = route.domain.clone();
                    tokio::spawn(async move {
                        if let Err(e) = tls.ensure_cert(&domain).await {
                            warn!(domain = %domain, error = %e, "TLS: falha ao provisionar certificado");
                        }
                    });
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
                self.log_step(&dep.id, &svc.id, "==> Deploy falhou — iniciando rollback").await;
                if let ServiceSource::Compose(compose) = &svc.spec.source {
                    let project_name = compose_project_name(&svc.id, &svc.spec.name);
                    info!(
                        deployment_id = %dep.id,
                        project = %project_name,
                        "step[RollingBack]: derrubando compose stack"
                    );
                    let network_name = self.network_name(&svc.spec.project_id);
                    let env_vars = self.resolve_env(&svc).await.unwrap_or_default();
                    let _ = docker::compose::down(
                        &compose.content,
                        &project_name,
                        &network_name,
                        &env_vars,
                    )
                    .await;
                    let err_status = ServiceStatus::Error(rollback_cause(dep));
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
                let mut live_ips: Vec<String> = Vec::new();
                for i in 0..live_replicas {
                    let live = containers::replica_live_name(&svc.spec.name, i);
                    if let Ok(Some(cid)) =
                        containers::find_by_name(&self.docker.inner, &live).await
                    {
                        if let Ok(ip) =
                            containers::get_container_ip(&self.docker.inner, &cid, &net).await
                        {
                            live_ips.push(ip);
                        }
                    }
                }
                if !live_ips.is_empty() {
                    if !svc.spec.domain_routes().is_empty() {
                        info!(
                            deployment_id = %dep.id,
                            ips = ?live_ips,
                            "step[RollingBack]: restaurando rotas de domínio para lives anteriores"
                        );
                        self.ingress.register_domains(&svc.spec, &live_ips, &svc.id);
                    }
                    if let Some(host_port) = svc.spec.host_port {
                        info!(
                            deployment_id = %dep.id,
                            host_port,
                            "step[RollingBack]: restaurando rota de porta para lives anteriores"
                        );
                        let backends: Vec<String> =
                            live_ips.iter().map(|ip| format!("{ip}:{}", svc.spec.port)).collect();
                        self.ingress.upsert_port_route(host_port, backends);
                    }
                } else {
                    info!(deployment_id = %dep.id, "step[RollingBack]: nenhum live anterior para restaurar");
                    if let Some(host_port) = svc.spec.host_port {
                        self.ingress.remove_port_route(host_port);
                    }
                }

                let err_status = ServiceStatus::Error(rollback_cause(dep));
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
                let project_name = compose_project_name(&svc.id, &svc.spec.name);
                info!(
                    deployment_id = %dep.id,
                    content_bytes = compose.content.len(),
                    project = %project_name,
                    "step[ComposingUp]: executando docker compose up"
                );
                let network_name = self.network_name(&svc.spec.project_id);
                let env_vars = self.resolve_env(&svc).await.unwrap_or_default();
                docker::compose::up(
                    &self.docker.inner,
                    &compose.content,
                    &project_name,
                    &svc.id,
                    &dep.id,
                    &network_name,
                    &self.bus,
                    &self.db,
                    &env_vars,
                    &self.clone_dir(&dep.id),
                    self.registry_internal_token.clone(),
                )
                .await?;

                // Compose ingress: busca qualquer container do projeto (prefix = "rp_<name>-")
                // O nome interno do serviço no compose file pode diferir do nome rustploy,
                // então usamos só o prefixo do projeto em vez de "rp_<name>-<name>".
                let main_container = format!("{}-", project_name);
                let live_container_id = containers::find_by_prefix(&self.docker.inner, &main_container)
                    .await
                    .ok()
                    .flatten();

                if let Some(cid) = &live_container_id {
                    if let Ok(ip) = containers::get_container_ip(&self.docker.inner, cid, &network_name).await {
                        let ips = vec![ip];
                        if !svc.spec.domain_routes().is_empty() {
                            info!(deployment_id = %dep.id, ?ips, "ComposingUp: registrando rotas de domínio");
                            self.ingress.register_domains(&svc.spec, &ips, &svc.id);
                        }
                        if let Some(host_port) = svc.spec.host_port {
                            let backend = format!("{}:{}", ips[0], svc.spec.port);
                            info!(deployment_id = %dep.id, host_port, backend, "ComposingUp: registrando rota de porta");
                            self.ingress.upsert_port_route(host_port, vec![backend]);
                            self.ensure_firewall(&dep.id, &svc.id, host_port).await;
                        }
                    }
                }

                self.log_step(&dep.id, &svc.id, "==> Compose up concluído — serviço Running ✓").await;
                info!(
                    deployment_id = %dep.id,
                    project = %project_name,
                    container_id = ?live_container_id,
                    "step[ComposingUp]: compose up concluído, promovendo serviço"
                );
                crate::db::services::update_status(
                    &self.db,
                    &svc.id,
                    &ServiceStatus::Running,
                    live_container_id.as_deref(),
                )
                .await?;
                self.bus.publish(Event::ServiceStatusChanged {
                    service_id: svc.id.clone(),
                    status: ServiceStatus::Running,
                });

                for route in svc.spec.domain_routes().into_iter().filter(|r| r.tls) {
                    let tls = self.tls.clone();
                    let domain = route.domain.clone();
                    tokio::spawn(async move {
                        if let Err(e) = tls.ensure_cert(&domain).await {
                            warn!(domain = %domain, error = %e, "TLS: falha ao provisionar certificado (compose)");
                        }
                    });
                }

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
                // Captura as últimas linhas do container antes do rollback removê-lo
                let crash_logs = containers::get_container_logs(&self.docker.inner, &container_id, 50).await;
                if crash_logs.is_empty() {
                    self.log_step(&dep.id, &svc.id, "  [sem output do container]").await;
                } else {
                    self.log_step(&dep.id, &svc.id, "--- output do container ---").await;
                    for line in &crash_logs {
                        self.log_step(&dep.id, &svc.id, line).await;
                    }
                    self.log_step(&dep.id, &svc.id, "--------------------------").await;
                }
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
        let s = id.find('_').map(|i| &id[i + 1..]).unwrap_or(id);
        &s[..8.min(s.len())]
    }

    /// Persiste uma linha de log de build no banco e a publica no event bus.
    /// Garante a liberação da porta externa no firewall do host (helper
    /// `rustployd-fw`) e registra o resultado no deploy log. Idempotente — rodar
    /// a cada deploy também re-cria a regra caso o admin a tenha removido.
    /// Falha nunca aborta o deploy (pior caso = porta bloqueada, como antes).
    async fn ensure_firewall(&self, deployment_id: &str, service_id: &str, host_port: u16) {
        let line = match crate::firewall::ensure_allowed(host_port).await {
            Ok(backend) if backend == "none" => format!(
                "--> Porta externa {host_port} exposta (nenhum firewall ativo no host)"
            ),
            Ok(backend) => format!("--> Porta externa {host_port} liberada no firewall ({backend})"),
            Err(e) => format!(
                "--> Aviso: não foi possível liberar a porta {host_port} no firewall: {e}. \
                 Se a conexão externa falhar, libere-a manualmente."
            ),
        };
        self.log_step(deployment_id, service_id, &line).await;
    }

    async fn log_step(&self, deployment_id: &str, service_id: &str, line: &str) {
        let ts = chrono::Utc::now();
        let _ = crate::db::build_logs::append(&self.db, deployment_id, line, ts).await;
        self.bus.publish(Event::BuildLog {
            deployment_id: deployment_id.to_string(),
            service_id: service_id.to_string(),
            line: line.to_string(),
            timestamp: ts,
        });
    }

    /// Se `image` aponta pro registry embutido do próprio rustployd,
    /// devolve as credenciais do token interno `rp-internal` (ver
    /// `crate::registry::internal_token`) pra que o Docker Engine do host
    /// consiga se autenticar no pull — necessário desde que a Fase 2 do
    /// registry passou a exigir Basic auth em toda rota, inclusive loopback.
    async fn registry_credentials_for(&self, image: &str) -> Option<bollard::auth::DockerCredentials> {
        let token = self.registry_internal_token.as_ref()?;
        let port = RustployConfig::global().registry.port;

        let mut domain = RustployConfig::global().registry.domain.clone();
        if let Ok(Some(d)) = crate::db::daemon_settings::get(
            &self.db,
            crate::db::daemon_settings::KEY_REGISTRY_DOMAIN,
        ).await {
            if !d.trim().is_empty() {
                domain = Some(d);
            }
        }

        if is_embedded_registry_image(image, port, domain.as_deref()) {
            Some(bollard::auth::DockerCredentials {
                username: Some("rp-internal".to_string()),
                password: Some(token.to_string()),
                ..Default::default()
            })
        } else {
            None
        }
    }

    fn image_for(&self, dep: &Deployment, svc: &Service) -> String {
        match &svc.spec.source {
            ServiceSource::Registry { image } => image.clone(),
            ServiceSource::Git(_) => format!("rp_{}:{}", svc.spec.safe_name(), self.short(&dep.id)),
            ServiceSource::Archive(_) => format!("rp_{}:{}", svc.spec.safe_name(), self.short(&dep.id)),
            ServiceSource::Compose(c) => format!("compose:{}", c.content),
        }
    }

    fn archive_dir(&self, service_id: &str, archive_id: &str) -> PathBuf {
        self.db_path
            .join("uploads")
            .join("services")
            .join(service_id)
            .join(archive_id)
    }

    fn network_name(&self, project_id: &str) -> String {
        networks::project_net_for(project_id)
    }

    async fn ensure_network(&self, project_id: &str) -> Result<String> {
        networks::ensure_project_network(&self.docker.inner, project_id).await
    }

    /// Wrapper fino: a lógica de verdade mora em `deploy::env_resolve::resolve`
    /// (reaproveitada pelo `JobRunner`, que precisa das mesmas env vars de
    /// base sem instanciar um `DeployExecutor` inteiro).
    async fn resolve_env(&self, svc: &Service) -> Result<Vec<(String, String)>> {
        super::env_resolve::resolve(&self.db, &self.secrets, svc).await
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

#[cfg(test)]
mod tests {
    use super::is_embedded_registry_image;

    #[test]
    fn reconhece_loopback_com_porta_certa() {
        assert!(is_embedded_registry_image("127.0.0.1:5100/app:v1", 5100, None));
    }

    #[test]
    fn nao_reconhece_porta_errada() {
        assert!(!is_embedded_registry_image("127.0.0.1:9999/app:v1", 5100, None));
    }

    #[test]
    fn reconhece_localhost() {
        assert!(is_embedded_registry_image("localhost:5100/app:v1", 5100, None));
    }

    #[test]
    fn reconhece_dominio_configurado_sem_porta() {
        assert!(is_embedded_registry_image(
            "registry.exemplo.com/app:v1", 5100, Some("registry.exemplo.com")
        ));
    }

    #[test]
    fn nao_reconhece_dominio_com_porta_5100_anexada() {
        // domínio:porta NUNCA é a forma certa (porta só existe em loopback) —
        // garantir que esse caso não bate por acidente com o prefixo do domínio.
        assert!(!is_embedded_registry_image(
            "registry.exemplo.com:5100/app:v1", 5100, Some("registry.exemplo.com")
        ));
    }

    #[test]
    fn imagem_externa_nao_bate() {
        assert!(!is_embedded_registry_image("nginx:latest", 5100, None));
        assert!(!is_embedded_registry_image("ghcr.io/user/app:v1", 5100, Some("registry.exemplo.com")));
    }
}

/// Testa só os dois ramos de `step[PreDeployCheck]` que não tocam Docker
/// (sem job configurado = no-op; job apagado/inexistente = falha) — o ramo
/// de exit code exigiria um `docker compose up` real, fora do escopo de um
/// teste unitário (nenhum outro teste deste crate sobe containers de verdade).
#[cfg(test)]
mod pre_deploy_check_tests {
    use super::*;
    use crate::{db, docker::DockerClient, event_bus::EventBus, ingress::{IngressController, TlsManager}, secrets::SecretsManager};
    use shared::config::AcmeConfig;
    use shared::{Healthcheck, ResourceLimits, ServiceSource, ServiceSpec};
    use ulid::Ulid;

    pub(super) async fn test_executor() -> DeployExecutor {
        let dir = std::env::temp_dir().join(format!("rustploy_test_predeploy_{}", Ulid::new()));
        let db = Arc::new(db::connect(&dir).await.unwrap());
        let docker = Arc::new(DockerClient::connect("/var/run/docker.sock").unwrap());
        let secrets = Arc::new(SecretsManager::new(&dir.join("master.key"), db.clone()).unwrap());
        let tls = Arc::new(
            TlsManager::new(
                dir.join("certs"),
                AcmeConfig { enabled: false, email: None, directory: String::new() },
            )
            .unwrap(),
        );
        DeployExecutor {
            db,
            docker,
            ingress: Arc::new(IngressController::new()),
            bus: Arc::new(EventBus::new()),
            secrets,
            tls,
            db_path: dir,
            drain_secs: 5,
            registry_internal_token: None,
        }
    }

    pub(super) fn spec(pre_deploy_job_ids: Vec<String>) -> ServiceSpec {
        ServiceSpec {
            name: format!("svc-{}", Ulid::new()),
            project_id: "proj-1".into(),
            source: ServiceSource::Registry { image: "nginx:latest".into() },
            port: 8080,
            host_port: None,
            domain: None,
            tls_enabled: false,
            env_vars: vec![],
            env_comments: vec![],
            volumes: vec![],
            healthcheck: Healthcheck::default(),
            replicas: 1,
            resources: ResourceLimits::default(),
            run_command: None,
            run_args: vec![],
            db_kind: None,
            domains: vec![],
            pre_deploy_job_id: None,
            pre_deploy_job_ids,
        }
    }

    #[tokio::test]
    async fn fila_vazia_e_no_op() {
        let executor = test_executor().await;
        let svc = db::services::create(&executor.db, spec(vec![])).await.unwrap();
        let mut dep = db::deployments::create(&executor.db, &svc.id, "nginx:latest")
            .await
            .unwrap();
        dep.state = DeployState::PreDeployCheck;

        let next = executor.step(&dep, &svc).await.unwrap();
        assert_eq!(next, DeployState::ResolvingDeps);
    }

    #[tokio::test]
    async fn job_inexistente_falha_o_step() {
        let executor = test_executor().await;
        let svc = db::services::create(&executor.db, spec(vec!["job_nao_existe".into()]))
            .await
            .unwrap();
        let mut dep = db::deployments::create(&executor.db, &svc.id, "nginx:latest")
            .await
            .unwrap();
        dep.state = DeployState::PreDeployCheck;

        let err = executor.step(&dep, &svc).await.unwrap_err();
        assert!(err.to_string().contains("não encontrado"));
    }

    /// Fila com dois checks, ambos apontando pra jobs que não existem: a
    /// falha tem que citar o PRIMEIRO da fila (`job_ausente_1`) — prova que o
    /// loop parou ali e nunca chegou a olhar o segundo item.
    #[tokio::test]
    async fn fila_para_no_primeiro_job_ausente_sem_chegar_no_segundo() {
        let executor = test_executor().await;
        let svc = db::services::create(
            &executor.db,
            spec(vec!["job_ausente_1".into(), "job_ausente_2".into()]),
        )
        .await
        .unwrap();
        let mut dep = db::deployments::create(&executor.db, &svc.id, "nginx:latest")
            .await
            .unwrap();
        dep.state = DeployState::PreDeployCheck;

        let err = executor.step(&dep, &svc).await.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("job_ausente_1"));
        assert!(!msg.contains("job_ausente_2"));
    }

    /// Retrocompat: specs antigos só têm `pre_deploy_job_id` (singular) —
    /// `pre_deploy_checks()` precisa cair nesse legado quando a fila nova
    /// está vazia (ver `ServiceSpec::pre_deploy_checks`).
    #[tokio::test]
    async fn legado_pre_deploy_job_id_ainda_funciona_via_fallback() {
        let executor = test_executor().await;
        let mut s = spec(vec![]);
        s.pre_deploy_job_id = Some("job_legado_nao_existe".into());
        let svc = db::services::create(&executor.db, s).await.unwrap();
        let mut dep = db::deployments::create(&executor.db, &svc.id, "nginx:latest")
            .await
            .unwrap();
        dep.state = DeployState::PreDeployCheck;

        let err = executor.step(&dep, &svc).await.unwrap_err();
        assert!(err.to_string().contains("job_legado_nao_existe"));
    }
}

/// Regressão do "erro de deploy invisível": a causa de um step falho tem que
/// (a) virar linha(s) de `build_log` — o único canal que a tela de log lê — e
/// (b) existir de verdade para o caso do Dockerfile ausente numa fonte `Git`,
/// que antes só o ramo `Archive` checava. Ver
/// `docs/plano-erro-de-deploy-invisivel.md`.
#[cfg(test)]
mod failure_log_tests {
    use super::pre_deploy_check_tests::{spec, test_executor};
    use super::*;
    use crate::db;
    use shared::{GitSource, ServiceSpec, StateTransition};

    fn git_spec() -> ServiceSpec {
        let mut s = spec(vec![]);
        s.source = ServiceSource::Git(GitSource {
            url: "https://github.com/exemplo/app".into(),
            branch: "main".into(),
            dockerfile_path: "Dockerfile".into(),
            build_context: ".".into(),
            ..GitSource::default()
        });
        s
    }

    // ── failure_log_lines ──────────────────────────────────────────────────

    #[test]
    fn causa_de_uma_linha_vira_um_marco_com_o_estado() {
        let linhas = failure_log_lines(
            "BuildingImage",
            "docker build error: Cannot locate specified Dockerfile: Dockerfile",
        );
        assert_eq!(linhas.len(), 1);
        assert_eq!(
            linhas[0],
            "==> Erro em [BuildingImage]: docker build error: \
             Cannot locate specified Dockerfile: Dockerfile"
        );
    }

    /// Erro de `docker build` costuma ser multi-linha: cada linha vira um
    /// registro próprio (o renderizador do cliente trata registro = linha), e
    /// as continuações vêm indentadas sob o marco.
    #[test]
    fn causa_multilinha_vira_varios_registros_indentados() {
        let linhas = failure_log_lines("Staging", "falhou ao subir\n  porta 8080 ocupada\n\n");
        assert_eq!(
            linhas,
            vec![
                "==> Erro em [Staging]: falhou ao subir".to_string(),
                "      porta 8080 ocupada".to_string(),
            ]
        );
    }

    #[test]
    fn causa_vazia_ainda_produz_uma_linha() {
        let linhas = failure_log_lines("Promoting", "   \n  ");
        assert_eq!(linhas.len(), 1);
        assert!(linhas[0].contains("falha sem mensagem"));
    }

    // ── missing_dockerfile_msg ─────────────────────────────────────────────

    /// O erro mais comum não é "não criei o Dockerfile", é "criei mas não fiz
    /// push" — por isso a mensagem do ramo Git cita o branch clonado.
    #[test]
    fn mensagem_do_git_cita_o_branch() {
        let msg = missing_dockerfile_msg(
            &ServiceSource::Git(GitSource {
                branch: "producao".into(),
                build_context: ".".into(),
                ..GitSource::default()
            }),
            "Dockerfile",
        );
        assert!(msg.contains("repositório"));
        assert!(msg.contains("producao"));
        // `build_context` "." não polui a mensagem.
        assert!(!msg.contains("build context"));
    }

    #[test]
    fn mensagem_cita_o_build_context_quando_ele_nao_e_a_raiz() {
        let msg = missing_dockerfile_msg(
            &ServiceSource::Archive(shared::ArchiveSource {
                build_context: "apps/web".into(),
                ..Default::default()
            }),
            "Dockerfile",
        );
        assert!(msg.contains("zip"));
        assert!(msg.contains("build context: apps/web"));
    }

    // ── rollback_cause ─────────────────────────────────────────────────────

    #[test]
    fn causa_do_rollback_vem_da_transicao_que_entrou_em_rollingback() {
        let mut dep = Deployment {
            id: "dep_1".into(),
            service_id: "svc_1".into(),
            image: "nginx".into(),
            state: DeployState::RollingBack,
            states_log: vec![],
            started_at: Utc::now(),
            finished_at: None,
        };
        dep.states_log.push(StateTransition {
            from: DeployState::Pending,
            to: DeployState::BuildingImage,
            at: Utc::now(),
            message: None,
        });
        dep.states_log.push(StateTransition {
            from: DeployState::BuildingImage,
            to: DeployState::RollingBack,
            at: Utc::now(),
            message: Some("docker build error: sem Dockerfile\ndetalhe ignorado".into()),
        });

        // Só a primeira linha, sem a continuação — este texto vira chip de
        // status na UI, não log.
        assert_eq!(rollback_cause(&dep), "docker build error: sem Dockerfile");
    }

    #[test]
    fn causa_do_rollback_cai_no_texto_generico_quando_nao_ha_mensagem() {
        let dep = Deployment {
            id: "dep_1".into(),
            service_id: "svc_1".into(),
            image: "nginx".into(),
            state: DeployState::RollingBack,
            states_log: vec![],
            started_at: Utc::now(),
            finished_at: None,
        };
        assert_eq!(rollback_cause(&dep), "deploy failed");
    }

    #[test]
    fn causa_longa_e_truncada_por_caractere() {
        let longa = "ç".repeat(400);
        let cortada = truncate_chars(&longa, 160);
        // Truncar por índice de BYTE no meio de um multibyte entraria em pânico.
        assert_eq!(cortada.chars().count(), 160);
        assert!(cortada.ends_with('…'));
    }

    // ── o step de build falha ANTES do Docker quando falta o Dockerfile ─────

    /// Fonte `Git` sem Dockerfile no contexto: o step tem que falhar sozinho,
    /// com mensagem acionável, sem nunca chamar o `images::build` (este teste
    /// roda sem Docker, e o diretório de clone nem existe).
    #[tokio::test]
    async fn step_building_image_falha_com_mensagem_util_sem_dockerfile() {
        let executor = test_executor().await;
        let svc = db::services::create(&executor.db, git_spec()).await.unwrap();
        let mut dep = db::deployments::create(&executor.db, &svc.id, "rp_app:1")
            .await
            .unwrap();
        dep.state = DeployState::BuildingImage;

        let err = executor.step(&dep, &svc).await.unwrap_err().to_string();
        assert!(err.contains("Dockerfile não encontrado no repositório"), "{err}");
        assert!(err.contains("main"), "{err}");
    }

    /// A ponta que faltava: a causa persistida no `build_log`, que é de onde a
    /// tela de log do serviço lê. Antes desta correção nada escrevia o erro ali
    /// e o log terminava num "==> Deploy falhou" sem motivo.
    #[tokio::test]
    async fn causa_do_step_falho_e_persistida_no_build_log() {
        let executor = test_executor().await;
        let svc = db::services::create(&executor.db, git_spec()).await.unwrap();
        let dep = db::deployments::create(&executor.db, &svc.id, "rp_app:1")
            .await
            .unwrap();

        let erro = executor
            .step(
                &Deployment { state: DeployState::BuildingImage, ..dep.clone() },
                &svc,
            )
            .await
            .unwrap_err();

        for linha in failure_log_lines(DeployState::BuildingImage.label(), &erro.to_string()) {
            executor.log_step(&dep.id, &svc.id, &linha).await;
        }

        let log = db::build_logs::get_for_deployment(&executor.db, &dep.id)
            .await
            .unwrap();
        assert!(!log.is_empty());
        assert!(log[0].line.starts_with("==> Erro em [BuildingImage]:"), "{}", log[0].line);
        assert!(
            log.iter().any(|l| l.line.contains("Dockerfile não encontrado no repositório")),
            "log: {:?}",
            log.iter().map(|l| &l.line).collect::<Vec<_>>()
        );
    }
}
