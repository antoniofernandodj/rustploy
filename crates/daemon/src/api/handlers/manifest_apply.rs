use crate::api::AppState;
use shared::{
    ActionVerb, ApplyReport, ProjectManifest, ResourceAction, ResourceActionKind,
    Response as RpResponse,
};
use tracing::{info, warn};

/// Reconcilia uma lista de manifestos YAML de projeto contra o banco.
///
/// Casamento por **nome**. Por padrão é **aditivo** (cria/atualiza); com `prune`
/// também remove serviços ausentes do manifesto. Com `deploy`, dispara o deploy
/// dos serviços criados/alterados depois de sincronizar.
pub async fn handle(
    state: AppState,
    manifests: Vec<String>,
    prune: bool,
    deploy: bool,
) -> RpResponse {
    info!(
        count = manifests.len(),
        prune, deploy, "manifest_apply: reconciliando manifestos"
    );
    let mut report = ApplyReport::default();
    // (service_id, "projeto/serviço") dos serviços criados/alterados.
    let mut changed: Vec<(String, String)> = Vec::new();

    for yaml in manifests {
        let manifest: ProjectManifest = match serde_yaml::from_str(&yaml) {
            Ok(m) => m,
            Err(e) => return RpResponse::err("InvalidManifest", e.to_string()),
        };
        if let Err(resp) = apply_one(&state, manifest, prune, &mut report, &mut changed).await {
            return resp;
        }
    }

    // Deploy dos serviços alterados — feito por último, após todas as escritas.
    if deploy {
        for (service_id, label) in &changed {
            match super::deploy_start::handle(state.clone(), service_id.clone()).await {
                RpResponse::Err { code, message } => {
                    warn!(%label, %code, %message, "manifest_apply: deploy falhou ao iniciar");
                }
                _ => report.deployed.push(label.clone()),
            }
        }
    }

    info!(
        actions = report.actions.len(),
        deployed = report.deployed.len(),
        "manifest_apply: concluído"
    );
    RpResponse::ManifestReport(report)
}

async fn apply_one(
    state: &AppState,
    manifest: ProjectManifest,
    prune: bool,
    report: &mut ApplyReport,
    changed: &mut Vec<(String, String)>,
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
        Some(p) if p.description == description => {
            // Só actualiza env vars se o manifesto declara pelo menos uma.
            // Lista vazia significa "manifesto sem env vars" (ex: export sem secrets),
            // não "apagar tudo" — preservar as existentes nesse caso.
            if !env_vars.is_empty() && p.env_vars != env_vars {
                crate::db::projects::update(&state.db, &p.id, name.clone(), description)
                    .await
                    .map_err(db_err)?;
                crate::db::projects::update_env_vars(&state.db, &p.id, env_vars, Vec::new())
                    .await
                    .map_err(db_err)?;
                (p.id, ActionVerb::Updated)
            } else {
                (p.id, ActionVerb::Unchanged)
            }
        }
        Some(p) => {
            crate::db::projects::update(&state.db, &p.id, name.clone(), description)
                .await
                .map_err(db_err)?;
            if !env_vars.is_empty() {
                crate::db::projects::update_env_vars(&state.db, &p.id, env_vars, Vec::new())
                    .await
                    .map_err(db_err)?;
            }
            (p.id, ActionVerb::Updated)
        }
        None => {
            let created = crate::db::projects::create(&state.db, name.clone(), description)
                .await
                .map_err(db_err)?;
            if !env_vars.is_empty() {
                crate::db::projects::update_env_vars(&state.db, &created.id, env_vars, Vec::new())
                    .await
                    .map_err(db_err)?;
            }
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

    let specs = manifest.service_specs(&project_id);
    for spec in &specs {
        let svc_name = spec.name.clone();
        let label = format!("{name}/{svc_name}");
        let (action, changed_id) = match existing_services.iter().find(|s| s.spec.name == svc_name) {
            Some(s) if &s.spec == spec => (ActionVerb::Unchanged, None),
            Some(s) => {
                // Se o manifesto não declara env vars, preservar as existentes para não
                // apagar vars que foram definidas manualmente após o export.
                let mut effective_spec = spec.clone();
                if effective_spec.env_vars.is_empty() && !s.spec.env_vars.is_empty() {
                    effective_spec.env_vars = s.spec.env_vars.clone();
                }
                crate::db::services::update_spec(&state.db, &s.id, effective_spec)
                    .await
                    .map_err(db_err)?;
                (ActionVerb::Updated, Some(s.id.clone()))
            }
            None => {
                let created = crate::db::services::create(&state.db, spec.clone())
                    .await
                    .map_err(db_err)?;
                (ActionVerb::Created, Some(created.id))
            }
        };
        if let Some(id) = changed_id {
            changed.push((id, label.clone()));
        }
        report.actions.push(ResourceAction {
            kind: ResourceActionKind::Service,
            name: label,
            action,
        });
    }

    // 3. Prune: remover serviços do projeto que não constam no manifesto.
    if prune {
        let wanted: std::collections::HashSet<&str> =
            specs.iter().map(|s| s.name.as_str()).collect();
        for s in &existing_services {
            if wanted.contains(s.spec.name.as_str()) {
                continue;
            }
            let label = format!("{name}/{}", s.spec.name);
            // Reusa a mesma semântica de remoção da TUI (remove rota + DB).
            match super::service_delete::handle(state.clone(), s.id.clone()).await {
                RpResponse::Err { code, message } => {
                    warn!(%label, %code, %message, "manifest_apply: prune falhou");
                }
                _ => {
                    info!(%label, "manifest_apply: serviço removido (prune)");
                    report.actions.push(ResourceAction {
                        kind: ResourceActionKind::Service,
                        name: label,
                        action: ActionVerb::Deleted,
                    });
                }
            }
        }
    }

    Ok(())
}

fn db_err(e: anyhow::Error) -> RpResponse {
    tracing::error!(error = %e, "manifest_apply: erro de banco");
    RpResponse::err("DatabaseError", e.to_string())
}
