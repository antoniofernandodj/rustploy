use anyhow::Result;
use sqlx::SqlitePool;
use std::fs;
use crate::transform::TransformedData;

pub async fn write_sql_file(path: &str, data: &TransformedData) -> Result<()> {
    let mut sql = String::new();
    sql.push_str("-- Migration from Dokploy to Rustploy\n");
    sql.push_str("BEGIN TRANSACTION;\n\n");

    for p in &data.projects {
        let env_vars = serde_json::to_string(&p.env_vars)?;
        sql.push_str(&format!(
            "INSERT INTO project (id, name, description, env_vars, created_at) VALUES ('{}', '{}', {}, '{}', '{}');\n",
            p.id,
            p.name,
            p.description.as_ref().map(|d| format!("'{}'", d)).unwrap_or_else(|| "NULL".to_string()),
            env_vars,
            p.created_at.to_rfc3339()
        ));
    }

    sql.push_str("\n");

    for s in &data.services {
        let spec = serde_json::to_string(&s.spec)?;
        sql.push_str(&format!(
            "INSERT INTO service (id, name, project_id, spec, status, created_at, updated_at) VALUES ('{}', '{}', '{}', '{}', 'Stopped', '{}', '{}');\n",
            s.id,
            s.spec.name,
            s.spec.project_id,
            spec.replace("'", "''"), // Escape single quotes
            s.created_at.to_rfc3339(),
            s.updated_at.to_rfc3339()
        ));
    }

    sql.push_str("\nCOMMIT;\n");
    fs::write(path, sql)?;
    Ok(())
}

pub async fn write_to_db(data: &TransformedData) -> Result<()> {
    // Determine DB path. Default to dev location or system location.
    let db_path = if std::path::Path::new("rustploy.db").exists() {
        "sqlite:rustploy.db"
    } else if std::path::Path::new("/var/lib/rustploy/db/rustploy.db").exists() {
        "sqlite:/var/lib/rustploy/db/rustploy.db"
    } else {
        // Fallback to local one, create it if missing
        "sqlite:rustploy.db"
    };

    let pool = SqlitePool::connect(db_path).await?;

    let mut project_id_map = std::collections::HashMap::new(); // Original ID from data -> ID in DB

    for p in &data.projects {
        let env_vars = serde_json::to_string(&p.env_vars)?;
        
        // Try to find existing project by name
        let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM project WHERE name = ?")
            .bind(&p.name)
            .fetch_optional(&pool)
            .await?;

        let final_id = if let Some((id,)) = existing {
            // Update existing project
            sqlx::query(
                "UPDATE project SET description = ?, env_vars = ? WHERE id = ?"
            )
            .bind(&p.description)
            .bind(&env_vars)
            .bind(&id)
            .execute(&pool)
            .await?;
            id
        } else {
            // Insert new project
            sqlx::query(
                "INSERT INTO project (id, name, description, env_vars, created_at) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(&p.id)
            .bind(&p.name)
            .bind(&p.description)
            .bind(&env_vars)
            .bind(&p.created_at.to_rfc3339())
            .execute(&pool)
            .await?;
            p.id.clone()
        };

        project_id_map.insert(p.id.clone(), final_id);
    }

    for s in &data.services {
        let spec = serde_json::to_string(&s.spec)?;
        
        // Reconcile project_id
        let project_id = project_id_map.get(&s.spec.project_id).unwrap_or(&s.spec.project_id);

        // Delete existing service with same name to avoid duplicates/confusion
        sqlx::query("DELETE FROM service WHERE name = ?")
            .bind(&s.spec.name)
            .execute(&pool)
            .await?;

        sqlx::query(
            "INSERT INTO service (id, name, project_id, spec, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&s.id)
        .bind(&s.spec.name)
        .bind(project_id)
        .bind(&spec)
        .bind("Stopped")
        .bind(&s.created_at.to_rfc3339())
        .bind(&s.updated_at.to_rfc3339())
        .execute(&pool)
        .await?;
    }

    Ok(())
}
