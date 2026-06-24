//! Minimal Gitea API client: OAuth2 token exchange/refresh plus the few REST
//! endpoints the UI needs (current user, repos, branches).
//!
//! Gitea accepts `Authorization: token <token>` for both OAuth access tokens
//! and Personal Access Tokens, so the read endpoints are auth-mode agnostic.

use anyhow::{Context, Result};
use serde::Deserialize;
use shared::{GitAccount, GitBranch, GitRepo};

/// Tokens returned by the OAuth token endpoint.
#[derive(Debug, Clone)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
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

fn base(base_url: &str) -> &str {
    base_url.trim_end_matches('/')
}

/// Exchanges an authorization `code` for access/refresh tokens.
pub async fn exchange_code(
    base_url: &str,
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<OAuthTokens> {
    let url = format!("{}/login/oauth/access_token", base(base_url));
    let resp = client()?
        .post(&url)
        .json(&serde_json::json!({
            "grant_type": "authorization_code",
            "client_id": client_id,
            "client_secret": client_secret,
            "code": code,
            "redirect_uri": redirect_uri,
        }))
        .send()
        .await
        .context("falha ao chamar token endpoint do Gitea")?;
    let resp = error_for_status(resp).await?;
    let t: TokenResponse = resp.json().await.context("token response inválido")?;
    Ok(OAuthTokens {
        access_token: t.access_token,
        refresh_token: t.refresh_token,
    })
}

/// Refreshes an expired access token.
pub async fn refresh(
    base_url: &str,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<OAuthTokens> {
    let url = format!("{}/login/oauth/access_token", base(base_url));
    let resp = client()?
        .post(&url)
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "client_id": client_id,
            "client_secret": client_secret,
            "refresh_token": refresh_token,
        }))
        .send()
        .await
        .context("falha ao renovar token do Gitea")?;
    let resp = error_for_status(resp).await?;
    let t: TokenResponse = resp.json().await.context("refresh response inválido")?;
    Ok(OAuthTokens {
        access_token: t.access_token,
        refresh_token: t.refresh_token.or_else(|| Some(refresh_token.to_string())),
    })
}

/// Returns the authenticated account. Doubles as validation for a pasted PAT.
pub async fn current_user(base_url: &str, token: &str) -> Result<GitAccount> {
    let url = format!("{}/api/v1/user", base(base_url));
    let resp = client()?
        .get(&url)
        .header("Authorization", format!("token {token}"))
        .send()
        .await
        .context("falha ao consultar usuário no Gitea")?;
    let resp = error_for_status(resp).await?;
    let u: UserResponse = resp.json().await.context("user response inválido")?;
    Ok(GitAccount {
        login: u.login,
        avatar_url: u.avatar_url,
    })
}

/// Lists repositories accessible to the token, paginating until exhausted.
pub async fn list_repos(base_url: &str, token: &str) -> Result<Vec<GitRepo>> {
    let c = client()?;
    let mut out = Vec::new();
    for page in 1..=20 {
        let url = format!(
            "{}/api/v1/user/repos?limit=50&page={page}",
            base(base_url)
        );
        let resp = c
            .get(&url)
            .header("Authorization", format!("token {token}"))
            .send()
            .await
            .context("falha ao listar repositórios no Gitea")?;
        let resp = error_for_status(resp).await?;
        let repos: Vec<RepoResponse> = resp.json().await.context("repos response inválido")?;
        let n = repos.len();
        out.extend(repos.into_iter().map(|r| GitRepo {
            full_name: r.full_name,
            clone_url: r.clone_url,
            default_branch: r.default_branch,
            private: r.private,
        }));
        if n < 50 {
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
        "{}/api/v1/repos/{repo_full_name}/branches?limit=100",
        base(base_url)
    );
    let resp = client()?
        .get(&url)
        .header("Authorization", format!("token {token}"))
        .send()
        .await
        .context("falha ao listar branches no Gitea")?;
    let resp = error_for_status(resp).await?;
    let branches: Vec<BranchResponse> =
        resp.json().await.context("branches response inválido")?;
    Ok(branches
        .into_iter()
        .map(|b| GitBranch { name: b.name })
        .collect())
}

/// Turns a non-2xx response into an error carrying the body for diagnostics.
async fn error_for_status(resp: reqwest::Response) -> Result<reqwest::Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let body = resp.text().await.unwrap_or_default();
    anyhow::bail!("Gitea respondeu {status}: {body}");
}
