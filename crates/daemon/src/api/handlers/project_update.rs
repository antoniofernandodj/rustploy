use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(
    state: AppState,
    id: String,
    name: String,
    description: Option<String>,
) -> RpResponse {
    if name.trim().is_empty() {
        return RpResponse::err("InvalidInput", "nome do projeto não pode ser vazio");
    }
    match crate::db::projects::update(&state.db, &id, name, description).await {
        Ok(Some(p)) => RpResponse::Project(p),
        Ok(None) => RpResponse::err("NotFound", "project not found"),
        Err(e) => RpResponse::err("DatabaseError", super::humanize_db_error(&e, "projeto")),
    }
}
