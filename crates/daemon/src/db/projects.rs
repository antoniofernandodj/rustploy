use super::{extract_id, Db};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use shared::{EnvVar, Project};
use surrealdb::sql::Datetime as SdbDatetime;
use ulid::Ulid;

#[derive(Debug, Serialize, Deserialize)]
struct ProjectRecord {
    id: Option<surrealdb::sql::Thing>,
    name: String,
    description: Option<String>,
    #[serde(default)]
    env_vars: Vec<EnvVar>,
    created_at: SdbDatetime,
}

impl ProjectRecord {
    fn into_project(self) -> Project {
        Project {
            id: self.id.as_ref().map(extract_id).unwrap_or_default(),
            name: self.name,
            description: self.description,
            env_vars: self.env_vars,
            created_at: self.created_at.0,
        }
    }
}

#[derive(Serialize)]
struct EnvVarsPatch {
    env_vars: Vec<EnvVar>,
}

pub async fn create(db: &Db, name: String, description: Option<String>) -> Result<Project> {
    let id = Ulid::new().to_string();
    let record = ProjectRecord {
        id: None,
        name,
        description,
        env_vars: vec![],
        created_at: SdbDatetime::from(Utc::now()),
    };
    let created: Option<ProjectRecord> = db.create(("project", &id)).content(record).await?;
    Ok(created.unwrap().into_project())
}

pub async fn update_env_vars(db: &Db, id: &str, env_vars: Vec<EnvVar>) -> Result<Option<Project>> {
    let patch = EnvVarsPatch { env_vars };
    let updated: Option<ProjectRecord> = db.update(("project", id)).merge(patch).await?;
    Ok(updated.map(|r| r.into_project()))
}

pub async fn list(db: &Db) -> Result<Vec<Project>> {
    let records: Vec<ProjectRecord> = db.select("project").await?;
    Ok(records.into_iter().map(|r| r.into_project()).collect())
}

pub async fn get(db: &Db, id: &str) -> Result<Option<Project>> {
    let record: Option<ProjectRecord> = db.select(("project", id)).await?;
    Ok(record.map(|r| r.into_project()))
}

pub async fn delete(db: &Db, id: &str) -> Result<bool> {
    let deleted: Option<ProjectRecord> = db.delete(("project", id)).await?;
    Ok(deleted.is_some())
}
