use crate::api::AppState;
use crate::db::git_providers::{self, StoredProvider};
use chrono::Utc;
use shared::{
    GitAuthMode, GitProviderDoc, GitProviderKind, ProjectEntry, ProjectManifest,
    Response as RpResponse, ServerManifest,
};
use std::collections::BTreeMap;
use tracing::info;
use ulid::Ulid;

/// Importa um manifesto (raiz `projects:` ou de projeto único `project:`)
/// junto com o TOML de variáveis (`EnvDoc`, aninhado por projeto → serviço) que
/// resolve os `${VAR}` usados nele. Interpola no daemon **com escopo** (cada
/// serviço olha a sua tabela, com fallback para a do projeto); se sobrar alguma
/// `${VAR}` sem valor em qualquer escopo, aborta ANTES de aplicar qualquer
/// mudança (`MissingEnvVars`, cada faltante rotulada com o escopo). Sem
/// faltantes, cria (pendente, sem credenciais) todo Git provider referenciado
/// no TOML que ainda não existe no destino — ver [`reconcile_git_providers`] —
/// e reconcilia exatamente como `Command::ManifestApply`.
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

    let env = match shared::parse_env_doc(&dotenv) {
        Ok(e) => e,
        Err(msg) => return RpResponse::err("InvalidEnvVars", msg),
    };

    let mut missing = Vec::new();
    for m in &mut projects {
        for var in m.interpolate(&env) {
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

    if let Err(resp) = reconcile_git_providers(&state, &env.git_provider).await {
        return resp;
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

/// Garante que todo Git provider referenciado em `docs` (chave = nome, o mesmo
/// usado em `source.git.provider` no YAML) exista no destino. Um provider já
/// existente com o mesmo nome é deixado intocado (nunca sobrescreve
/// credenciais/autenticação já configuradas); um nome ausente vira uma linha
/// **pendente** — kind/base_url/auth_mode/oauth_client_id do TOML, sem
/// token/secret — que o usuário completa depois pela GUI (OAuth ou colar o
/// PAT). Segredos nunca trafegam pelo manifesto.
async fn reconcile_git_providers(
    state: &AppState,
    docs: &BTreeMap<String, GitProviderDoc>,
) -> Result<(), RpResponse> {
    if docs.is_empty() {
        return Ok(());
    }
    let existing = git_providers::list(&state.db)
        .await
        .map_err(|e| RpResponse::err("DatabaseError", e.to_string()))?;
    let existing_names: std::collections::HashSet<&str> =
        existing.iter().map(|p| p.name.as_str()).collect();

    for (name, doc) in docs {
        if existing_names.contains(name.as_str()) {
            continue;
        }
        let Some(kind) = GitProviderKind::from_str(&doc.kind) else {
            return Err(RpResponse::err(
                "InvalidGitProvider",
                format!("git provider '{name}': kind desconhecido '{}'", doc.kind),
            ));
        };
        let Some(auth_mode) = GitAuthMode::from_str(&doc.auth_mode) else {
            return Err(RpResponse::err(
                "InvalidGitProvider",
                format!("git provider '{name}': auth_mode desconhecido '{}'", doc.auth_mode),
            ));
        };
        let stored = StoredProvider {
            id: format!("gp_{}", Ulid::new()),
            kind: kind.as_str().to_string(),
            name: name.clone(),
            base_url: doc.base_url.clone(),
            auth_mode: auth_mode.as_str().to_string(),
            oauth_client_id: doc.oauth_client_id.clone(),
            oauth_client_secret_enc: None,
            access_token_enc: None,
            refresh_token_enc: None,
            account_login: None,
            account_avatar: None,
            created_at: Utc::now(),
        };
        git_providers::insert(&state.db, &stored)
            .await
            .map_err(|e| RpResponse::err("DatabaseError", e.to_string()))?;
        info!(%name, "manifest_import: git provider pendente criado a partir do TOML (requer reautenticação)");
    }
    Ok(())
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
