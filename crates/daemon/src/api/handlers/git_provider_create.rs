use crate::api::AppState;
use crate::db::git_providers::{self, StoredProvider};
use chrono::Utc;
use shared::{GitAuthMode, GitProviderKind, Response};
use ulid::Ulid;

/// Registers a Git provider. For `Pat` mode the token is validated immediately
/// (and the account recorded); for `OAuth` mode the row is created pending the
/// browser authorization handled by `GitOAuthStart` + the callback.
#[allow(clippy::too_many_arguments)]
pub async fn handle(
    state: AppState,
    kind: GitProviderKind,
    name: String,
    base_url: String,
    auth_mode: GitAuthMode,
    oauth_client_id: Option<String>,
    oauth_client_secret: Option<String>,
    pat: Option<String>,
) -> Response {
    let mut base_url = base_url.trim().trim_end_matches('/').to_string();
    // GitHub conecta ao github.com por padrão — a Base URL só é preenchida para
    // GitHub Enterprise Server. Os demais provedores (Gitea) exigem uma URL.
    if base_url.is_empty() {
        if kind == GitProviderKind::Github {
            base_url = "https://github.com".to_string();
        } else {
            return Response::err("InvalidInput", "Base URL obrigatória");
        }
    }

    let id = format!("gp_{}", Ulid::new());
    let client_secret_enc = match &oauth_client_secret {
        Some(s) if !s.is_empty() => match state.secrets.encrypt(s) {
            Ok(e) => Some(e),
            Err(e) => return Response::err("EncryptError", e.to_string()),
        },
        _ => None,
    };

    // PAT mode: validar agora chamando current_user e já gravar como token.
    let (access_token_enc, account_login, account_avatar) = if auth_mode == GitAuthMode::Pat {
        let Some(pat) = pat.filter(|p| !p.is_empty()) else {
            return Response::err("InvalidInput", "Personal Access Token obrigatório");
        };
        match crate::git_providers::current_user(kind, &base_url, &pat).await {
            Ok(acc) => {
                let enc = match state.secrets.encrypt(&pat) {
                    Ok(e) => e,
                    Err(e) => return Response::err("EncryptError", e.to_string()),
                };
                (Some(enc), Some(acc.login), acc.avatar_url)
            }
            Err(e) => return Response::err("GitAuthFailed", e.to_string()),
        }
    } else {
        if oauth_client_id.as_deref().unwrap_or("").is_empty() || client_secret_enc.is_none() {
            return Response::err("InvalidInput", "Client ID e Client Secret obrigatórios");
        }
        (None, None, None)
    };

    let stored = StoredProvider {
        id: id.clone(),
        kind: kind.as_str().to_string(),
        name: name.trim().to_string(),
        base_url,
        auth_mode: auth_mode.as_str().to_string(),
        oauth_client_id,
        oauth_client_secret_enc: client_secret_enc,
        access_token_enc,
        refresh_token_enc: None,
        account_login,
        account_avatar,
        created_at: Utc::now(),
    };

    match git_providers::insert(&state.db, &stored).await {
        Ok(()) => Response::GitProviderInfo(stored.to_public()),
        Err(e) => Response::err("DatabaseError", e.to_string()),
    }
}
