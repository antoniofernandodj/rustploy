use crate::api::AppState;
use shared::Response;

pub async fn handle(state: AppState, project_id: String, name: String) -> Response {
    match state.secrets.delete(&project_id, &name).await {
        Ok(()) => Response::Ok,
        Err(e) => Response::err("SecretDeleteFailed", e.to_string()),
    }
}
