//! Execução de uma limpeza (agendada ou "Executar agora"): roda os recursos
//! marcados em `DockerCleanupConfig` chamando as MESMAS funções `*_core` que
//! os botões manuais da aba Docker usam, agrega o resultado, publica
//! `Event::DockerCleanupCompleted` e persiste `last_run_at`/`next_run_at` +
//! um resumo, pra tela mostrar "última limpeza" sem depender do SSE.

use crate::api::AppState;
use crate::api::handlers::docker_prune::{self, PruneStat};
use crate::db::daemon_settings;
use chrono::Utc;
use shared::{DockerCleanupConfig, DockerCleanupLastRun, DockerCleanupResourceResult, Event};
use tracing::{info, warn};

/// Ordem importa: containers primeiro (libera imagens/redes em uso por eles),
/// depois imagens, depois volumes/redes que só ficam "sem uso" após isso.
pub async fn run(state: &AppState, mut config: DockerCleanupConfig) -> DockerCleanupConfig {
    let mut results = Vec::new();

    if config.containers {
        results.push(record(
            "containers",
            docker_prune::prune_containers_core(state).await,
        ));
    }
    if config.images {
        results.push(record(
            "images",
            docker_prune::prune_images_core(state, config.images_all).await,
        ));
    }
    if config.volumes {
        results.push(record(
            "volumes",
            docker_prune::prune_volumes_core(state, config.volumes_all).await,
        ));
    }
    if config.networks {
        results.push(record(
            "networks",
            docker_prune::prune_networks_core(state).await,
        ));
    }
    if config.build_cache {
        results.push(record(
            "build_cache",
            docker_prune::prune_build_cache_core().await,
        ));
    }

    let now = Utc::now();
    config.last_run_at = Some(now);
    config.recompute_next_run(now);

    if let Err(e) = save_config(state, &config).await {
        warn!(error = %e, "maintenance: falha ao persistir config após execução");
    }

    let last_run = DockerCleanupLastRun {
        at: now,
        results: results.clone(),
    };
    if let Ok(json) = serde_json::to_string(&last_run) {
        if let Err(e) = daemon_settings::set(
            &state.db,
            daemon_settings::KEY_DOCKER_CLEANUP_LAST_RUN,
            &json,
        )
        .await
        {
            warn!(error = %e, "maintenance: falha ao persistir resumo da execução");
        }
    }

    info!(count = results.len(), "maintenance: limpeza concluída");
    state
        .bus
        .publish(Event::DockerCleanupCompleted { at: now, results });

    config
}

fn record(resource: &str, result: Result<PruneStat, String>) -> DockerCleanupResourceResult {
    match result {
        Ok(s) => DockerCleanupResourceResult {
            resource: resource.to_string(),
            count: s.count,
            reclaimed_bytes: s.reclaimed_bytes,
            error: None,
        },
        Err(e) => {
            warn!(resource, error = %e, "maintenance: falha ao limpar recurso");
            DockerCleanupResourceResult {
                resource: resource.to_string(),
                count: 0,
                reclaimed_bytes: 0,
                error: Some(e),
            }
        }
    }
}

pub(crate) async fn save_config(
    state: &AppState,
    config: &DockerCleanupConfig,
) -> anyhow::Result<()> {
    let json = serde_json::to_string(config)?;
    daemon_settings::set(&state.db, daemon_settings::KEY_DOCKER_CLEANUP_CONFIG, &json).await
}

pub(crate) async fn load_config(state: &AppState) -> anyhow::Result<DockerCleanupConfig> {
    let raw = daemon_settings::get(&state.db, daemon_settings::KEY_DOCKER_CLEANUP_CONFIG).await?;
    Ok(match raw {
        Some(json) => serde_json::from_str(&json).unwrap_or_default(),
        None => DockerCleanupConfig::default(),
    })
}

pub(crate) async fn load_last_run(state: &AppState) -> Option<DockerCleanupLastRun> {
    let raw = daemon_settings::get(&state.db, daemon_settings::KEY_DOCKER_CLEANUP_LAST_RUN)
        .await
        .ok()
        .flatten()?;
    serde_json::from_str(&raw).ok()
}
