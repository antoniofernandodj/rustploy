//! Ticker de agendamento dos jobs one-shot — mesmo formato de `metrics.rs`/
//! `env_backup.rs`: `tokio::time::interval`, a cada tick busca os jobs
//! vencidos e dispara cada um em background (não bloqueia o próprio ticker).

use crate::api::AppState;
use crate::db;
use std::time::Duration;
use tokio::time::MissedTickBehavior;
use tracing::warn;

const TICK_SECS: u64 = 30;

pub async fn scheduler_loop(state: AppState) {
    let mut ticker = tokio::time::interval(Duration::from_secs(TICK_SECS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        ticker.tick().await;
        let due = match db::job::list_due(&state.db, chrono::Utc::now()).await {
            Ok(jobs) => jobs,
            Err(e) => {
                warn!(error = %e, "scheduler_loop: falha ao listar jobs vencidos");
                continue;
            }
        };
        for job in due {
            let job_id = job.id.clone();
            if let Err(e) = super::runner::spawn(&state, job).await {
                warn!(job_id = %job_id, error = %e, "scheduler_loop: falha ao disparar job agendado");
            }
        }
    }
}
