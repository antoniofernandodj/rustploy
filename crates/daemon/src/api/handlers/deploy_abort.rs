use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn handle(state: AppState, deployment_id: String) -> RpResponse {
    let dep = match crate::db::deployments::get(&state.db, &deployment_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return RpResponse::err("NotFound", "deployment not found"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };
    if dep.state.is_terminal() {
        return RpResponse::err("InvalidState", "deployment already finished");
    }
    match crate::db::deployments::transition(
        &state.db,
        &deployment_id,
        &dep.state,
        shared::DeployState::RollingBack,
        Some("aborted by user".into()),
    )
    .await
    {
        Ok(d) => RpResponse::Deployment(d),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
