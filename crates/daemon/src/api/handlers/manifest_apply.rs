use crate::api::AppState;
use shared::{
    ActionVerb, ApplyReport, ProjectManifest, ResourceAction, ResourceActionKind,
    Response as RpResponse,
};
use tracing::info;

/// Reconcilia uma lista de manifestos YAML de projeto contra o banco.
///
/// Estratégia **aditiva** (nunca deleta): projetos e serviços são casados por
/// **nome**; existentes são atualizados, ausentes são criados. Não dispara deploy.
pub async fn handle(state: AppState, manifests: Vec<String>) -> RpResponse {
    info!(count = manifests.len(), "manifest_apply: reconciliando manifestos");
    let mut report = ApplyReport::default();

    for yaml in manifests {
        let manifest: ProjectManifest = match serde_yaml::from_str(&yaml) {
            Ok(m) => m,
            Err(e) => return RpResponse::err("InvalidManifest", e.to_string()),
        };
        if let Err(resp) = apply_one(&state, manifest, &mut report).await {
            return resp;
        }
    }

    info!(actions = report.actions.len(), "manifest_apply: concluído");
    RpResponse::ManifestReport(report)
}

async fn apply_one(
    state: &AppState,
    manifest: ProjectManifest,
    report: &mut ApplyReport,
) -> Result<(), RpResponse> {
    let (name, description, env_vars) = manifest.project_fields();
    if name.trim().is_empty() {
        return Err(RpResponse::err(
            "InvalidManifest",
            "projeto sem nome no manifesto",
        ));
    }

    // 1. Resolver projeto por nome.
    let existing = crate::db::projects::list(&state.db)
        .await
        .map_err(db_err)?
        .into_iter()
        .find(|p| p.name == name);

    let (project_id, project_action) = match existing {
        Some(p) => {
            crate::db::projects::update(&state.db, &p.id, name.clone(), description)
                .await
                .map_err(db_err)?;
            crate::db::projects::update_env_vars(&state.db, &p.id, env_vars)
                .await
                .map_err(db_err)?;
            (p.id, ActionVerb::Updated)
        }
        None => {
            let created = crate::db::projects::create(&state.db, name.clone(), description)
                .await
                .map_err(db_err)?;
            crate::db::projects::update_env_vars(&state.db, &created.id, env_vars)
                .await
                .map_err(db_err)?;
            (created.id, ActionVerb::Created)
        }
    };

    report.actions.push(ResourceAction {
        kind: ResourceActionKind::Project,
        name: name.clone(),
        action: project_action,
    });

    // 2. Reconciliar serviços por nome dentro do projeto.
    let existing_services = crate::db::services::list(&state.db, &project_id)
        .await
        .map_err(db_err)?;

    for spec in manifest.service_specs(&project_id) {
        let svc_name = spec.name.clone();
        let action = match existing_services.iter().find(|s| s.spec.name == svc_name) {
            Some(s) => {
                crate::db::services::update_spec(&state.db, &s.id, spec)
                    .await
                    .map_err(db_err)?;
                ActionVerb::Updated
            }
            None => {
                crate::db::services::create(&state.db, spec)
                    .await
                    .map_err(db_err)?;
                ActionVerb::Created
            }
        };
        report.actions.push(ResourceAction {
            kind: ResourceActionKind::Service,
            name: format!("{name}/{svc_name}"),
            action,
        });
    }

    Ok(())
}

fn db_err(e: anyhow::Error) -> RpResponse {
    tracing::error!(error = %e, "manifest_apply: erro de banco");
    RpResponse::err("DatabaseError", e.to_string())
}
