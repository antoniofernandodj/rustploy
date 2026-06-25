use crate::api::AppState;
use shared::Response;

pub async fn handle(state: AppState) -> Response {
    match crate::db::git_providers::list(&state.db).await {
        Ok(ps) => Response::GitProviders(ps.iter().map(|p| p.to_public()).collect()),
        Err(e) => Response::err("DatabaseError", e.to_string()),
    }
}
