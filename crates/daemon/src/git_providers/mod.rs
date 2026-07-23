//! Clients for hosted Git providers. Gitea and GitHub are implemented; the
//! module is structured so GitLab (etc.) can be added alongside them. The
//! per-provider REST clients live in `gitea`/`github`; the free functions below
//! dispatch on [`shared::GitProviderKind`] so callers stay provider-agnostic.

pub mod gitea;
pub mod github;

use crate::db::git_providers::{self, StoredProvider};
use crate::db::Db;
use crate::secrets::SecretsManager;
use anyhow::{Context, Result};
use shared::{GitAccount, GitBranch, GitProviderKind, GitRepo};

/// Reads a stored provider's kind, defaulting to Gitea for forward-compat with
/// rows written before a kind existed.
fn kind_of(p: &StoredProvider) -> GitProviderKind {
    GitProviderKind::from_str(&p.kind).unwrap_or(GitProviderKind::Gitea)
}

/// OAuth tokens, unified across providers.
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

// ── Provider-agnostic dispatch ────────────────────────────────────────────

pub async fn current_user(kind: GitProviderKind, base_url: &str, token: &str) -> Result<GitAccount> {
    match kind {
        GitProviderKind::Gitea => gitea::current_user(base_url, token).await,
        GitProviderKind::Github => github::current_user(base_url, token).await,
    }
}

pub async fn list_repos(kind: GitProviderKind, base_url: &str, token: &str) -> Result<Vec<GitRepo>> {
    match kind {
        GitProviderKind::Gitea => gitea::list_repos(base_url, token).await,
        GitProviderKind::Github => github::list_repos(base_url, token).await,
    }
}

pub async fn list_branches(
    kind: GitProviderKind,
    base_url: &str,
    token: &str,
    repo_full_name: &str,
) -> Result<Vec<GitBranch>> {
    match kind {
        GitProviderKind::Gitea => gitea::list_branches(base_url, token, repo_full_name).await,
        GitProviderKind::Github => github::list_branches(base_url, token, repo_full_name).await,
    }
}

pub async fn exchange_code(
    kind: GitProviderKind,
    base_url: &str,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<OAuthTokens> {
    match kind {
        GitProviderKind::Gitea => {
            let t = gitea::exchange_code(base_url, client_id, client_secret, code, redirect_uri).await?;
            Ok(OAuthTokens { access_token: t.access_token, refresh_token: t.refresh_token })
        }
        GitProviderKind::Github => {
            let t = github::exchange_code(base_url, client_id, client_secret, code, redirect_uri).await?;
            Ok(OAuthTokens { access_token: t.access_token, refresh_token: t.refresh_token })
        }
    }
}

/// Ensures `redirect_uri` is registered on the provider's OAuth app when the
/// provider supports it (Gitea does; GitHub is a no-op — its callback URLs are
/// not editable via API).
pub async fn ensure_redirect_uri(
    kind: GitProviderKind,
    base_url: &str,
    token: &str,
    client_id: &str,
    redirect_uri: &str,
) -> Result<()> {
    match kind {
        GitProviderKind::Gitea => gitea::ensure_redirect_uri(base_url, token, client_id, redirect_uri).await,
        GitProviderKind::Github => github::ensure_redirect_uri(base_url, token, client_id, redirect_uri).await,
    }
}

/// Builds the browser authorization URL. `client_id`/`redirect_uri`/`state`
/// arrive already percent-encoded.
pub fn authorize_url(
    kind: GitProviderKind,
    base_url: &str,
    client_id_enc: &str,
    redirect_uri_enc: &str,
    state_enc: &str,
) -> String {
    match kind {
        GitProviderKind::Gitea => format!(
            "{}/login/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&state={}",
            base_url.trim_end_matches('/'),
            client_id_enc,
            redirect_uri_enc,
            state_enc,
        ),
        GitProviderKind::Github => {
            github::authorize_url(base_url, client_id_enc, redirect_uri_enc, state_enc)
        }
    }
}

/// Path segment of the OAuth callback for a provider kind — `gitea` or `github`.
/// The daemon serves both under `{public_base}/oauth/{seg}/callback`.
pub fn callback_path_segment(kind: GitProviderKind) -> &'static str {
    kind.as_str()
}

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

    let tokens = match kind_of(p) {
        GitProviderKind::Gitea => {
            let t = gitea::refresh(&p.base_url, client_id, &client_secret, &refresh).await.ok()?;
            OAuthTokens { access_token: t.access_token, refresh_token: t.refresh_token }
        }
        GitProviderKind::Github => {
            let t = github::refresh(&p.base_url, client_id, &client_secret, &refresh).await.ok()?;
            OAuthTokens { access_token: t.access_token, refresh_token: t.refresh_token }
        }
    };

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
