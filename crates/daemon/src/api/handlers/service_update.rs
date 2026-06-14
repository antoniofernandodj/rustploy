use crate::api::AppState;
use shared::{Response as RpResponse, ServiceSpec};
use tracing::warn;

pub async fn handle(state: AppState, id: String, spec: ServiceSpec) -> RpResponse {
    if spec.tls_enabled {
        if let Some(domain) = spec.domain.clone() {
            let tls = state.tls.clone();
            tokio::spawn(async move {
                if let Err(e) = tls.ensure_cert(&domain).await {
                    warn!(domain = %domain, error = %e, "TLS: falha ao provisionar certificado ao atualizar serviço");
                }
            });
        }
    }

    match crate::db::services::update_spec(&state.db, &id, spec).await {
        Ok(Some(s)) => RpResponse::Service(s),
        Ok(None) => RpResponse::err("NotFound", "service not found"),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
