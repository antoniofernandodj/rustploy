use crate::api::AppState;
use shared::Response;

pub async fn handle(state: AppState, provider_id: String) -> Response {
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
    match crate::git_providers::list_repos(kind, &provider.base_url, &token).await {
        Ok(repos) => Response::GitRepos(repos),
        Err(first) => {
            // Token possivelmente expirado: tenta um refresh OAuth e repete.
            match crate::git_providers::refresh_access_token(&state.db, &state.secrets, &provider).await {
                Some(fresh) => match crate::git_providers::list_repos(kind, &provider.base_url, &fresh).await {
                    Ok(repos) => Response::GitRepos(repos),
                    Err(e) => Response::err("GitApiError", e.to_string()),
                },
                None => Response::err("GitApiError", first.to_string()),
            }
        }
    }
}
