use crate::api::AppState;
use shared::{Response as RpResponse, ServiceSource, ServiceSpec};
use tracing::info;

pub async fn handle(state: AppState, mut spec: ServiceSpec) -> RpResponse {
    // Sentinela 0 → aloca porta externa da faixa configurada; porta manual
    // duplicada → erro antes de persistir.
    if let Err(e) = crate::ports::resolve_host_port(&state.db, &mut spec, None).await {
        return RpResponse::err("PortAllocationError", e);
    }

    info!(
        name = %spec.name,
        project_id = %spec.project_id,
        source = match &spec.source {
            ServiceSource::Registry { image } => format!("registry:{image}"),
            ServiceSource::Git(g) => format!("git:{}", g.url),
            ServiceSource::Archive(a) => format!("archive:{}", a.archive_id),
            ServiceSource::Compose(c) => format!("compose:{}", c.content),
        },
        port = spec.port,
        env_vars = spec.env_vars.len(),
        volumes = spec.volumes.len(),
        "service_create: criando serviço"
    );

    match crate::db::services::create(&state.db, spec).await {
        Ok(s) => {
            info!(service_id = %s.id, name = %s.spec.name, "service_create: serviço criado no banco");
            if let Some(port) = s.spec.host_port {
                crate::firewall::ensure_allowed_bg(port);
            }
            RpResponse::Service(s)
        }
        Err(e) => {
            tracing::error!(error = %e, "service_create: falha ao criar serviço");
            RpResponse::err("DatabaseError", super::humanize_db_error(&e, "serviço"))
        }
    }
}
