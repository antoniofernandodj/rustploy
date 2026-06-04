use crate::api::AppState;
use shared::Response;

pub async fn handle(state: AppState, project_id: String) -> Response {
    match state.secrets.list_names(&project_id).await {
        Ok(names) => Response::SecretNames(names),
        Err(e) => Response::err("SecretListFailed", e.to_string()),
    }
}
