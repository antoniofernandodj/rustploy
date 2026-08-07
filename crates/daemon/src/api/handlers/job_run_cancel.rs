use crate::api::AppState;
use shared::Response as RpResponse;

/// Sinaliza cancelamento pro `job_run` em `state.active_jobs` (ver
/// `docs/plano-cancelamento-de-jobs.md`) — quem observa o sinal e mata o
/// processo `docker compose up` de verdade é `docker::compose::run_once_up`.
/// Não remove a entrada do mapa: quem faz isso é a própria task ao terminar
/// (normalmente ou por cancelamento), como já acontece em `active_deploys`.
pub async fn handle(state: AppState, job_run_id: String) -> RpResponse {
    let sent = match state.active_jobs.lock() {
        Ok(map) => match map.get(&job_run_id) {
            Some(tx) => tx.send(true).is_ok(),
            None => false,
        },
        Err(_) => false,
    };

    if sent {
        RpResponse::Ok
    } else {
        RpResponse::err(
            "NotFound",
            "job_run não está em execução (já terminou ou o id é inválido)",
        )
    }
}
