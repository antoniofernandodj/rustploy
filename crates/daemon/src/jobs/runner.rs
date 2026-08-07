//! Execução de um `Job` (tarefa one-shot via docker-compose): resolve rede +
//! env vars do serviço gatilho, sobe o stack até `main_service` terminar
//! (`docker::compose::run_once`), grava o resultado em `job_run`/`job_log` e
//! reagenda (`Job::recurrence`, quando houver).

use crate::api::AppState;
use crate::db;
use crate::docker::{self, networks, DockerClient};
use crate::event_bus::EventBus;
use crate::secrets::SecretsManager;
use anyhow::Result;
use chrono::Utc;
use shared::{Event, Job, JobRun};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct JobRunner {
    pub db: Arc<db::Db>,
    pub docker: Arc<DockerClient>,
    pub bus: Arc<EventBus>,
    pub secrets: Arc<SecretsManager>,
    pub db_path: PathBuf,
    pub registry_internal_token: Option<Arc<str>>,
}

/// Cria o `job_run` e dispara a execução em background (`tokio::spawn`) —
/// usado tanto pelo `scheduler_loop` (jobs vencidos) quanto por
/// `Command::JobRunNow` (disparo manual). Retorna assim que o `job_run` é
/// criado; o resultado chega depois via `Event::JobRunStateChanged` +
/// `Command::JobRunHistory`/`GetJobLogs`.
pub async fn spawn(state: &AppState, job: Job) -> Result<JobRun> {
    let run = db::job_run::create(&state.db, &job.id).await?;
    let runner = Arc::new(JobRunner {
        db: state.db.clone(),
        docker: state.docker.clone(),
        bus: state.bus.clone(),
        secrets: state.secrets.clone(),
        db_path: state.db_path.clone(),
        registry_internal_token: state.registry_internal_token.clone(),
    });
    let run_id = run.id.clone();

    // Sinal de cancelamento (`Command::JobRunCancel` → `AppState::
    // active_jobs`): registrado ANTES da task começar a rodar, removido
    // depois de terminar (sucesso, falha ou cancelamento) — mesmo idioma de
    // `active_deploys`, mas com um `watch<bool>` observado dentro de
    // `docker::compose::run_once_up` em vez de abortar a task (abortar a
    // task não mataria o processo `docker compose up` filho). Só o caminho
    // scheduler/`JobRunNow` (aqui) registra — o `run_inner_mirrored` chamado
    // pelo `DeployExecutor` (`PreDeployCheck`) passa `None`, fora de escopo
    // por ora.
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    if let Ok(mut map) = state.active_jobs.lock() {
        map.insert(run_id.clone(), cancel_tx);
    }
    let active_jobs = state.active_jobs.clone();

    tokio::spawn(async move {
        runner.run(&job, run_id.clone(), Some(cancel_rx)).await;
        if let Ok(mut map) = active_jobs.lock() {
            map.remove(&run_id);
        }
    });
    Ok(run)
}

impl JobRunner {
    pub async fn run(
        &self,
        job: &Job,
        run_id: String,
        cancel_rx: Option<tokio::sync::watch::Receiver<bool>>,
    ) {
        info!(job_id = %job.id, run_id = %run_id, "job_runner: iniciando execução");
        self.bus.publish(Event::JobRunStateChanged {
            job_id: job.id.clone(),
            job_run_id: run_id.clone(),
            running: true,
            success: None,
        });

        let result = self.run_inner_mirrored(job, &run_id, None, cancel_rx).await;
        let success = match &result {
            Ok(exit_code) => *exit_code == 0,
            Err(e) => {
                error!(job_id = %job.id, run_id = %run_id, error = %e, "job_runner: falha ao executar job");
                let _ = db::job_run::finish(&self.db, &run_id, -1).await;
                false
            }
        };

        self.bus.publish(Event::JobRunStateChanged {
            job_id: job.id.clone(),
            job_run_id: run_id.clone(),
            running: false,
            success: Some(success),
        });

        self.reschedule(job).await;
    }

    /// Roda `job` até o `main_service` terminar e devolve o exit code — sem
    /// tocar em `job.recurrence`/`reschedule` (isso é responsabilidade de
    /// `run()`, chamado pelo scheduler/`JobRunNow`). `mirror_deployment`,
    /// quando `Some((deployment_id, service_id))`, também espelha a saída
    /// como `Event::BuildLog` — usado pelo `DeployExecutor` no estado
    /// `PreDeployCheck` (ver `docs/plano-pre-deploy-gate.md`) pra a saída do
    /// check aparecer na tela de deploy; nesse caminho `cancel_rx` é sempre
    /// `None` (cancelamento de job só é suportado no caminho scheduler/
    /// `JobRunNow`, via `spawn()` — ver `docs/plano-cancelamento-de-jobs.md`).
    pub(crate) async fn run_inner_mirrored(
        &self,
        job: &Job,
        run_id: &str,
        mirror_deployment: Option<(String, String)>,
        cancel_rx: Option<tokio::sync::watch::Receiver<bool>>,
    ) -> Result<i32> {
        let network_name = networks::ensure_project_network(&self.docker.inner, &job.project_id).await?;

        // Base (projeto [+ serviço gatilho]) + overrides do próprio job, maior
        // precedência — ver deploy::env_resolve::resolve_job.
        let env_vars = crate::deploy::env_resolve::resolve_job(&self.db, &self.secrets, job).await?;

        // Nome de projeto do compose: só minúsculas/dígitos/`_`/`-` (regra do
        // próprio `docker compose`) — o run_id (ULID) tem letras maiúsculas.
        let project_name = run_id.to_lowercase();
        let build_dir = self.db_path.join("jobs").join(run_id);

        // Job git-sourced: clona pra dentro do PRÓPRIO build_dir (vira o
        // checkout) antes do compose — mesma resolução de credenciais do
        // deploy de serviço git-sourced (`deploy::git::resolve_clone_
        // credentials`), reusada aqui pra não duplicar. O progresso do clone
        // vira linha de log do job (mesmo bus_batch que o stdout do compose),
        // pra aparecer ao vivo em "Ver logs" igual o resto da execução.
        let build_source = match &job.git_source {
            Some(git) => {
                let (token, username) = crate::deploy::git::resolve_clone_credentials(
                    &self.db,
                    &self.secrets,
                    git.provider_id.as_deref(),
                    git.credentials.as_deref(),
                    git.username.as_deref(),
                    &job.project_id,
                )
                .await;

                let bus = self.bus.clone();
                let db_handle = self.db.clone();
                let jid = job.id.clone();
                let rid = run_id.to_string();
                crate::deploy::git::clone(
                    crate::deploy::git::CloneOptions {
                        url: &git.url,
                        branch: &git.branch,
                        token: token.as_deref(),
                        username: username.as_deref(),
                        dir: &build_dir,
                    },
                    move |p| {
                        let ts = Utc::now();
                        bus.publish(Event::JobLogLine {
                            job_run_id: rid.clone(),
                            job_id: jid.clone(),
                            line: p.description.clone(),
                            timestamp: ts,
                            stream: shared::protocol::LogStream::Stdout,
                        });
                        let db_handle = db_handle.clone();
                        let rid = rid.clone();
                        let line = p.description;
                        tokio::spawn(async move {
                            let _ = db::job_log::append(
                                &db_handle,
                                &rid,
                                &shared::protocol::LogStream::Stdout,
                                &line,
                                ts,
                            )
                            .await;
                        });
                    },
                )
                .await?;

                docker::compose::JobBuildSource::Git {
                    compose_rel_path: &git.compose_path,
                }
            }
            None => docker::compose::JobBuildSource::Compose(&job.compose),
        };

        // Cancelamento pode chegar durante um clone Git lento (não
        // interrompido no meio — só checado depois); sem isso, o job
        // seguiria pra subir containers mesmo já tendo sido cancelado.
        let already_cancelled = cancel_rx.as_ref().map(|rx| *rx.borrow()).unwrap_or(false);
        let exit_code = if already_cancelled {
            docker::compose::CANCELLED_EXIT_CODE
        } else {
            docker::compose::run_once(
                build_source,
                &project_name,
                &network_name,
                &job.main_service,
                &job.id,
                run_id,
                &self.bus,
                &self.db,
                &env_vars,
                &build_dir,
                self.registry_internal_token.clone(),
                mirror_deployment,
                cancel_rx,
            )
            .await?
        };

        db::job_run::finish(&self.db, run_id, exit_code).await?;

        if let Err(e) = tokio::fs::remove_dir_all(&build_dir).await {
            warn!(run_id = %run_id, error = %e, "job_runner: falha ao limpar build_dir (best-effort)");
        }

        Ok(exit_code)
    }

    async fn reschedule(&self, job: &Job) {
        let now = Utc::now();
        let next_run_at = job.recurrence.map(|r| r.next_after(now));
        if let Err(e) = db::job::mark_fired(&self.db, &job.id, now, next_run_at).await {
            error!(job_id = %job.id, error = %e, "job_runner: falha ao reagendar");
        }
    }
}
