use crate::api::AppState;
use crate::db::git_providers::{self, StoredProvider};
use chrono::Utc;
use shared::{
    GitAuthMode, GitProviderDoc, GitProviderKind, ProjectEntry, ProjectManifest,
    Response as RpResponse, ServerManifest,
};
use std::collections::BTreeMap;
use tracing::{debug, info, warn};
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
    info!(
        yaml_len = yaml.len(),
        dotenv_len = dotenv.len(),
        prune,
        deploy,
        "manifest_import: recebido"
    );

    let mut projects = match parse_projects(&yaml) {
        Ok(p) => p,
        Err(msg) => {
            warn!(%msg, "manifest_import: YAML inválido");
            return RpResponse::err("InvalidManifest", msg);
        }
    };
    if projects.is_empty() {
        warn!("manifest_import: nenhum projeto encontrado no manifesto");
        return RpResponse::err("InvalidManifest", "nenhum projeto encontrado no manifesto");
    }
    for m in &projects {
        let git_refs: Vec<(String, Option<String>)> = m
            .services
            .iter()
            .filter_map(|s| s.source.git.as_ref().map(|g| (s.name.clone(), g.provider.clone())))
            .collect();
        info!(
            project = %m.project.name,
            service_count = m.services.len(),
            ?git_refs,
            "manifest_import: projeto parseado (referências git ANTES de interpolar)"
        );
    }

    let env = match shared::parse_env_doc(&dotenv) {
        Ok(e) => e,
        Err(msg) => {
            warn!(%msg, "manifest_import: TOML inválido");
            return RpResponse::err("InvalidEnvVars", msg);
        }
    };
    info!(
        project_keys = ?env.project.keys().collect::<Vec<_>>(),
        git_provider_keys = ?env.git_provider.keys().collect::<Vec<_>>(),
        "manifest_import: TOML parseado"
    );

    let mut missing = Vec::new();
    for m in &mut projects {
        for var in m.interpolate(&env) {
            if !missing.contains(&var) {
                missing.push(var);
            }
        }
    }
    if !missing.is_empty() {
        warn!(
            count = missing.len(),
            ?missing,
            "manifest_import: variáveis não resolvidas, abortando sem aplicar"
        );
        return RpResponse::MissingEnvVars(missing);
    }

    if let Err(resp) = reconcile_git_providers(&state.db, &env.git_provider).await {
        warn!(?resp, "manifest_import: reconcile_git_providers falhou, abortando sem aplicar");
        return resp;
    }

    if let Err(resp) = check_git_provider_refs(&state.db, &projects).await {
        warn!(?resp, "manifest_import: check_git_provider_refs falhou, abortando sem aplicar");
        return resp;
    }

    // Os manifestos já interpolados voltam a trafegar como YAML (mesmo motivo
    // do `ManifestApply`) e reutilizam a reconciliação existente.
    let manifests = match projects
        .iter()
        .map(serde_yaml::to_string)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(m) => m,
        Err(e) => return RpResponse::err("SerializeError", e.to_string()),
    };
    for (m, yaml) in projects.iter().zip(manifests.iter()) {
        let git_refs: Vec<(String, Option<String>)> = m
            .services
            .iter()
            .filter_map(|s| s.source.git.as_ref().map(|g| (s.name.clone(), g.provider.clone())))
            .collect();
        info!(
            project = %m.project.name,
            ?git_refs,
            yaml_len = yaml.len(),
            "manifest_import: repassando pra manifest_apply (referências git DEPOIS de interpolar/reserializar)"
        );
    }

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
        debug!("reconcile_git_providers: TOML sem tabela [git_provider], nada a fazer");
        return Ok(());
    }
    let existing = git_providers::list(db)
        .await
        .map_err(|e| RpResponse::err("DatabaseError", e.to_string()))?;
    let existing_names: std::collections::HashSet<&str> =
        existing.iter().map(|p| p.name.as_str()).collect();
    info!(
        toml_names = ?docs.keys().collect::<Vec<_>>(),
        existing_names = ?existing_names,
        "reconcile_git_providers: comparando TOML com providers já existentes no destino"
    );

    for (name, doc) in docs {
        if existing_names.contains(name.as_str()) {
            info!(%name, "reconcile_git_providers: provider já existe no destino com esse nome, mantido intocado");
            continue;
        }
        let Some(kind) = GitProviderKind::from_str(&doc.kind) else {
            warn!(%name, kind = %doc.kind, "reconcile_git_providers: kind desconhecido no TOML, abortando");
            return Err(RpResponse::err(
                "InvalidGitProvider",
                format!("git provider '{name}': kind desconhecido '{}'", doc.kind),
            ));
        };
        let Some(auth_mode) = GitAuthMode::from_str(&doc.auth_mode) else {
            warn!(%name, auth_mode = %doc.auth_mode, "reconcile_git_providers: auth_mode desconhecido no TOML, abortando");
            return Err(RpResponse::err(
                "InvalidGitProvider",
                format!("git provider '{name}': auth_mode desconhecido '{}'", doc.auth_mode),
            ));
        };
        let new_id = format!("gp_{}", Ulid::new());
        let stored = StoredProvider {
            id: new_id.clone(),
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
        info!(
            %name,
            id = %new_id,
            base_url = %doc.base_url,
            auth_mode = %doc.auth_mode,
            "manifest_import: git provider pendente criado a partir do TOML (requer reautenticação)"
        );
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
        debug!("check_git_provider_refs: nenhum serviço git referencia provider por nome");
        return Ok(());
    }

    let existing = git_providers::list(db)
        .await
        .map_err(|e| RpResponse::err("DatabaseError", e.to_string()))?;
    let existing_names: std::collections::HashSet<&str> =
        existing.iter().map(|p| p.name.as_str()).collect();
    info!(
        ?referenced,
        existing_names = ?existing_names,
        "check_git_provider_refs: validando referências do YAML contra providers do destino"
    );

    let unresolved: Vec<String> = referenced
        .into_iter()
        .filter(|name| !existing_names.contains(name))
        .map(str::to_string)
        .collect();
    if !unresolved.is_empty() {
        warn!(
            ?unresolved,
            existing_names = ?existing_names,
            "check_git_provider_refs: referência(s) de provider sem correspondência — abortando import"
        );
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

    /// `AppState` completo (Docker/ingress/TLS/secrets reais, mas sem
    /// depender de nenhum daemon Docker/Let's Encrypt de verdade — nada aqui
    /// chega a fazer uma chamada de rede) para exercitar os handlers reais
    /// (`super::handle`, `manifest_apply::handle`) fim-a-fim, em vez de lógica
    /// replicada manualmente no teste.
    async fn test_state(db: crate::db::Db) -> AppState {
        let tmp = std::env::temp_dir().join(format!("rustploy-test-state-{}", Ulid::new()));
        let config = shared::RustployConfig::default();
        let docker = std::sync::Arc::new(
            crate::docker::DockerClient::connect(&config.docker.socket_path).unwrap(),
        );
        let db = std::sync::Arc::new(db);
        let bus = std::sync::Arc::new(crate::event_bus::EventBus::new());
        let ingress = std::sync::Arc::new(crate::ingress::IngressController::new());
        let secrets = std::sync::Arc::new(
            crate::secrets::SecretsManager::new(&tmp.join("master.key"), db.clone()).unwrap(),
        );
        let tls = std::sync::Arc::new(
            crate::ingress::TlsManager::new(tmp.join("certs"), config.ingress.acme.clone())
                .unwrap(),
        );
        AppState::new(
            db,
            docker,
            ingress,
            bus,
            secrets,
            tls,
            tmp.join("db"),
            tmp.join("backup"),
            30,
            config.api,
            None,
            None,
        )
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

    /// Fim-a-fim de verdade: chama `ManifestExportAll::handle` e depois
    /// `ManifestImport::handle` (este módulo) no MESMO daemon — os dois
    /// handlers reais, não lógica replicada — pra garantir que nada se perde
    /// entre o limite export → texto → import.
    #[tokio::test]
    async fn end_to_end_export_then_import_preserves_provider_link() {
        let db = temp_db().await;

        let project = crate::db::projects::create(&db, "Chiquitos".into(), None)
            .await
            .unwrap();
        let stored = StoredProvider {
            id: "gp_E2E".into(),
            kind: "gitea".into(),
            name: "Gitea".into(),
            base_url: "https://gitea.chiquitos.tech".into(),
            auth_mode: "oauth".into(),
            oauth_client_id: Some("cid".into()),
            oauth_client_secret_enc: None,
            access_token_enc: None,
            refresh_token_enc: None,
            account_login: None,
            account_avatar: None,
            created_at: Utc::now(),
        };
        git_providers::insert(&db, &stored).await.unwrap();

        let spec = ServiceSpec {
            name: "api".into(),
            project_id: project.id.clone(),
            source: ServiceSource::Git(GitSource {
                url: "https://gitea.chiquitos.tech/Chiquitos/chiquitos.git".into(),
                branch: "main-api".into(),
                provider_id: Some("gp_E2E".into()),
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

        let state = test_state(db).await;

        let export = super::super::manifest_export_all::handle(state.clone()).await;
        let RpResponse::ManifestBundle { yaml, dotenv } = export else {
            panic!("esperava ManifestBundle, veio {export:?}");
        };
        assert!(yaml.contains("provider: Gitea"), "YAML sem a referência: {yaml}");
        assert!(dotenv.contains("[git_provider.Gitea]"), "TOML sem a tabela: {dotenv}");

        let import = handle(state.clone(), yaml, dotenv, false, false).await;
        let RpResponse::ManifestReport(report) = import else {
            panic!("esperava ManifestReport, veio {import:?}");
        };
        assert!(
            report.actions.iter().any(|a| a.name.ends_with("/api")),
            "serviço api não apareceu no report: {report:?}"
        );

        let services = crate::db::services::list(&state.db, &project.id).await.unwrap();
        let api = services.iter().find(|s| s.spec.name == "api").unwrap();
        let ServiceSource::Git(g) = &api.spec.source else {
            panic!("esperava git")
        };
        assert_eq!(
            g.provider_id.as_deref(),
            Some("gp_E2E"),
            "provider_id perdido no round-trip real dos handlers"
        );
    }

    /// Mesmo fim-a-fim, mas export de um daemon e import em outro
    /// completamente vazio — o cenário exato dos prints do usuário: o
    /// provider "Gitea" não existe ainda no destino, então precisa nascer
    /// pendente (sem token) a partir do TOML antes do serviço poder ser
    /// vinculado a ele.
    #[tokio::test]
    async fn end_to_end_export_then_import_on_fresh_daemon_links_pending_provider() {
        let src_db = temp_db().await;
        let dst_db = temp_db().await;

        let project = crate::db::projects::create(&src_db, "Chiquitos".into(), None)
            .await
            .unwrap();
        let stored = StoredProvider {
            id: "gp_SRC2".into(),
            kind: "gitea".into(),
            name: "Gitea".into(),
            base_url: "https://gitea.chiquitos.tech".into(),
            auth_mode: "oauth".into(),
            oauth_client_id: Some("cid".into()),
            oauth_client_secret_enc: None,
            access_token_enc: None,
            refresh_token_enc: None,
            account_login: None,
            account_avatar: None,
            created_at: Utc::now(),
        };
        git_providers::insert(&src_db, &stored).await.unwrap();
        let spec = ServiceSpec {
            name: "api".into(),
            project_id: project.id.clone(),
            source: ServiceSource::Git(GitSource {
                url: "https://gitea.chiquitos.tech/Chiquitos/chiquitos.git".into(),
                branch: "main-api".into(),
                provider_id: Some("gp_SRC2".into()),
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

        let src_state = test_state(src_db).await;
        let export = super::super::manifest_export_all::handle(src_state).await;
        let RpResponse::ManifestBundle { yaml, dotenv } = export else {
            panic!("esperava ManifestBundle, veio {export:?}");
        };

        let dst_state = test_state(dst_db).await;
        assert!(git_providers::list(&dst_state.db).await.unwrap().is_empty());

        let import = handle(dst_state.clone(), yaml, dotenv, false, false).await;
        let RpResponse::ManifestReport(report) = import else {
            panic!("esperava ManifestReport, veio {import:?}");
        };
        assert!(report.actions.iter().any(|a| a.name.ends_with("/api")));

        let providers = git_providers::list(&dst_state.db).await.unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].name, "Gitea");
        assert!(
            providers[0].access_token_enc.is_none(),
            "provider pendente não deveria ter token"
        );

        let dst_project = crate::db::projects::list(&dst_state.db)
            .await
            .unwrap()
            .into_iter()
            .find(|p| p.name == "Chiquitos")
            .unwrap();
        let services = crate::db::services::list(&dst_state.db, &dst_project.id)
            .await
            .unwrap();
        let api = services.iter().find(|s| s.spec.name == "api").unwrap();
        let ServiceSource::Git(g) = &api.spec.source else {
            panic!("esperava git")
        };
        assert_eq!(
            g.provider_id.as_deref(),
            Some(providers[0].id.as_str()),
            "serviço importado não ficou vinculado ao provider pendente recém-criado"
        );
    }
}
