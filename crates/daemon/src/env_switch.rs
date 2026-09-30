//! Troca de uma env var de conexão (`DATABASE_URL`…) num projeto, com o valor
//! antigo guardado para rollback.
//!
//! Uma chave pode estar no **projeto** e/ou em **serviços** (o serviço
//! sobrescreve o projeto no deploy). Trocar só um dos lugares deixaria o outro
//! apontando para o banco antigo — a falha silenciosa que o doc de desenho
//! descreve. Então a troca vai em todo lugar onde a chave já existe; se não
//! existe em lugar nenhum, entra no projeto.

use crate::db::Db;
use anyhow::Result;
use shared::{EnvVar, EnvVarValue};

/// Onde a chave estava e o que valia (`None` = não existia ali).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnvChange {
    /// `None` = env do projeto; `Some(id)` = env desse serviço.
    pub service_id: Option<String>,
    pub key: String,
    pub previous: Option<EnvVarValue>,
}

/// A chave já está definida no projeto ou em algum serviço dele?
pub async fn is_defined(db: &Db, project_id: &str, key: &str) -> Result<bool> {
    if let Some(p) = crate::db::projects::get(db, project_id).await?
        && p.env_vars.iter().any(|e| e.key == key)
    {
        return Ok(true);
    }
    Ok(crate::db::services::list(db, project_id)
        .await?
        .iter()
        .any(|s| s.spec.env_vars.iter().any(|e| e.key == key)))
}

/// Grava `value` (Plain) em todo lugar onde `key` existe; se em nenhum, no projeto.
pub async fn switch(db: &Db, project_id: &str, key: &str, value: &str) -> Result<Vec<EnvChange>> {
    let new = EnvVarValue::Plain(value.to_string());
    let mut changes = vec![];

    for svc in crate::db::services::list(db, project_id).await? {
        let mut spec = svc.spec.clone();
        let Some(e) = spec.env_vars.iter_mut().find(|e| e.key == key) else {
            continue;
        };
        changes.push(EnvChange {
            service_id: Some(svc.id.clone()),
            key: key.to_string(),
            previous: Some(std::mem::replace(&mut e.value, new.clone())),
        });
        crate::db::services::update_spec(db, &svc.id, spec).await?;
    }

    let Some(project) = crate::db::projects::get(db, project_id).await? else {
        anyhow::bail!("projeto não encontrado");
    };
    let mut vars = project.env_vars.clone();
    let project_has = vars.iter().any(|e| e.key == key);
    if project_has || changes.is_empty() {
        let previous = match vars.iter_mut().find(|e| e.key == key) {
            Some(e) => Some(std::mem::replace(&mut e.value, new.clone())),
            None => {
                vars.push(EnvVar {
                    key: key.to_string(),
                    value: new.clone(),
                });
                None
            }
        };
        crate::db::projects::update_env_vars(db, project_id, vars, project.env_comments).await?;
        changes.push(EnvChange {
            service_id: None,
            key: key.to_string(),
            previous,
        });
    }
    Ok(changes)
}

/// Desfaz [`switch`]: devolve o valor antigo (ou remove a chave criada).
pub async fn restore(db: &Db, project_id: &str, changes: &[EnvChange]) -> Result<()> {
    for c in changes {
        match &c.service_id {
            Some(sid) => {
                let Some(svc) = crate::db::services::get(db, sid).await? else {
                    continue;
                };
                let mut spec = svc.spec.clone();
                match &c.previous {
                    Some(v) => {
                        if let Some(e) = spec.env_vars.iter_mut().find(|e| e.key == c.key) {
                            e.value = v.clone();
                        }
                    }
                    None => spec.env_vars.retain(|e| e.key != c.key),
                }
                crate::db::services::update_spec(db, sid, spec).await?;
            }
            None => {
                let Some(project) = crate::db::projects::get(db, project_id).await? else {
                    continue;
                };
                let mut vars = project.env_vars.clone();
                match &c.previous {
                    Some(v) => {
                        if let Some(e) = vars.iter_mut().find(|e| e.key == c.key) {
                            e.value = v.clone();
                        }
                    }
                    None => vars.retain(|e| e.key != c.key),
                }
                crate::db::projects::update_env_vars(db, project_id, vars, project.env_comments)
                    .await?;
            }
        }
    }
    Ok(())
}
