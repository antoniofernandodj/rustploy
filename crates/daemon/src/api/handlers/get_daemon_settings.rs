use crate::{api::AppState, db::daemon_settings};
use shared::Response as RpResponse;

pub async fn handle(state: AppState) -> RpResponse {
    let acme_email = daemon_settings::get(&state.db, daemon_settings::KEY_ACME_EMAIL)
        .await
        .ok()
        .flatten();

    let registry_domain = daemon_settings::get(&state.db, daemon_settings::KEY_REGISTRY_DOMAIN)
        .await
        .ok()
        .flatten();

    RpResponse::DaemonSettings {
        // Derivada de `[api]`, não persistida: a GUI a exibe (é a base das URLs
        // de webhook e do redirect OAuth), mas não tem como editá-la — muda-se
        // configurando `api.domain`/`api.port` no daemon.
        public_base_url: state.public_base_url(),
        acme_email,
        registry_domain,
    }
}
