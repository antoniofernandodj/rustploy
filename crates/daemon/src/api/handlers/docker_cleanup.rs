use crate::api::AppState;
use crate::maintenance;
use chrono::Utc;
use shared::{DockerCleanupConfig, Response as RpResponse};

pub async fn get_config(state: AppState) -> RpResponse {
    let config = match maintenance::run::load_config(&state).await {
        Ok(c) => c,
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };
    let last_run = maintenance::run::load_last_run(&state).await;
    RpResponse::DockerCleanupConfig { config, last_run }
}

pub async fn set_config(state: AppState, mut config: DockerCleanupConfig) -> RpResponse {
    // Recalculado no servidor (não confia no que o cliente mandou) — é o que
    // faz o scheduler saber quando é a próxima janela, sem recalcular a
    // partir de "agora" a cada tick (ver `DockerCleanupConfig::recompute_next_run`).
    config.recompute_next_run(Utc::now());
    if let Err(e) = maintenance::run::save_config(&state, &config).await {
        return RpResponse::err("DatabaseError", e.to_string());
    }
    let last_run = maintenance::run::load_last_run(&state).await;
    RpResponse::DockerCleanupConfig { config, last_run }
}

/// Dispara a limpeza fora do horário agendado (botão "Executar agora"), com
/// os recursos marcados atualmente — independente do interruptor geral
/// `enabled` (é um botão de teste, não liga o agendamento). Roda em
/// background, igual `Command::JobRunNow`: a tela acompanha o resultado pelo
/// `Event::DockerCleanupCompleted`, não pela resposta deste RPC.
pub async fn run_now(state: AppState) -> RpResponse {
    let config = match maintenance::run::load_config(&state).await {
        Ok(c) => c,
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };
    if !config.any_resource_enabled() {
        return RpResponse::err(
            "NoResourceSelected",
            "marque pelo menos um recurso antes de executar",
        );
    }
    tokio::spawn(async move {
        maintenance::run::run(&state, config).await;
    });
    RpResponse::Ok
}
