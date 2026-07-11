use crate::api::AppState;
use shared::{Response as RpResponse, ServiceSpec};
use tracing::warn;

pub async fn handle(state: AppState, id: String, mut spec: ServiceSpec) -> RpResponse {
    // Porta externa: sentinela 0 → aloca; manual duplicada → erro.
    if let Err(e) = crate::ports::resolve_host_port(&state.db, &mut spec, Some(&id)).await {
        return RpResponse::err("PortAllocationError", e);
    }
    let old_port = match crate::db::services::get(&state.db, &id).await {
        Ok(Some(s)) => s.spec.host_port,
        _ => None,
    };

    for route in spec.domain_routes().into_iter().filter(|r| r.tls) {
        let tls = state.tls.clone();
        let domain = route.domain.clone();
        tokio::spawn(async move {
            if let Err(e) = tls.ensure_cert(&domain).await {
                warn!(domain = %domain, error = %e, "TLS: falha ao provisionar certificado ao atualizar serviço");
            }
        });
    }

    match crate::db::services::update_spec(&state.db, &id, spec).await {
        Ok(Some(s)) => {
            sync_firewall(&state, &s.id, old_port, s.spec.host_port).await;
            RpResponse::Service(s)
        }
        Ok(None) => RpResponse::err("NotFound", "service not found"),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}

/// Porta mudou/removida → fecha a antiga (se mais ninguém a usa) e o listener
/// do ingress; porta nova/mantida → garante a liberação.
async fn sync_firewall(state: &AppState, id: &str, old: Option<u16>, new: Option<u16>) {
    if let Some(old_port) = old.filter(|o| Some(*o) != new) {
        state.ingress.remove_port_route(old_port);
        if !crate::ports::port_in_use_by_other(&state.db, old_port, Some(id)).await {
            crate::firewall::ensure_denied_bg(old_port);
        }
    }
    if let Some(new_port) = new {
        crate::firewall::ensure_allowed_bg(new_port);
    }
}
