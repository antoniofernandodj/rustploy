use crate::api::AppState;
use shared::Response as RpResponse;
use tracing::info;

pub async fn handle(state: AppState, deployment_id: String) -> RpResponse {
    // Recusa apagar um deployment activo (não terminal)
    match crate::db::deployments::get(&state.db, &deployment_id).await {
        Ok(Some(dep)) if !dep.state.is_terminal() => {
            return RpResponse::err("DEPLOY_ACTIVE", "Não é possível apagar um deployment em andamento.");
        }
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
        _ => {}
    }

    // Apaga logs de build do SQLite
    if let Err(e) = crate::db::build_logs::delete_for_deployment(&state.db, &deployment_id).await {
        return RpResponse::err("DatabaseError", e.to_string());
    }

    // Apaga o registo do deployment
    if let Err(e) = crate::db::deployments::delete(&state.db, &deployment_id).await {
        return RpResponse::err("DatabaseError", e.to_string());
    }

    // Apaga artefactos de build em disco (se existirem)
    let build_dir = state.db_path.join("builds").join(&deployment_id);
    if build_dir.exists() {
        let _ = tokio::fs::remove_dir_all(&build_dir).await;
    }

    info!(deployment_id, "deployment deleted");
    RpResponse::Ok
}
