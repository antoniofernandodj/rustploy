use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState, id: String) -> RpResponse {
    let mut host_port = None;
    if let Ok(Some(svc)) = crate::db::services::get(&state.db, &id).await {
        state.ingress.remove_domains(&svc.spec);
        if let Some(port) = svc.spec.host_port {
            state.ingress.remove_port_route(port);
            host_port = Some(port);
        }
    }
    match crate::db::services::delete(&state.db, &id).await {
        Ok(true) => {
            // Diretório de um serviço Compose (`<db_path>/compose/<id>`, com
            // os `ComposeFile` materializados). Best-effort: nada aqui é
            // recriável a partir do DB depois que o spec sumiu, mas também
            // nada depende dele — o stack já foi derrubado.
            let compose_dir = state.db_path.join("compose").join(&id);
            if compose_dir.exists() {
                if let Err(e) = std::fs::remove_dir_all(&compose_dir) {
                    tracing::warn!(service_id = %id, error = %e, "service_delete: falha ao remover diretório de compose");
                }
            }
            // Sem regra órfã: com o serviço fora do DB, fecha a porta no
            // firewall (a menos que outro serviço compartilhe a mesma porta).
            if let Some(port) = host_port {
                if !crate::ports::port_in_use_by_other(&state.db, port, Some(&id)).await {
                    crate::firewall::ensure_denied_bg(port);
                }
            }
            // Jobs gatilhados por este serviço vão junto (senão ficariam
            // órfãos, falhando em loop a cada recorrência) — a GUI avisa no
            // diálogo de confirmação antes de chegar aqui.
            match crate::db::job::delete_by_trigger_service(&state.db, &id).await {
                Ok(n) if n > 0 => {
                    tracing::info!(service_id = %id, jobs = n, "service_delete: jobs gatilhados removidos em cascata");
                }
                Err(e) => {
                    tracing::warn!(service_id = %id, error = %e, "service_delete: falha ao remover jobs gatilhados (ficarão órfãos)");
                }
                _ => {}
            }
            RpResponse::Ok
        }
        Ok(false) => RpResponse::err("NotFound", "service not found"),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
