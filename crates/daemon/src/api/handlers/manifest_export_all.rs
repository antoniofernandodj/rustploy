use crate::api::AppState;
use shared::{Response as RpResponse, ServerManifest};
use std::collections::BTreeMap;
use tracing::info;

/// Exporta TODOS os projetos+serviços como um único manifesto raiz, com todo
/// valor de env var `Plain` redigido para `${KEY}` (nunca o valor real no
/// YAML) e o `.env` complementar com esses valores reais. Secrets seguem como
/// `secret:NOME`, nunca decifradas.
pub async fn handle(state: AppState) -> RpResponse {
    info!("manifest_export_all: exportando todos os projetos");

    let projects = match crate::db::projects::list(&state.db).await {
        Ok(p) => p,
        Err(e) => {
            tracing::error!(error = %e, "manifest_export_all: erro ao listar projetos");
            return RpResponse::err("DatabaseError", e.to_string());
        }
    };

    let mut items = Vec::with_capacity(projects.len());
    for project in projects {
        let services = match crate::db::services::list(&state.db, &project.id).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(error = %e, project_id = %project.id, "manifest_export_all: erro ao listar serviços");
                return RpResponse::err("DatabaseError", e.to_string());
            }
        };
        items.push((project, services));
    }

    let (manifest, dotenv): (ServerManifest, BTreeMap<String, String>) =
        ServerManifest::from_existing_redacted(&items);

    match serde_yaml::to_string(&manifest) {
        Ok(yaml) => RpResponse::ManifestBundle {
            yaml,
            dotenv: shared::format_dotenv(&dotenv),
        },
        Err(e) => RpResponse::err("SerializeError", e.to_string()),
    }
}
