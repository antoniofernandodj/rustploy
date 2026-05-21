use crate::{api::AppState, deploy::executor::DeployExecutor};
use shared::{Response as RpResponse, ServiceStatus};
use std::sync::Arc;

pub async fn handle(state: AppState, service_id: String) -> RpResponse {
    let svc = match crate::db::services::get(&state.db, &service_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return RpResponse::err("NotFound", "service not found"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    if matches!(svc.status, ServiceStatus::Deploying) {
        return RpResponse::err("ServiceAlreadyDeploying", "deploy already in progress");
    }

    let image = match &svc.spec.source {
        shared::ServiceSource::Registry { image } => image.clone(),
        shared::ServiceSource::Git(_) => format!("rp_{}", svc.spec.name),
    };

    let dep = match crate::db::deployments::create(&state.db, &service_id, &image).await {
        Ok(d) => d,
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    let _ = crate::db::services::update_status(
        &state.db,
        &service_id,
        &ServiceStatus::Deploying,
        None,
    )
    .await;

    state.bus.publish(shared::Event::ServiceStatusChanged {
        service_id: service_id.clone(),
        status: ServiceStatus::Deploying,
    });

    let executor = Arc::new(DeployExecutor {
        db: state.db.clone(),
        docker: state.docker.clone(),
        ingress: state.ingress.clone(),
        bus: state.bus.clone(),
        secrets: state.secrets.clone(),
        db_path: state.db_path.clone(),
        drain_secs: state.drain_secs,
    });
    let dep_id = dep.id.clone();
    tokio::spawn(async move { executor.run(dep_id).await });

    RpResponse::Deployment(dep)
}
