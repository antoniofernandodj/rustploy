use super::{extract_id, Db};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use shared::Project;
use ulid::Ulid;

#[derive(Debug, Serialize, Deserialize)]
struct ProjectRecord {
    id: Option<surrealdb::sql::Thing>,
    name: String,
    description: Option<String>,
    created_at: chrono::DateTime<Utc>,
}

impl ProjectRecord {
    fn into_project(self) -> Project {
        Project {
            id: self.id.as_ref().map(extract_id).unwrap_or_default(),
            name: self.name,
            description: self.description,
            created_at: self.created_at,
        }
    }
}

pub async fn create(db: &Db, name: String, description: Option<String>) -> Result<Project> {
    let id = Ulid::new().to_string();
    let record = ProjectRecord {
        id: None,
        name,
        description,
        created_at: Utc::now(),
    };
    let created: Option<ProjectRecord> = db.create(("project", &id)).content(record).await?;
    Ok(created.unwrap().into_project())
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
