use crate::{
    api::AppState,
    db::{daemon_settings, webhook_tokens},
};
use shared::Response as RpResponse;

pub async fn handle(state: AppState, service_id: String) -> RpResponse {
    let token = match webhook_tokens::get(&state.db, &service_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return RpResponse::WebhookUrl(None),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    let url = build_url(&state, &service_id, &token).await;
    RpResponse::WebhookUrl(Some(url))
}

pub async fn build_url(state: &AppState, service_id: &str, token: &str) -> String {
    let server_domain = daemon_settings::get(&state.db, daemon_settings::KEY_WEBHOOK_BASE_URL)
        .await
        .ok()
        .flatten();

    match server_domain {
        Some(domain) => {
            let base = domain.trim().trim_end_matches('/');
            format!("{}/webhook/{}/{}", base, service_id, token)
        }
        None => {
            let ip = outbound_ip();
            format!(
                "http://{}:{}/webhook/{}/{}",
                ip, state.webhook_port, service_id, token
            )
        }
    }
}

/// Detecta o IP de saída da máquina conectando um socket UDP em 8.8.8.8:80
/// (sem enviar dados) e lendo o endereço local escolhido pelo kernel.
fn outbound_ip() -> String {
    use std::net::UdpSocket;
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("8.8.8.8:80")?;
            s.local_addr()
        })
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|_| "localhost".to_string())
}
