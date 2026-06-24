//! Persistence for connected Git providers (Gitea OAuth2 / PAT).
//!
//! Secrets (OAuth client secret, access/refresh tokens, PAT) are stored already
//! encrypted by the caller via `SecretsManager`; this module only moves opaque
//! strings in and out of SQLite. The public [`shared::GitProvider`] projection
//! deliberately omits every secret column.

use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use shared::{GitAccount, GitAuthMode, GitProvider, GitProviderKind};

/// Full row as stored, including the encrypted secret columns. Used internally
/// by the daemon (e.g. the deploy executor and OAuth callback); never sent to
/// clients — call [`StoredProvider::to_public`] for that.
#[derive(Debug, Clone)]
pub struct StoredProvider {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub base_url: String,
    pub auth_mode: String,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret_enc: Option<String>,
    pub access_token_enc: Option<String>,
    pub refresh_token_enc: Option<String>,
    pub account_login: Option<String>,
    pub account_avatar: Option<String>,
    pub created_at: DateTime<Utc>,
}

type Row = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);

const COLS: &str = "id, kind, name, base_url, auth_mode, oauth_client_id, \
    oauth_client_secret_enc, access_token_enc, refresh_token_enc, \
    account_login, account_avatar, created_at";

fn row_to_stored(r: Row) -> Result<StoredProvider> {
    Ok(StoredProvider {
        id: r.0,
        kind: r.1,
        name: r.2,
        base_url: r.3,
        auth_mode: r.4,
        oauth_client_id: r.5,
        oauth_client_secret_enc: r.6,
        access_token_enc: r.7,
        refresh_token_enc: r.8,
        account_login: r.9,
        account_avatar: r.10,
        created_at: DateTime::parse_from_rfc3339(&r.11)?.with_timezone(&Utc),
    })
}

impl StoredProvider {
    /// Client-facing projection. Drops every secret column.
    pub fn to_public(&self) -> GitProvider {
        let account = self.account_login.as_ref().map(|login| GitAccount {
            login: login.clone(),
            avatar_url: self.account_avatar.clone(),
        });
        GitProvider {
            id: self.id.clone(),
            kind: GitProviderKind::from_str(&self.kind).unwrap_or(GitProviderKind::Gitea),
            name: self.name.clone(),
            base_url: self.base_url.clone(),
            auth_mode: GitAuthMode::from_str(&self.auth_mode).unwrap_or(GitAuthMode::OAuth),
            oauth_client_id: self.oauth_client_id.clone(),
            account,
            created_at: self.created_at,
        }
    }
}

pub async fn insert(db: &Db, p: &StoredProvider) -> Result<()> {
    sqlx::query(
        "INSERT INTO git_provider (id, kind, name, base_url, auth_mode, oauth_client_id, \
         oauth_client_secret_enc, access_token_enc, refresh_token_enc, account_login, \
         account_avatar, created_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",
    )
    .bind(&p.id)
    .bind(&p.kind)
    .bind(&p.name)
    .bind(&p.base_url)
    .bind(&p.auth_mode)
    .bind(&p.oauth_client_id)
    .bind(&p.oauth_client_secret_enc)
    .bind(&p.access_token_enc)
    .bind(&p.refresh_token_enc)
    .bind(&p.account_login)
    .bind(&p.account_avatar)
    .bind(p.created_at.to_rfc3339())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn list(db: &Db) -> Result<Vec<StoredProvider>> {
    let q = format!("SELECT {COLS} FROM git_provider ORDER BY created_at ASC");
    let rows = sqlx::query_as::<_, Row>(&q).fetch_all(db).await?;
    rows.into_iter().map(row_to_stored).collect()
}

pub async fn get(db: &Db, id: &str) -> Result<Option<StoredProvider>> {
    let q = format!("SELECT {COLS} FROM git_provider WHERE id = ?");
    let row = sqlx::query_as::<_, Row>(&q)
        .bind(id)
        .fetch_optional(db)
        .await?;
    row.map(row_to_stored).transpose()
}

/// Records the connected account and its tokens once OAuth completes (or a PAT
/// validates). Tokens come in already encrypted.
pub async fn set_tokens(
    db: &Db,
    id: &str,
    access_token_enc: Option<&str>,
    refresh_token_enc: Option<&str>,
    account_login: &str,
    account_avatar: Option<&str>,
) -> Result<()> {
    sqlx::query(
        "UPDATE git_provider SET access_token_enc = ?, refresh_token_enc = ?, \
         account_login = ?, account_avatar = ? WHERE id = ?",
    )
    .bind(access_token_enc)
    .bind(refresh_token_enc)
    .bind(account_login)
    .bind(account_avatar)
    .bind(id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete(db: &Db, id: &str) -> Result<bool> {
    let n = sqlx::query("DELETE FROM git_provider WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?
        .rows_affected();
    Ok(n > 0)
}
