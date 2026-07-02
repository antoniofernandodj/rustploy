use crate::{api::AppState, db::daemon_settings};
use shared::Response as RpResponse;
use tracing::{error, info, warn};

pub async fn handle(
    state: AppState,
    webhook_base_url: Option<String>,
    acme_email: Option<String>,
) -> RpResponse {
    if let Err(e) = save_optional(
        &state,
        daemon_settings::KEY_WEBHOOK_BASE_URL,
        webhook_base_url,
    )
    .await
    {
        return e;
    }

    let email_trimmed = acme_email
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    if let Err(e) = save_optional(&state, daemon_settings::KEY_ACME_EMAIL, email_trimmed.clone()).await {
        return e;
    }

    match email_trimmed {
        Some(email) => {
            state.tls.enable_acme(email);
            provision_existing_domains(state);
        }
        None => {
            state.tls.disable_acme();
        }
    }

    info!("daemon settings saved");
    RpResponse::Ok
}

/// Emite certificados para todos os services já em execução com tls_enabled.
fn provision_existing_domains(state: AppState) {
    tokio::spawn(async move {
        let services = match crate::db::services::get_running(&state.db).await {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "TLS: falha ao listar services para provisionamento");
                return;
            }
        };

        for svc in services {
            for route in svc.spec.domain_routes().into_iter().filter(|r| r.tls) {
                let tls = state.tls.clone();
                let domain = route.domain.clone();
                tokio::spawn(async move {
                    if let Err(e) = tls.ensure_cert(&domain).await {
                        warn!(domain = %domain, error = %e, "TLS: falha ao provisionar certificado");
                    } else {
                        info!(domain = %domain, "TLS: certificado provisionado");
                    }
                });
            }
        }
    });
}

async fn save_optional(
    state: &AppState,
    key: &str,
    value: Option<String>,
) -> Result<(), RpResponse> {
    match value {
        Some(v) if !v.trim().is_empty() => {
            daemon_settings::set(&state.db, key, v.trim()).await.map_err(|e| {
                error!(error = %e, key, "failed to save daemon setting");
                RpResponse::err("DatabaseError", e.to_string())
            })
        }
        _ => daemon_settings::delete(&state.db, key).await.map_err(|e| {
            error!(error = %e, key, "failed to delete daemon setting");
            RpResponse::err("DatabaseError", e.to_string())
        }),
    }
}
