//! Minimal GitHub API client: OAuth2 token exchange/refresh plus the few REST
//! endpoints the UI needs (current user, repos, branches). Mirrors `gitea.rs`
//! so the dispatch layer in `mod.rs` can treat the two interchangeably.
//!
//! GitHub splits the web host (`https://github.com`, where OAuth authorize/token
//! live) from the API host (`https://api.github.com`). For GitHub Enterprise
//! Server the two share the configured base (`{base}/login/oauth/...` and
//! `{base}/api/v3`). The `base_url` stored for the provider is the **web** host;
//! [`api_base`]/[`web_base`] derive the rest.
//!
//! GitHub accepts `Authorization: Bearer <token>` for both OAuth access tokens
//! and Personal Access Tokens, so the read endpoints are auth-mode agnostic.

use anyhow::{Context, Result};
use serde::Deserialize;
use shared::{GitAccount, GitBranch, GitRepo};

/// Tokens returned by the OAuth token endpoint. Standard OAuth Apps do not issue
/// a refresh token (only GitHub Apps, or OAuth Apps with expiring tokens turned
/// on, do); `refresh_token` is `None` in the common case.
#[derive(Debug, Clone)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

/// Token endpoint reply. With `Accept: application/json` GitHub returns HTTP 200
/// even on errors, carrying `{ "error": "...", "error_description": "..." }`
/// instead of `access_token`, so both shapes are optional here.
#[derive(Deserialize)]
struct TokenResponse {
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct UserResponse {
    login: String,
    #[serde(default)]
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct RepoResponse {
    full_name: String,
    clone_url: String,
    #[serde(default)]
    default_branch: String,
    #[serde(default)]
    private: bool,
}

#[derive(Deserialize)]
struct BranchResponse {
    name: String,
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("rustploy")
        .build()
        .context("falha ao construir HTTP client")
}

fn trim(base_url: &str) -> &str {
    base_url.trim_end_matches('/')
}

/// True when `base_url` points at github.com (the public host) rather than a
/// GitHub Enterprise Server install.
fn is_dotcom(base_url: &str) -> bool {
    let b = trim(base_url);
    b.is_empty() || b == "https://github.com" || b == "http://github.com"
}

/// Web host, where the OAuth authorize/token endpoints live.
pub fn web_base(base_url: &str) -> String {
    if is_dotcom(base_url) {
        "https://github.com".to_string()
    } else {
        trim(base_url).to_string()
    }
}

/// REST API host. github.com uses the dedicated `api.github.com`; Enterprise
/// serves the API under `{base}/api/v3`.
fn api_base(base_url: &str) -> String {
    if is_dotcom(base_url) {
        "https://api.github.com".to_string()
    } else {
        format!("{}/api/v3", trim(base_url))
    }
}

/// Builds the authorization URL the client opens in a browser. `scope=repo`
/// grants read/write to private repos (needed to clone them at deploy time).
/// `redirect_uri`/`state` must arrive already percent-encoded.
pub fn authorize_url(base_url: &str, client_id_enc: &str, redirect_uri_enc: &str, state_enc: &str) -> String {
    format!(
        "{}/login/oauth/authorize?client_id={}&redirect_uri={}&scope=repo&state={}",
        web_base(base_url),
        client_id_enc,
        redirect_uri_enc,
        state_enc,
    )
}

/// Exchanges an authorization `code` for an access token.
pub async fn exchange_code(
    base_url: &str,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<OAuthTokens> {
    let url = format!("{}/login/oauth/access_token", web_base(base_url));
    let resp = client()?
        .post(&url)
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": client_id,
            "client_secret": client_secret,
            "code": code,
            "redirect_uri": redirect_uri,
        }))
        .send()
        .await
        .context("falha ao chamar token endpoint do GitHub")?;
    let resp = error_for_status(resp).await?;
    let t: TokenResponse = resp.json().await.context("token response inválido")?;
    tokens_or_error(t)
}

/// Refreshes an expired access token (only meaningful when expiring OAuth tokens
/// are enabled; standard OAuth Apps never reach here — see `mod.rs`).
pub async fn refresh(
    base_url: &str,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<OAuthTokens> {
    let url = format!("{}/login/oauth/access_token", web_base(base_url));
    let resp = client()?
        .post(&url)
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": client_id,
            "client_secret": client_secret,
            "refresh_token": refresh_token,
        }))
        .send()
        .await
        .context("falha ao renovar token do GitHub")?;
    let resp = error_for_status(resp).await?;
    let mut t: TokenResponse = resp.json().await.context("refresh response inválido")?;
    // Preserva o refresh token anterior se o GitHub não devolver um novo.
    if t.refresh_token.is_none() {
        t.refresh_token = Some(refresh_token.to_string());
    }
    tokens_or_error(t)
}

fn tokens_or_error(t: TokenResponse) -> Result<OAuthTokens> {
    if let Some(err) = t.error {
        let desc = t.error_description.unwrap_or_default();
        anyhow::bail!("GitHub OAuth error {err}: {desc}");
    }
    let access_token = t
        .access_token
        .context("token response sem access_token")?;
    Ok(OAuthTokens {
        access_token,
        refresh_token: t.refresh_token,
    })
}

/// Returns the authenticated account. Doubles as validation for a pasted PAT.
pub async fn current_user(base_url: &str, token: &str) -> Result<GitAccount> {
    let url = format!("{}/user", api_base(base_url));
    let resp = get(&url, token)
        .await
        .context("falha ao consultar usuário no GitHub")?;
    let resp = error_for_status(resp).await?;
    let u: UserResponse = resp.json().await.context("user response inválido")?;
    Ok(GitAccount {
        login: u.login,
        avatar_url: u.avatar_url,
    })
}

/// Lists repositories accessible to the token, paginating until exhausted.
pub async fn list_repos(base_url: &str, token: &str) -> Result<Vec<GitRepo>> {
    let api = api_base(base_url);
    let mut out = Vec::new();
    for page in 1..=20 {
        let url = format!(
            "{api}/user/repos?per_page=100&page={page}&affiliation=owner,collaborator,organization_member"
        );
        let resp = get(&url, token)
            .await
            .context("falha ao listar repositórios no GitHub")?;
        let resp = error_for_status(resp).await?;
        let repos: Vec<RepoResponse> = resp.json().await.context("repos response inválido")?;
        let n = repos.len();
        out.extend(repos.into_iter().map(|r| GitRepo {
            full_name: r.full_name,
            clone_url: r.clone_url,
            default_branch: r.default_branch,
            private: r.private,
        }));
        if n < 100 {
            break;
        }
    }
    Ok(out)
}

/// Lists branches of `owner/repo`.
pub async fn list_branches(
    base_url: &str,
    token: &str,
    repo_full_name: &str,
) -> Result<Vec<GitBranch>> {
    let url = format!(
        "{}/repos/{repo_full_name}/branches?per_page=100",
        api_base(base_url)
    );
    let resp = get(&url, token)
        .await
        .context("falha ao listar branches no GitHub")?;
    let resp = error_for_status(resp).await?;
    let branches: Vec<BranchResponse> =
        resp.json().await.context("branches response inválido")?;
    Ok(branches
        .into_iter()
        .map(|b| GitBranch { name: b.name })
        .collect())
}

/// GitHub OAuth App callback URLs cannot be edited via the API (unlike Gitea),
/// so there is nothing to auto-sync — the user registers the redirect URI in the
/// app's settings by hand. Kept for signature parity with `gitea::`.
pub async fn ensure_redirect_uri(
    _base_url: &str,
    _token: &str,
    _client_id: &str,
    _redirect_uri: &str,
) -> Result<()> {
    Ok(())
}

/// GET with the standard GitHub headers.
async fn get(url: &str, token: &str) -> Result<reqwest::Response> {
    Ok(client()?
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?)
}

/// Turns a non-2xx response into an error carrying the body for diagnostics.
async fn error_for_status(resp: reqwest::Response) -> Result<reqwest::Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().await.unwrap_or_default();
    anyhow::bail!("GitHub respondeu {status}: {body}");
}
