use crate::api::AppState;
use shared::{ProjectEntry, ProjectManifest, Response as RpResponse, ServerManifest};
use tracing::info;

/// Importa um manifesto (raiz `projects:` ou de projeto único `project:`)
/// junto com o `.env` texto que resolve os `${VAR}` usados nele. Interpola no
/// daemon; se sobrar alguma `${VAR}` sem valor em qualquer projeto, aborta
/// ANTES de aplicar qualquer mudança (`MissingEnvVars`). Sem faltantes,
/// reconcilia exatamente como `Command::ManifestApply`.
pub async fn handle(
    state: AppState,
    yaml: String,
    dotenv: String,
    prune: bool,
    deploy: bool,
) -> RpResponse {
    let mut projects = match parse_projects(&yaml) {
        Ok(p) => p,
        Err(msg) => return RpResponse::err("InvalidManifest", msg),
    };
    if projects.is_empty() {
        return RpResponse::err("InvalidManifest", "nenhum projeto encontrado no manifesto");
    }

    let env = shared::parse_dotenv(&dotenv);
    let lookup = |k: &str| env.get(k).cloned();

    let mut missing = Vec::new();
    for m in &mut projects {
        for var in m.interpolate(&lookup) {
            if !missing.contains(&var) {
                missing.push(var);
            }
        }
    }
    if !missing.is_empty() {
        info!(
            count = missing.len(),
            "manifest_import: variáveis não resolvidas, abortando sem aplicar"
        );
        return RpResponse::MissingEnvVars(missing);
    }

    // Os manifestos já interpolados voltam a trafegar como YAML (mesmo motivo
    // do `ManifestApply`: postcard não suporta os defaults/skips dos structs
    // do manifesto) e reutilizam a reconciliação existente.
    let manifests = match projects
        .iter()
        .map(serde_yaml::to_string)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(m) => m,
        Err(e) => return RpResponse::err("SerializeError", e.to_string()),
    };

    super::manifest_apply::handle(state, manifests, prune, deploy).await
}

/// Extrai a lista de `ProjectManifest` de um YAML colado (raiz ou projeto
/// único). Sem suporte a `include:` — não faz sentido para um texto sem
/// arquivo de origem (fluxo de textarea da GUI).
fn parse_projects(yaml: &str) -> Result<Vec<ProjectManifest>, String> {
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).map_err(|e| e.to_string())?;

    if value.get("projects").is_some() {
        let server: ServerManifest = serde_yaml::from_value(value).map_err(|e| e.to_string())?;
        server
            .projects
            .into_iter()
            .map(|entry| match entry {
                ProjectEntry::Inline(m) => Ok(m),
                ProjectEntry::Include { include } => {
                    Err(format!("include: não suportado neste fluxo ({include})"))
                }
            })
            .collect()
    } else if value.get("project").is_some() {
        serde_yaml::from_value(value)
            .map(|m| vec![m])
            .map_err(|e| e.to_string())
    } else {
        Err("manifesto inválido: esperado a chave `project:` ou `projects:` no topo".to_string())
    }
}
