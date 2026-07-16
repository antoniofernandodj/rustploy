use crate::api::AppState;
use shared::{EnvDoc, Response as RpResponse, ServerManifest};
use std::collections::BTreeMap;
use tracing::info;

/// Exporta TODOS os projetos+serviços como um único manifesto raiz, com todo
/// valor de env var `Plain` redigido para `${KEY}` (nunca o valor real no
/// YAML) e o TOML complementar (`EnvDoc`, aninhado por projeto → serviço) com
/// esses valores reais. Secrets seguem como `secret:NOME`, nunca decifradas.
/// Serviços `git` que usam um provider conectado (Gitea) referenciam-no pelo
/// nome no YAML; os dados não-secretos do provider (URL, modo de auth, client
/// id) vão para o mesmo TOML — ver [`shared::GitProviderDoc`].
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

    let providers: BTreeMap<String, shared::GitProvider> = match crate::db::git_providers::list(&state.db).await {
        Ok(list) => list.into_iter().map(|p| (p.id.clone(), p.to_public())).collect(),
        Err(e) => {
            tracing::error!(error = %e, "manifest_export_all: erro ao listar git providers");
            return RpResponse::err("DatabaseError", e.to_string());
        }
    };

    let (manifest, env_doc): (ServerManifest, EnvDoc) =
        ServerManifest::from_existing_redacted(&items, &providers);

    match serde_yaml::to_string(&manifest) {
        Ok(yaml) => RpResponse::ManifestBundle {
            yaml,
            dotenv: shared::format_env_doc(&env_doc),
        },
        Err(e) => RpResponse::err("SerializeError", e.to_string()),
    }
}
