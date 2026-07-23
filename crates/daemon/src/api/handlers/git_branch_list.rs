use crate::api::AppState;
use shared::Response;

pub async fn handle(state: AppState, provider_id: String, repo_full_name: String) -> Response {
    let provider = match crate::db::git_providers::get(&state.db, &provider_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return Response::err("NotFound", "Provider não encontrado"),
        Err(e) => return Response::err("DatabaseError", e.to_string()),
    };
    let token = match crate::git_providers::usable_token(&state.secrets, &provider) {
        Ok(t) => t,
        Err(e) => return Response::err("NotConnected", e.to_string()),
    };
    let kind = shared::GitProviderKind::from_str(&provider.kind)
        .unwrap_or(shared::GitProviderKind::Gitea);
    match crate::git_providers::list_branches(kind, &provider.base_url, &token, &repo_full_name).await {
        Ok(branches) => Response::GitBranches(branches),
        Err(first) => {
            match crate::git_providers::refresh_access_token(&state.db, &state.secrets, &provider).await {
                Some(fresh) => match crate::git_providers::list_branches(kind, &provider.base_url, &fresh, &repo_full_name).await {
                    Ok(branches) => Response::GitBranches(branches),
                    Err(e) => Response::err("GitApiError", e.to_string()),
                },
                None => Response::err("GitApiError", first.to_string()),
            }
        }
    }
}
