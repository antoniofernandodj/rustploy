//! Clients for hosted Git providers. Only Gitea is implemented today; the
//! module is structured so GitHub/GitLab can be added alongside `gitea`.

pub mod gitea;

use crate::db::git_providers::{self, StoredProvider};
use crate::db::Db;
use crate::secrets::SecretsManager;
use anyhow::{Context, Result};

/// Decrypts the access token (OAuth) or PAT a provider authenticates with.
/// Both auth modes store their bearer in `access_token_enc`.
pub fn usable_token(secrets: &SecretsManager, p: &StoredProvider) -> Result<String> {
    let enc = p
        .access_token_enc
        .as_deref()
        .context("provider Git não conectado (sem token)")?;
    secrets.decrypt(enc)
}

/// Attempts to refresh an expired OAuth access token, persisting the new pair.
/// Returns the fresh access token on success. PAT providers (or those without a
/// refresh token) yield `None` — there's nothing to refresh.
pub async fn refresh_access_token(
    db: &Db,
    secrets: &SecretsManager,
    p: &StoredProvider,
) -> Option<String> {
    if p.auth_mode != shared::GitAuthMode::OAuth.as_str() {
        return None;
    }
    let client_id = p.oauth_client_id.as_deref()?;
    let client_secret = secrets.decrypt(p.oauth_client_secret_enc.as_deref()?).ok()?;
    let refresh = secrets.decrypt(p.refresh_token_enc.as_deref()?).ok()?;

    let tokens = gitea::refresh(&p.base_url, client_id, &client_secret, &refresh)
        .await
        .ok()?;

    let access_enc = secrets.encrypt(&tokens.access_token).ok()?;
    let refresh_enc = tokens
        .refresh_token
        .as_deref()
        .and_then(|r| secrets.encrypt(r).ok());
    git_providers::set_tokens(
        db,
        &p.id,
        Some(&access_enc),
        refresh_enc.as_deref(),
        p.account_login.as_deref().unwrap_or_default(),
        p.account_avatar.as_deref(),
    )
    .await
    .ok()?;
    Some(tokens.access_token)
}
