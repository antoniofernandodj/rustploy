use crate::api::AppState;
use shared::Response;

pub async fn handle(state: AppState, id: String) -> Response {
    match crate::db::git_providers::delete(&state.db, &id).await {
        Ok(true) => Response::Ok,
        Ok(false) => Response::err("NotFound", "Provider não encontrado"),
        Err(e) => Response::err("DatabaseError", e.to_string()),
    }
}
