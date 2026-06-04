use crate::api::AppState;
use shared::Response;

pub async fn handle(
    state: AppState,
    project_id: String,
    name: String,
    value: String,
) -> Response {
    match state.secrets.set(&project_id, &name, &value).await {
        Ok(()) => Response::Ok,
        Err(e) => Response::err("SecretSetFailed", e.to_string()),
    }
}
