//! Basic auth do registry OCI embutido — checada em TODA rota (inclusive
//! `GET /v2/`), sem bypass mesmo em loopback: o listener em 127.0.0.1 é
//! alcançável por qualquer processo/usuário do host, então não há origem
//! confiável por construção. Ver `docs/plano-registry-embutido.md`.

use base64::Engine;
use hyper::body::Incoming;
use hyper::Request;
use sha2::{Digest, Sha256};

use super::error::RegistryError;
use crate::db::registry_tokens;
use crate::db::Db;

/// Nível de acesso exigido por uma rota — `Push` satisfaz também exigência de
/// `Pull` (um token de escrita pode ler).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Pull,
    Push,
}

fn scope_satisfies(token_scope: &str, required: Scope) -> bool {
    match required {
        Scope::Pull => token_scope == "pull" || token_scope == "push",
        Scope::Push => token_scope == "push",
    }
}

/// Verifica `Authorization: Basic <base64(user:pass)>` contra os tokens
/// cadastrados. O `user` é ignorado — só a senha (o segredo do token) é
/// comparada, hasheada com SHA-256; o `user` só existe porque o esquema Basic
/// exige um par usuário/senha (o `docker login` sempre manda os dois).
pub async fn check(req: &Request<Incoming>, db: &Db, required: Scope) -> Result<(), RegistryError> {
    let unauthorized = || RegistryError::Unauthorized("credenciais ausentes ou inválidas".to_string());

    let header = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(unauthorized)?;
    let b64 = header.strip_prefix("Basic ").ok_or_else(unauthorized)?;
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|_| unauthorized())?;
    let text = String::from_utf8(decoded).map_err(|_| unauthorized())?;
    let (_user, pass) = text.split_once(':').ok_or_else(unauthorized)?;

    let hash = hex::encode(Sha256::digest(pass.as_bytes()));
    let scope = registry_tokens::verify_scope(db, &hash)
        .await?
        .ok_or_else(unauthorized)?;

    if !scope_satisfies(&scope, required) {
        return Err(unauthorized());
    }

    let db = db.clone();
    tokio::spawn(async move {
        let _ = registry_tokens::touch_last_used(&db, &hash).await;
    });

    Ok(())
}
