//! Leitura e conserto da tabela de rotas do ingress proxy.
//!
//! A tabela vive **em memória** no `IngressController` (um `ArcSwap` lido sem
//! lock pelo proxy). Ela só era escrita no fim de um deploy e no boot, e não
//! havia como olhá-la: um domínio respondendo 502 não tinha diagnóstico
//! possível pela API — nem conserto, a não ser redeployar.
use crate::api::AppState;
use shared::Response as RpResponse;
use tracing::info;

/// Foto da tabela viva (domínios + portas).
pub async fn routes(state: AppState) -> RpResponse {
    RpResponse::IngressRoutes(state.ingress.snapshot())
}

/// Recalcula as rotas a partir dos containers que existem de fato no Docker e
/// devolve a tabela já corrigida.
///
/// `service_id` é aceito para permitir escopo, mas o reconcile do daemon opera
/// sobre todos os serviços de uma vez; o parâmetro serve de filtro de log e
/// mantém a porta aberta para um reconcile por serviço sem trocar o protocolo.
pub async fn reconcile(state: AppState, service_id: Option<String>) -> RpResponse {
    info!(service_id = ?service_id, "ingress: reconcile sob demanda");
    crate::deploy::recovery::reconcile(&state.db, &state.docker, &state.ingress, &state.tls).await;
    RpResponse::IngressRoutes(state.ingress.snapshot())
}
