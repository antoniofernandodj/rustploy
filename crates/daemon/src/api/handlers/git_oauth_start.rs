use crate::api::AppState;
use shared::Response;
use ulid::Ulid;

/// Produces the Gitea authorization URL the client opens in a browser, after
/// stashing a CSRF `state` mapped to this provider for the callback to consume.
pub async fn handle(state: AppState, provider_id: String) -> Response {
    let provider = match crate::db::git_providers::get(&state.db, &provider_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return Response::err("NotFound", "Provider não encontrado"),
        Err(e) => return Response::err("DatabaseError", e.to_string()),
    };

    let client_id = provider.oauth_client_id.clone().unwrap_or_default();
    if client_id.is_empty() {
        return Response::err("InvalidInput", "Provider sem Client ID (OAuth)");
    }

    let redirect_uri = match crate::api::webhook_server::callback_redirect_uri(&state).await {
        Some(u) => u,
        None => {
            return Response::err(
                "ServerDomainMissing",
                "Configure o domínio do servidor (Web Server) antes de conectar via OAuth",
            );
        }
    };

    let csrf = Ulid::new().to_string();
    state
        .oauth_states
        .lock()
        .unwrap()
        .insert(csrf.clone(), provider_id);

    let url = format!(
        "{}/login/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&state={}",
        provider.base_url.trim_end_matches('/'),
        pct(&client_id),
        pct(&redirect_uri),
        pct(&csrf),
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
