use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::Full;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode, body::Incoming};
use hyper_util::rt::TokioIo;
use tracing::{error, info, warn};

use super::AppState;
use crate::db::webhook_tokens;

pub async fn run(state: AppState, port: u16) {
    let addr: std::net::SocketAddr = ([0, 0, 0, 0], port).into();
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!(port, error = %e, "webhook server: falha ao bind");
            return;
        }
    };
    info!(port, "webhook server: escutando");

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "webhook server: accept error");
                continue;
            }
        };
        let state = state.clone();
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            if let Err(e) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, service_fn(move |req| handle(req, state.clone())))
                .await
            {
                warn!(peer = %peer, error = %e, "webhook server: connection error");
            }
        });
    }
}

async fn handle(
    req: Request<Incoming>,
    state: AppState,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // OAuth callback do Gitea (GET) — fora do esquema POST /webhook/...
    if req.method() == Method::GET && req.uri().path() == "/oauth/gitea/callback" {
        return Ok(oauth_callback(req, state).await);
    }

    // Aceita apenas POST /webhook/{service_id}/{token}
    if req.method() != Method::POST {
        return Ok(resp(StatusCode::METHOD_NOT_ALLOWED, "method not allowed"));
    }

    let path = req.uri().path().to_owned();
    let parts: Vec<&str> = path.trim_start_matches('/').splitn(3, '/').collect();
    if parts.len() != 3 || parts[0] != "webhook" {
        return Ok(resp(StatusCode::NOT_FOUND, "not found"));
    }

    let service_id = parts[1].to_string();
    let provided_token = parts[2].to_string();

    let stored = match webhook_tokens::get(&state.db, &service_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return Ok(resp(StatusCode::UNAUTHORIZED, "invalid token")),
        Err(e) => {
            error!(service_id = %service_id, error = %e, "webhook: db error");
            return Ok(resp(StatusCode::INTERNAL_SERVER_ERROR, "internal error"));
        }
    };

    if stored != provided_token {
        return Ok(resp(StatusCode::UNAUTHORIZED, "invalid token"));
    }

    info!(service_id = %service_id, "webhook: disparando deploy");
    tokio::spawn(async move {
        crate::api::handlers::deploy_start::handle(state, service_id).await;
    });

    Ok(resp(StatusCode::OK, "deploy triggered"))
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

/// Completes the Gitea OAuth2 authorization-code flow: validates the CSRF
/// `state`, exchanges the `code` for tokens, records the connected account.
async fn oauth_callback(req: Request<Incoming>, state: AppState) -> Response<Full<Bytes>> {
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

    let redirect_uri = match callback_redirect_uri(&state).await {
        Some(u) => u,
        None => {
            return html(
                StatusCode::BAD_REQUEST,
                "Erro",
                "Configure o domínio do servidor (Web Server) antes de conectar.",
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

/// Builds `{webhook_base_url}/oauth/gitea/callback` from daemon settings.
pub async fn callback_redirect_uri(state: &AppState) -> Option<String> {
    use crate::db::daemon_settings;
    let base = daemon_settings::get(&state.db, daemon_settings::KEY_WEBHOOK_BASE_URL)
        .await
        .ok()
        .flatten()?;
    let base = base.trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    Some(format!("{base}/oauth/gitea/callback"))
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
