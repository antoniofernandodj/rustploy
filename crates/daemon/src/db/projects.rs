use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use shared::{EnvVar, Project};
use ulid::Ulid;

struct ProjectRow {
    id: String,
    name: String,
    description: Option<String>,
    env_vars: String,
    created_at: DateTime<Utc>,
}

fn row_to_project(row: ProjectRow) -> Result<Project> {
    let env_vars: Vec<EnvVar> = serde_json::from_str(&row.env_vars)?;
    Ok(Project {
        id: row.id,
        name: row.name,
        description: row.description,
        env_vars,
        created_at: row.created_at,
    })
}

pub async fn create(db: &Db, name: String, description: Option<String>) -> Result<Project> {
    let id = Ulid::new().to_string();
    let now = Utc::now();
    let env_vars_json = "[]";
    sqlx::query(
        "INSERT INTO project (id, name, description, env_vars, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&description)
    .bind(env_vars_json)
    .bind(now)
    .execute(db)
    .await?;
    Ok(Project {
        id,
        name,
        description,
        env_vars: vec![],
        created_at: now,
    })
}

pub async fn update_env_vars(db: &Db, id: &str, env_vars: Vec<EnvVar>) -> Result<Option<Project>> {
    let env_vars_json = serde_json::to_string(&env_vars)?;
    let rows_affected = sqlx::query(
        "UPDATE project SET env_vars = ? WHERE id = ?",
    )
    .bind(&env_vars_json)
    .bind(id)
    .execute(db)
    .await?
    .rows_affected();
    if rows_affected == 0 {
        return Ok(None);
    }
    get(db, id).await
}

pub async fn list(db: &Db) -> Result<Vec<Project>> {
    let rows = sqlx::query_as::<_, (String, String, Option<String>, String, DateTime<Utc>)>(
        "SELECT id, name, description, env_vars, created_at FROM project ORDER BY created_at ASC",
    )
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(|(id, name, description, env_vars, created_at)| {
            row_to_project(ProjectRow { id, name, description, env_vars, created_at })
        })
        .collect()
}

pub async fn get(db: &Db, id: &str) -> Result<Option<Project>> {
    let row = sqlx::query_as::<_, (String, String, Option<String>, String, DateTime<Utc>)>(
        "SELECT id, name, description, env_vars, created_at FROM project WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(|(id, name, description, env_vars, created_at)| {
        row_to_project(ProjectRow { id, name, description, env_vars, created_at })
    })
    .transpose()
}

pub async fn delete(db: &Db, id: &str) -> Result<bool> {
    let rows_affected = sqlx::query("DELETE FROM project WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?
        .rows_affected();
    Ok(rows_affected > 0)
}
