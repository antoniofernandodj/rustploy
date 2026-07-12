//! Resolução de env vars com secrets decifradas — extraído de
//! `DeployExecutor::resolve_env` pra ser reaproveitado também pelo `JobRunner`
//! (`crates/daemon/src/jobs/runner.rs`), que precisa das mesmas env vars de
//! base (projeto + serviço gatilho, secrets incluídas) sem instanciar um
//! `DeployExecutor` inteiro.

use crate::db::Db;
use crate::secrets::SecretsManager;
use anyhow::Result;
use shared::{EnvVarValue, Service};
use std::collections::HashMap;
use tracing::{debug, error, warn};

/// Funde env vars do projeto (base) com as do serviço (sobrescreve por
/// chave), decifrando `EnvVarValue::Secret` via `secrets`. Mesma precedência
/// de `shared::resolve_env_vars`, mas com secrets resolvidas em texto puro
/// (`shared::resolve_env_vars` deixa `EnvVarValue` intacto).
pub async fn resolve(db: &Db, secrets: &SecretsManager, svc: &Service) -> Result<Vec<(String, String)>> {
    let project_env = match crate::db::projects::get(db, &svc.spec.project_id).await {
        Ok(Some(project)) => project.env_vars,
        Ok(None) => {
            warn!(
                service_id = %svc.id,
                project_id = %svc.spec.project_id,
                "resolve_env: projeto não encontrado no banco — env vars de projeto não serão injetadas"
            );
            vec![]
        }
        Err(e) => {
            error!(
                service_id = %svc.id,
                project_id = %svc.spec.project_id,
                error = %e,
                "resolve_env: falha ao carregar projeto (possível erro de desserialização do JSON env_vars) — env vars de projeto não serão injetadas"
            );
            vec![]
        }
    };

    let mut env_map: HashMap<String, String> = HashMap::new();

    for ev in &project_env {
        let value = match &ev.value {
            EnvVarValue::Plain(v) => v.clone(),
            EnvVarValue::Secret(name) => {
                debug!(service_id = %svc.id, secret = %name, "resolve_env: desencriptando secret do projeto");
                secrets.get_raw(&svc.spec.project_id, name).await.unwrap_or_default()
            }
        };
        env_map.insert(ev.key.clone(), value);
    }

    for ev in &svc.spec.env_vars {
        let value = match &ev.value {
            EnvVarValue::Plain(v) => v.clone(),
            EnvVarValue::Secret(name) => {
                debug!(service_id = %svc.id, secret = %name, "resolve_env: desencriptando secret do serviço");
                secrets.get_raw(&svc.spec.project_id, name).await.unwrap_or_default()
            }
        };
        env_map.insert(ev.key.clone(), value);
    }

    let keys: Vec<&str> = env_map.keys().map(|k| k.as_str()).collect();
    tracing::info!(
        service_id = %svc.id,
        project_vars = project_env.len(),
        service_vars = svc.spec.env_vars.len(),
        total = env_map.len(),
        keys = ?keys,
        "resolve_env: vars resolvidas (projeto + serviço)"
    );

    Ok(env_map.into_iter().collect())
}
