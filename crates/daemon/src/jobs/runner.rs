//! Execução de um `Job` (tarefa one-shot via docker-compose): resolve rede +
//! env vars do serviço gatilho, sobe o stack até `main_service` terminar
//! (`docker::compose::run_once`), grava o resultado em `job_run`/`job_log` e
//! reagenda (`Job::recurrence`, quando houver).

use crate::api::AppState;
use crate::db;
use crate::docker::{self, networks, DockerClient};
use crate::event_bus::EventBus;
use crate::secrets::SecretsManager;
use anyhow::{anyhow, Result};
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
    });
    let run_id = run.id.clone();
    tokio::spawn(async move {
        runner.run(&job, run_id).await;
    });
    Ok(run)
}

impl JobRunner {
    pub async fn run(&self, job: &Job, run_id: String) {
        info!(job_id = %job.id, run_id = %run_id, "job_runner: iniciando execução");
        self.bus.publish(Event::JobRunStateChanged {
            job_id: job.id.clone(),
            job_run_id: run_id.clone(),
            running: true,
            success: None,
        });

        let result = self.run_inner(job, &run_id).await;
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

    async fn run_inner(&self, job: &Job, run_id: &str) -> Result<i32> {
        let svc = db::services::get(&self.db, &job.trigger_service_id)
            .await?
            .ok_or_else(|| anyhow!("serviço gatilho não encontrado: {}", job.trigger_service_id))?;

        let network_name = networks::ensure_project_network(&self.docker.inner, &job.project_id).await?;
        let env_vars = crate::deploy::env_resolve::resolve(&self.db, &self.secrets, &svc).await?;

        // Nome de projeto do compose: só minúsculas/dígitos/`_`/`-` (regra do
        // próprio `docker compose`) — o run_id (ULID) tem letras maiúsculas.
        let project_name = run_id.to_lowercase();
        let build_dir = self.db_path.join("jobs").join(run_id);

        let exit_code = docker::compose::run_once(
            &job.compose,
            &project_name,
            &network_name,
            &job.main_service,
            &job.id,
            run_id,
            &self.bus,
            &self.db,
            &env_vars,
            &build_dir,
        )
        .await?;

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
