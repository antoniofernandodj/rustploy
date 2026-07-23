use crate::api::AppState;
use shared::Response;
use tracing::warn;
use ulid::Ulid;

/// Produces the provider's authorization URL the client opens in a browser,
/// after stashing a CSRF `state` mapped to this provider for the callback to
/// consume. Works for any provider kind via the `git_providers` dispatch layer.
pub async fn handle(state: AppState, provider_id: String) -> Response {
    let provider = match crate::db::git_providers::get(&state.db, &provider_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return Response::err("NotFound", "Provider não encontrado"),
        Err(e) => return Response::err("DatabaseError", e.to_string()),
    };
    let kind = shared::GitProviderKind::from_str(&provider.kind)
        .unwrap_or(shared::GitProviderKind::Gitea);

    let client_id = provider.oauth_client_id.clone().unwrap_or_default();
    if client_id.is_empty() {
        return Response::err("InvalidInput", "Provider sem Client ID (OAuth)");
    }

    let redirect_uri = match crate::api::public_routes::callback_redirect_uri(&state, kind) {
        Some(u) => u,
        None => {
            return Response::err(
                "ServerDomainMissing",
                "Não foi possível determinar a URL pública do daemon (configure [api] domain)",
            );
        }
    };

    // Se já há um access token armazenado, tenta garantir que a redirect URI
    // atual está registrada no provider — auto-cura após mudança de domínio/porta
    // (no-op no GitHub, que não permite editar a callback URL via API).
    if let Some(enc) = &provider.access_token_enc {
        if let Ok(token) = state.secrets.decrypt(enc) {
            if let Err(e) = crate::git_providers::ensure_redirect_uri(
                kind,
                &provider.base_url,
                &token,
                &client_id,
                &redirect_uri,
            )
            .await
            {
                warn!(error = %e, "oauth: falha ao pré-sincronizar redirect URI (continuando)");
            }
        }
    }

    let csrf = Ulid::new().to_string();
    state
        .oauth_states
        .lock()
        .unwrap()
        .insert(csrf.clone(), provider_id);

    let url = crate::git_providers::authorize_url(
        kind,
        &provider.base_url,
        &pct(&client_id),
        &pct(&redirect_uri),
        &pct(&csrf),
    );
    Response::OAuthUrl(url)
}

/// Percent-encodes a query-string value (RFC 3986 unreserved set kept as-is).
fn pct(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
