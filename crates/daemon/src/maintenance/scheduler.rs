//! Ticker que verifica se a limpeza automática de Docker está devida —
//! mesmo idioma de `jobs::scheduler::scheduler_loop`, mas sem tabela própria:
//! a config é um singleton em `daemon_settings`, então cada tick só carrega
//! um JSON e compara `next_run_at` (persistido, nunca recalculado a partir
//! de "agora" — ver `DockerCleanupConfig::recompute_next_run`) contra `now`.

use crate::api::AppState;
use chrono::Utc;
use std::time::Duration;
use tokio::time::MissedTickBehavior;
use tracing::warn;

const TICK_SECS: u64 = 60;

pub async fn scheduler_loop(state: AppState) {
    let mut ticker = tokio::time::interval(Duration::from_secs(TICK_SECS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        ticker.tick().await;

        let config = match super::run::load_config(&state).await {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "maintenance scheduler: falha ao carregar config");
                continue;
            }
        };

        let due = match config.next_run_at {
            Some(next) => Utc::now() >= next,
            None => false,
        };
        if due {
            super::run::run(&state, config).await;
        }
    }
}
