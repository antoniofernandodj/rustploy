use crate::api::AppState;
use shared::{ProjectManifest, Response as RpResponse};
use std::collections::BTreeMap;
use tracing::info;

/// Exporta o estado atual de um projeto como manifesto declarativo.
/// Secrets aparecem como `secret:NOME` (nunca o valor decifrado).
pub async fn handle(state: AppState, project_id: String) -> RpResponse {
    info!(%project_id, "manifest_export: exportando projeto");

    let project = match crate::db::projects::get(&state.db, &project_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return RpResponse::err("NotFound", "project not found"),
        Err(e) => {
            tracing::error!(error = %e, "manifest_export: erro ao carregar projeto");
            return RpResponse::err("DatabaseError", e.to_string());
        }
    };

    let services = match crate::db::services::list(&state.db, &project_id).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "manifest_export: erro ao listar serviços");
            return RpResponse::err("DatabaseError", e.to_string());
        }
    };

    let providers: BTreeMap<String, shared::GitProvider> = match crate::db::git_providers::list(&state.db).await {
        Ok(list) => list.into_iter().map(|p| (p.id.clone(), p.to_public())).collect(),
        Err(e) => {
            tracing::error!(error = %e, "manifest_export: erro ao listar git providers");
            return RpResponse::err("DatabaseError", e.to_string());
        }
    };

    let manifest = ProjectManifest::from_existing(&project, &services, &providers);
    match serde_yaml::to_string(&manifest) {
        Ok(yaml) => RpResponse::Manifest(yaml),
        Err(e) => RpResponse::err("SerializeError", e.to_string()),
    }
}
