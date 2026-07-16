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
/// no TOML que ainda não existe no destino — ver [`reconcile_git_providers`].
/// Se sobrar alguma referência `source.git.provider` sem provider correspondente
/// no destino (nem no TOML, nem já conectado) — ver [`check_git_provider_refs`]
/// — também aborta ANTES de aplicar, em vez de silenciosamente cair pra "Git"
/// sem provider. Sem pendências, reconcilia exatamente como `Command::ManifestApply`.
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

    if let Err(resp) = reconcile_git_providers(&state.db, &env.git_provider).await {
        return resp;
    }

    if let Err(resp) = check_git_provider_refs(&state.db, &projects).await {
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
    db: &crate::db::Db,
    docs: &BTreeMap<String, GitProviderDoc>,
) -> Result<(), RpResponse> {
    if docs.is_empty() {
        return Ok(());
    }
    let existing = git_providers::list(db)
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
        git_providers::insert(db, &stored)
            .await
            .map_err(|e| RpResponse::err("DatabaseError", e.to_string()))?;
        info!(%name, "manifest_import: git provider pendente criado a partir do TOML (requer reautenticação)");
    }
    Ok(())
}

/// Garante que todo `source.git.provider` (nome) referenciado no YAML resolve
/// a um provider existente no destino, DEPOIS de [`reconcile_git_providers`]
/// já ter criado os que faltavam a partir do TOML. Sem esta checagem, um nome
/// sem entrada correspondente no TOML **e** sem provider já existente com esse
/// nome seria silenciosamente descartado (o serviço cairia pra "Git" puro, sem
/// provider vinculado, sem nenhum aviso) — aqui vira erro explícito, na mesma
/// linha do `MissingEnvVars`: nada é aplicado até o TOML ou o provider de
/// destino serem corrigidos.
async fn check_git_provider_refs(
    db: &crate::db::Db,
    projects: &[ProjectManifest],
) -> Result<(), RpResponse> {
    let mut referenced: Vec<&str> = Vec::new();
    for m in projects {
        for s in &m.services {
            if let Some(name) = s.source.git.as_ref().and_then(|g| g.provider.as_deref()) {
                if !referenced.contains(&name) {
                    referenced.push(name);
                }
            }
        }
    }
    if referenced.is_empty() {
        return Ok(());
    }

    let existing = git_providers::list(db)
        .await
        .map_err(|e| RpResponse::err("DatabaseError", e.to_string()))?;
    let existing_names: std::collections::HashSet<&str> =
        existing.iter().map(|p| p.name.as_str()).collect();

    let unresolved: Vec<String> = referenced
        .into_iter()
        .filter(|name| !existing_names.contains(name))
        .map(str::to_string)
        .collect();
    if !unresolved.is_empty() {
        return Err(RpResponse::err(
            "UnresolvedGitProvider",
            format!(
                "Git provider(s) referenciado(s) no manifesto sem dados no TOML nem provider já \
                 conectado com esse nome no destino: {}",
                unresolved.join(", ")
            ),
        ));
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

#[cfg(test)]
mod git_provider_iac_tests {
    use super::*;
    use shared::{GitSource, ServiceSource, ServiceSpec};

    async fn temp_db() -> crate::db::Db {
        let dir = std::env::temp_dir().join(format!("rustploy-test-{}", Ulid::new()));
        crate::db::connect(&dir).await.unwrap()
    }

    #[tokio::test]
    async fn export_then_reimport_keeps_git_provider_link_same_daemon() {
        let db = temp_db().await;

        let project = crate::db::projects::create(&db, "Chiquitos".into(), None)
            .await
            .unwrap();

        let stored = StoredProvider {
            id: "gp_TEST".into(),
            kind: "gitea".into(),
            name: "Gitea".into(),
            base_url: "https://gitea.chiquitos.tech".into(),
            auth_mode: "pat".into(),
            oauth_client_id: None,
            oauth_client_secret_enc: None,
            access_token_enc: None,
            refresh_token_enc: None,
            account_login: Some("alice".into()),
            account_avatar: None,
            created_at: Utc::now(),
        };
        git_providers::insert(&db, &stored).await.unwrap();

        let spec = ServiceSpec {
            name: "api".into(),
            project_id: project.id.clone(),
            source: ServiceSource::Git(GitSource {
                url: "https://gitea.chiquitos.tech/Chiquitos/chiquitos.git".into(),
                provider_id: Some("gp_TEST".into()),
                ..GitSource::default()
            }),
            port: 3000,
            host_port: None,
            domain: None,
            tls_enabled: false,
            env_vars: vec![],
            env_comments: vec![],
            volumes: vec![],
            healthcheck: Default::default(),
            replicas: 1,
            resources: Default::default(),
            run_command: None,
            run_args: vec![],
            db_kind: None,
            domains: vec![],
        };
        crate::db::services::create(&db, spec).await.unwrap();

        // ── EXPORT (mesma lógica de manifest_export_all::handle) ──
        let providers: BTreeMap<String, shared::GitProvider> = git_providers::list(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|p| (p.id.clone(), p.to_public()))
            .collect();
        let projects = crate::db::projects::list(&db).await.unwrap();
        let mut items = Vec::new();
        for p in projects {
            let services = crate::db::services::list(&db, &p.id).await.unwrap();
            items.push((p, services));
        }
        let (manifest, env_doc) = ServerManifest::from_existing_redacted(&items, &providers);
        let yaml = serde_yaml::to_string(&manifest).unwrap();
        let toml_text = shared::format_env_doc(&env_doc);

        assert!(yaml.contains("provider: Gitea"), "provider ref ausente no YAML");
        assert!(
            env_doc.git_provider.contains_key("Gitea"),
            "provider doc ausente no EnvDoc"
        );

        // ── IMPORT via o handler de verdade (mesmo daemon, provider já existe) ──
        let mut projects = parse_projects(&yaml).unwrap();
        let env = shared::parse_env_doc(&toml_text).unwrap();

        let mut missing = Vec::new();
        for m in &mut projects {
            missing.extend(m.interpolate(&env));
        }
        assert!(missing.is_empty(), "missing: {missing:?}");

        reconcile_git_providers(&db, &env.git_provider).await.unwrap();
        check_git_provider_refs(&db, &projects).await.unwrap();

        let provider_ids: BTreeMap<String, String> = git_providers::list(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|p| (p.name, p.id))
            .collect();

        let specs = projects[0].service_specs(&project.id, &provider_ids);
        let ServiceSource::Git(g) = &specs[0].source else {
            panic!("esperava git")
        };
        assert_eq!(g.provider_id.as_deref(), Some("gp_TEST"));

        // Persiste via o mesmo caminho de `apply_one` (update em serviço já
        // existente) e refaz a leitura, como a GUI faria ao abrir o serviço.
        let existing = crate::db::services::list(&db, &project.id).await.unwrap();
        let existing_svc = existing.iter().find(|s| s.spec.name == "api").unwrap();
        crate::db::services::update_spec(&db, &existing_svc.id, specs[0].clone())
            .await
            .unwrap();
        let refetched = crate::db::services::get(&db, &existing_svc.id)
            .await
            .unwrap()
            .unwrap();
        let ServiceSource::Git(g2) = &refetched.spec.source else {
            panic!("esperava git")
        };
        assert_eq!(g2.provider_id.as_deref(), Some("gp_TEST"));
    }

    /// Mesmo cenário, mas importando num daemon "fresco" (provider ainda não
    /// existe no destino) — exercita reconcile_git_providers de verdade.
    #[tokio::test]
    async fn import_creates_pending_provider_on_fresh_daemon() {
        let src_db = temp_db().await;
        let dst_db = temp_db().await;

        let project = crate::db::projects::create(&src_db, "Chiquitos".into(), None)
            .await
            .unwrap();
        let stored = StoredProvider {
            id: "gp_SRC".into(),
            kind: "gitea".into(),
            name: "Gitea".into(),
            base_url: "https://gitea.chiquitos.tech".into(),
            auth_mode: "pat".into(),
            oauth_client_id: None,
            oauth_client_secret_enc: None,
            access_token_enc: None,
            refresh_token_enc: None,
            account_login: Some("alice".into()),
            account_avatar: None,
            created_at: Utc::now(),
        };
        git_providers::insert(&src_db, &stored).await.unwrap();
        let spec = ServiceSpec {
            name: "api".into(),
            project_id: project.id.clone(),
            source: ServiceSource::Git(GitSource {
                url: "https://gitea.chiquitos.tech/Chiquitos/chiquitos.git".into(),
                provider_id: Some("gp_SRC".into()),
                ..GitSource::default()
            }),
            port: 3000,
            host_port: None,
            domain: None,
            tls_enabled: false,
            env_vars: vec![],
            env_comments: vec![],
            volumes: vec![],
            healthcheck: Default::default(),
            replicas: 1,
            resources: Default::default(),
            run_command: None,
            run_args: vec![],
            db_kind: None,
            domains: vec![],
        };
        crate::db::services::create(&src_db, spec).await.unwrap();

        let providers: BTreeMap<String, shared::GitProvider> = git_providers::list(&src_db)
            .await
            .unwrap()
            .into_iter()
            .map(|p| (p.id.clone(), p.to_public()))
            .collect();
        let projects = crate::db::projects::list(&src_db).await.unwrap();
        let mut items = Vec::new();
        for p in projects {
            let services = crate::db::services::list(&src_db, &p.id).await.unwrap();
            items.push((p, services));
        }
        let (manifest, env_doc) = ServerManifest::from_existing_redacted(&items, &providers);
        let yaml = serde_yaml::to_string(&manifest).unwrap();
        let toml_text = shared::format_env_doc(&env_doc);

        // ── import no dst_db (provider "Gitea" não existe ainda lá) ──
        assert!(git_providers::list(&dst_db).await.unwrap().is_empty());

        let mut projects = parse_projects(&yaml).unwrap();
        let env = shared::parse_env_doc(&toml_text).unwrap();
        let mut missing = Vec::new();
        for m in &mut projects {
            missing.extend(m.interpolate(&env));
        }
        assert!(missing.is_empty(), "missing: {missing:?}");

        reconcile_git_providers(&dst_db, &env.git_provider).await.unwrap();
        check_git_provider_refs(&dst_db, &projects).await.unwrap();

        let provider_ids: BTreeMap<String, String> = git_providers::list(&dst_db)
            .await
            .unwrap()
            .into_iter()
            .map(|p| (p.name, p.id))
            .collect();

        let dst_project = crate::db::projects::create(&dst_db, "Chiquitos".into(), None)
            .await
            .unwrap();
        let specs = projects[0].service_specs(&dst_project.id, &provider_ids);
        let ServiceSource::Git(g) = &specs[0].source else {
            panic!("esperava git")
        };
        assert!(g.provider_id.is_some(), "provider não resolvido no daemon fresco");

        // Cria (caminho `None => create` do apply_one) e refaz a leitura.
        let created = crate::db::services::create(&dst_db, specs[0].clone())
            .await
            .unwrap();
        let refetched = crate::db::services::get(&dst_db, &created.id)
            .await
            .unwrap()
            .unwrap();
        let ServiceSource::Git(g2) = &refetched.spec.source else {
            panic!("esperava git")
        };
        assert!(g2.provider_id.is_some());
    }

    /// `source.git.provider` referenciando um nome sem entrada no TOML e sem
    /// provider já conectado com esse nome: antes cairia silenciosamente pra
    /// "sem provider"; agora `check_git_provider_refs` barra o import.
    #[tokio::test]
    async fn import_fails_loud_when_provider_ref_is_unresolvable() {
        let db = temp_db().await;
        let yaml = r#"
project:
  name: p
services:
  - name: api
    source:
      git:
        url: https://gitea.example.com/acme/api.git
        provider: Nao Existe
    port: 3000
"#;
        let projects = parse_projects(yaml).unwrap();
        let err = check_git_provider_refs(&db, &projects).await.unwrap_err();
        match err {
            RpResponse::Err { code, message } => {
                assert_eq!(code, "UnresolvedGitProvider");
                assert!(message.contains("Nao Existe"), "{message}");
            }
            other => panic!("esperava Response::Err, veio {other:?}"),
        }
    }
}
