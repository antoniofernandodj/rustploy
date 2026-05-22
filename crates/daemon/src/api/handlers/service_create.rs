use crate::api::AppState;
use shared::{Response as RpResponse, ServiceSource, ServiceSpec};
use tracing::info;

pub async fn handle(state: AppState, spec: ServiceSpec) -> RpResponse {
    info!(
        name = %spec.name,
        project_id = %spec.project_id,
        source = match &spec.source {
            ServiceSource::Registry { image } => format!("registry:{image}"),
            ServiceSource::Git(g) => format!("git:{}", g.url),
        },
        port = spec.port,
        env_vars = spec.env_vars.len(),
        volumes = spec.volumes.len(),
        "service_create: criando serviço"
    );

    match crate::db::services::create(&state.db, spec).await {
        Ok(s) => {
            info!(service_id = %s.id, name = %s.spec.name, "service_create: serviço criado no banco");
            RpResponse::Service(s)
        }
        Err(e) => {
            tracing::error!(error = %e, "service_create: falha ao criar serviço");
            RpResponse::err("DatabaseError", e.to_string())
        }
    }
}
