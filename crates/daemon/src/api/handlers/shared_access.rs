//! `Command::SharedAccess{List,Grant,Revoke}`: quais projetos alcançam um
//! servidor de banco compartilhado (ver `deploy::shared_net`).

use crate::{api::AppState, db, deploy::shared_net};
use shared::{Response as RpResponse, Service, SharedAccess};

async fn server(state: &AppState, id: &str) -> Result<Service, RpResponse> {
    match db::services::get(&state.db, id).await {
        Ok(Some(s)) if s.spec.shared.is_some() => Ok(s),
        Ok(Some(_)) => Err(RpResponse::err(
            "NotShared",
            "este serviço não é um servidor compartilhado",
        )),
        Ok(None) => Err(RpResponse::err("NotFound", "service not found")),
        Err(e) => Err(RpResponse::err("DatabaseError", e.to_string())),
    }
}

async fn view(state: &AppState, svc: &Service) -> Vec<SharedAccess> {
    let projects = db::shared_access::projects_of(&state.db, &svc.id)
        .await
        .unwrap_or_default();
    let alias = shared_net::alias_of(svc);
    let mut out = vec![];
    for project_id in projects {
        let network = db::projects::network_name(&state.db, &project_id)
            .await
            .unwrap_or_default();
        out.push(SharedAccess {
            server_service_id: svc.id.clone(),
            project_id,
            network,
            alias: alias.clone(),
        });
    }
    out
}

pub async fn list(state: AppState, server_service_id: String) -> RpResponse {
    match server(&state, &server_service_id).await {
        Ok(svc) => RpResponse::SharedAccessList(view(&state, &svc).await),
        Err(r) => r,
    }
}

pub async fn grant(state: AppState, server_service_id: String, project_id: String) -> RpResponse {
    let svc = match server(&state, &server_service_id).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    match db::projects::get(&state.db, &project_id).await {
        Ok(Some(_)) => {}
        Ok(None) => return RpResponse::err("NotFound", "project not found"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    }
    if project_id == svc.spec.project_id {
        return RpResponse::err(
            "InvalidRequest",
            "o servidor já está na rede deste projeto; use o nome interno dele",
        );
    }
    // Conecta agora, se o servidor estiver no ar. Se não estiver, o acesso fica
    // gravado e o deploy/reconcile conecta depois. Falha de alias em uso aborta
    // antes de gravar o acesso.
    if let Some(cid) = shared_net::server_container(&state.docker.inner, &svc).await
        && let Err(e) =
            shared_net::connect_to_project(&state.db, &state.docker, &svc, &cid, &project_id).await
    {
        return RpResponse::err("NetworkError", e.to_string());
    }
    if let Err(e) = db::shared_access::grant(&state.db, &svc.id, &project_id).await {
        return RpResponse::err("DatabaseError", e.to_string());
    }
    RpResponse::SharedAccessList(view(&state, &svc).await)
}

pub async fn revoke(state: AppState, server_service_id: String, project_id: String) -> RpResponse {
    let svc = match server(&state, &server_service_id).await {
        Ok(s) => s,
        Err(r) => return r,
    };
    if let Err(e) = db::shared_access::revoke(&state.db, &svc.id, &project_id).await {
        return RpResponse::err("DatabaseError", e.to_string());
    }
    let _ = shared_net::disconnect_from_project(&state.db, &state.docker, &svc, &project_id).await;
    RpResponse::SharedAccessList(view(&state, &svc).await)
}
