use crate::api::AppState;
use shared::Response as RpResponse;
use tracing::warn;

pub async fn handle(state: AppState, id: String) -> RpResponse {
    match crate::db::job::delete(&state.db, &id).await {
        Ok(true) => {
            // Cascade: qualquer serviço que referenciava este job como
            // pré-deploy check fica com uma referência órfã — sem isso o
            // próximo deploy falha em PreDeployCheck com "job não
            // encontrado" (ver docs/plano-pre-deploy-gate.md).
            if let Err(e) = crate::db::services::clear_pre_deploy_job(&state.db, &id).await {
                warn!(job_id = %id, error = %e, "job_delete: falha ao limpar pre_deploy_job_id órfão em serviços");
            }
            RpResponse::Ok
        }
        Ok(false) => RpResponse::err("NotFound", "job not found"),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
