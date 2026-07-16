use crate::api::AppState;
use shared::{EnvDoc, Response as RpResponse, ServerManifest, ServiceSource};
use std::collections::BTreeMap;
use tracing::{debug, info, warn};

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
    info!(
        provider_count = providers.len(),
        providers = ?providers.values().map(|p| format!("{}={}", p.id, p.name)).collect::<Vec<_>>(),
        "manifest_export_all: git providers carregados do banco"
    );

    // Diagnóstico: resolve aqui mesmo (id -> nome) pra logar ANTES de entrar
    // no `shared::manifest` (format-agnostic, sem tracing) — se um
    // `GitSource.provider_id` não bater com nenhum provider carregado acima
    // (provider deletado/recriado com outro id, por ex.), o export vai OMITIR
    // o campo `provider` no YAML pra esse serviço, silenciosamente, do lado
    // de dentro de `ServerManifest::from_existing_redacted`. Este log é o
    // único jeito de flagrar isso sem instrumentar a crate `shared`.
    for (project, services) in &items {
        for s in services {
            let ServiceSource::Git(g) = &s.spec.source else { continue };
            let Some(pid) = &g.provider_id else { continue };
            match providers.get(pid) {
                Some(p) => debug!(
                    project = %project.name,
                    service = %s.spec.name,
                    provider_id = %pid,
                    provider_name = %p.name,
                    "manifest_export_all: git provider resolvido pra export"
                ),
                None => warn!(
                    project = %project.name,
                    service = %s.spec.name,
                    provider_id = %pid,
                    "manifest_export_all: provider_id referenciado não existe mais no banco — \
                     este serviço vai sair do YAML SEM o campo `provider` (referência órfã)"
                ),
            }
        }
    }

    let (manifest, env_doc): (ServerManifest, EnvDoc) =
        ServerManifest::from_existing_redacted(&items, &providers);

    match serde_yaml::to_string(&manifest) {
        Ok(yaml) => {
            info!(
                yaml_len = yaml.len(),
                dotenv_git_provider_count = env_doc.git_provider.len(),
                dotenv_git_provider_names = ?env_doc.git_provider.keys().collect::<Vec<_>>(),
                "manifest_export_all: exportado"
            );
            RpResponse::ManifestBundle {
                yaml,
                dotenv: shared::format_env_doc(&env_doc),
            }
        }
        Err(e) => RpResponse::err("SerializeError", e.to_string()),
    }
}
