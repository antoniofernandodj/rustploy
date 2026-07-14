//! Rotas HTTP **públicas** (sem Bearer): o webhook de deploy e o callback OAuth
//! do Gitea. Não há listener próprio aqui — desde a unificação das portas
//! (`docs/plano-unificacao-webhook-api.md`) as duas rotas são servidas pelo
//! listener da API (`http_api.rs`), que as roteia para cá **antes** do gate de
//! token, porque cada uma tem autenticação própria: o webhook valida o token de
//! 192 bits que vem na URL; o callback valida o `state` CSRF emitido no início
//! do fluxo OAuth.

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Method, Request, Response, StatusCode, body::Incoming};
use tracing::{error, info, warn};

use super::AppState;
use crate::db::webhook_tokens;

/// `POST /webhook/{service_id}/{token}` — valida o token e dispara um deploy.
/// O corpo da requisição é ignorado (ver `docs/webhooks.md`); a autenticação é
/// inteiramente o token na URL. Método e path chegam já roteados pelo `http_api`.
///
/// Todo método é roteado para cá (não só POST) para que um GET — a URL colada no
/// navegador, o "ping" de um provedor — receba o `405` honesto do webhook, e não
/// o `401 unauthorized` do gate de Bearer da API, que faria parecer que a URL
/// está errada.
pub async fn webhook(method: &Method, path: &str, state: AppState) -> Response<Full<Bytes>> {
    if method != Method::POST {
        return resp(StatusCode::METHOD_NOT_ALLOWED, "method not allowed");
    }

    let parts: Vec<&str> = path.trim_start_matches('/').splitn(3, '/').collect();
    if parts.len() != 3 || parts[0] != "webhook" {
        return resp(StatusCode::NOT_FOUND, "not found");
    }

    let service_id = parts[1].to_string();
    let provided_token = parts[2];

    let stored = match webhook_tokens::get(&state.db, &service_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return resp(StatusCode::UNAUTHORIZED, "invalid token"),
        Err(e) => {
            error!(service_id = %service_id, error = %e, "webhook: db error");
            return resp(StatusCode::INTERNAL_SERVER_ERROR, "internal error");
        }
    };

    // Comparação em tempo constante: o token é o único segredo do endpoint.
    if !constant_time_eq(stored.as_bytes(), provided_token.as_bytes()) {
        return resp(StatusCode::UNAUTHORIZED, "invalid token");
    }

    info!(service_id = %service_id, "webhook: disparando deploy");
    tokio::spawn(async move {
        crate::api::handlers::deploy_start::handle(state, service_id).await;
    });

    resp(StatusCode::OK, "deploy triggered")
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn resp(status: StatusCode, body: &'static str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

fn html(status: StatusCode, title: &str, body: &str) -> Response<Full<Bytes>> {
    let page = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title>\
         <style>body{{font-family:system-ui,sans-serif;background:#1b1d22;color:#e6eaf0;\
         display:flex;align-items:center;justify-content:center;height:100vh;margin:0}}\
         .card{{text-align:center;padding:32px 48px;border-radius:12px;background:#23262d}}\
         h1{{color:#4ccfe6;font-size:20px}}p{{color:#9aa0a8}}</style></head>\
         <body><div class=\"card\"><h1>{title}</h1><p>{body}</p></div></body></html>"
    );
    Response::builder()
        .status(status)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from(page)))
        .unwrap()
}

/// `GET /oauth/gitea/callback` — completes the Gitea OAuth2 authorization-code
/// flow: validates the CSRF `state`, exchanges the `code` for tokens, records
/// the connected account.
pub async fn oauth_callback(req: Request<Incoming>, state: AppState) -> Response<Full<Bytes>> {
    use crate::db::git_providers;

    let query = req.uri().query().unwrap_or("");
    let mut code = None;
    let mut csrf = None;
    for (k, v) in url_decode_pairs(query) {
        match k.as_str() {
            "code" => code = Some(v),
            "state" => csrf = Some(v),
            _ => {}
        }
    }
    let (Some(code), Some(csrf)) = (code, csrf) else {
        return html(StatusCode::BAD_REQUEST, "Erro", "Parâmetros OAuth ausentes.");
    };

    // Consome o state (CSRF) e recupera o provider associado.
    let provider_id = match state.oauth_states.lock().unwrap().remove(&csrf) {
        Some(id) => id,
        None => return html(StatusCode::BAD_REQUEST, "Erro", "State OAuth inválido ou expirado."),
    };

    let provider = match git_providers::get(&state.db, &provider_id).await {
        Ok(Some(p)) => p,
        _ => return html(StatusCode::NOT_FOUND, "Erro", "Provider não encontrado."),
    };

    let client_id = provider.oauth_client_id.clone().unwrap_or_default();
    let client_secret = match &provider.oauth_client_secret_enc {
        Some(enc) => match state.secrets.decrypt(enc) {
            Ok(s) => s,
            Err(e) => {
                error!(error = %e, "oauth: falha ao decifrar client_secret");
                return html(StatusCode::INTERNAL_SERVER_ERROR, "Erro", "Falha ao ler client secret.");
            }
        },
        None => return html(StatusCode::BAD_REQUEST, "Erro", "Provider sem client secret."),
    };

    let redirect_uri = match callback_redirect_uri(&state) {
        Some(u) => u,
        None => {
            return html(
                StatusCode::BAD_REQUEST,
                "Erro",
                "Não foi possível determinar a URL pública do daemon (configure [api] domain).",
            );
        }
    };

    let tokens = match crate::git_providers::gitea::exchange_code(
        &provider.base_url,
        &client_id,
        &client_secret,
        &code,
        &redirect_uri,
    )
    .await
    {
        Ok(t) => t,
        Err(e) => {
            error!(error = %e, "oauth: troca de code falhou");
            return html(StatusCode::BAD_GATEWAY, "Erro", "Falha ao trocar o código por token.");
        }
    };

    // Auto-registra a redirect URI atual no app OAuth2 do Gitea para que
    // futuras trocas funcionem mesmo após mudança do domínio/porta da API.
    {
        let base_url = provider.base_url.clone();
        let token = tokens.access_token.clone();
        let cid = client_id.clone();
        let ru = redirect_uri.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::git_providers::gitea::ensure_redirect_uri(
                &base_url, &token, &cid, &ru,
            )
            .await
            {
                warn!(error = %e, "oauth: falha ao sincronizar redirect URI no Gitea (não crítico)");
            }
        });
    }

    let account = match crate::git_providers::gitea::current_user(&provider.base_url, &tokens.access_token).await {
        Ok(a) => a,
        Err(e) => {
            error!(error = %e, "oauth: current_user falhou");
            return html(StatusCode::BAD_GATEWAY, "Erro", "Token obtido, mas falha ao ler a conta.");
        }
    };

    let access_enc = state.secrets.encrypt(&tokens.access_token).ok();
    let refresh_enc = tokens
        .refresh_token
        .as_deref()
        .and_then(|r| state.secrets.encrypt(r).ok());

    if let Err(e) = git_providers::set_tokens(
        &state.db,
        &provider_id,
        access_enc.as_deref(),
        refresh_enc.as_deref(),
        &account.login,
        account.avatar_url.as_deref(),
    )
    .await
    {
        error!(error = %e, "oauth: falha ao persistir tokens");
        return html(StatusCode::INTERNAL_SERVER_ERROR, "Erro", "Falha ao salvar a conexão.");
    }

    info!(provider_id = %provider_id, login = %account.login, "oauth: conta Gitea conectada");
    html(
        StatusCode::OK,
        "Conta conectada",
        &format!("Gitea @{} conectado. Você já pode fechar esta aba.", account.login),
    )
}

/// Builds `{public_base_url}/oauth/gitea/callback` — a base sai de `[api]`
/// (domínio/porta do listener que atende este callback), não mais de uma setting
/// no banco. `Option` mantido porque o chamador ainda trata o caso "sem base
/// utilizável"; hoje ela é sempre derivável.
pub fn callback_redirect_uri(state: &AppState) -> Option<String> {
    Some(format!("{}/oauth/gitea/callback", state.public_base_url()))
}

/// Tiny `application/x-www-form-urlencoded` query parser (percent-decoding).
fn url_decode_pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|s| !s.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(k), percent_decode(v))
        })
        .collect()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push((hi * 16 + lo) as u8);
                    i += 3;
                    continue;
                }
                out.push(b'%');
                i += 1;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
