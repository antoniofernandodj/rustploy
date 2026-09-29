//! Tabela `project`: CRUD de projetos e suas env vars de nível de projeto.

use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use shared::{EnvComment, EnvVar, Project};
use ulid::Ulid;

struct ProjectRow {
    id: String,
    name: String,
    description: Option<String>,
    env_vars: String,
    env_comments: String,
    created_at: DateTime<Utc>,
}

fn row_to_project(row: ProjectRow) -> Result<Project> {
    let env_vars: Vec<EnvVar> = serde_json::from_str(&row.env_vars)?;
    let env_comments: Vec<EnvComment> = serde_json::from_str(&row.env_comments)?;
    Ok(Project {
        id: row.id,
        name: row.name,
        description: row.description,
        env_vars,
        env_comments,
        created_at: row.created_at,
    })
}

pub async fn create(db: &Db, name: String, description: Option<String>) -> Result<Project> {
    let id = format!("prj_{}", Ulid::new());
    let now = Utc::now();
    // Rede gravada na criação (ID inteiro: não colide) e lida daí em diante.
    let network_name = shared::new_project_network_name(&id);
    sqlx::query(
        "INSERT INTO project (id, name, description, env_vars, env_comments, created_at, network_name)
         VALUES (?, ?, ?, '[]', '[]', ?, ?)",
    )
    .bind(&id)
    .bind(&name)
    .bind(&description)
    .bind(now)
    .bind(&network_name)
    .execute(db)
    .await?;
    Ok(Project {
        id,
        name,
        description,
        env_vars: vec![],
        env_comments: vec![],
        created_at: now,
    })
}

pub async fn update_env_vars(
    db: &Db,
    id: &str,
    env_vars: Vec<EnvVar>,
    env_comments: Vec<EnvComment>,
) -> Result<Option<Project>> {
    let env_vars_json = serde_json::to_string(&env_vars)?;
    let env_comments_json = serde_json::to_string(&env_comments)?;
    let rows_affected =
        sqlx::query("UPDATE project SET env_vars = ?, env_comments = ? WHERE id = ?")
            .bind(&env_vars_json)
            .bind(&env_comments_json)
            .bind(id)
            .execute(db)
            .await?
            .rows_affected();
    if rows_affected == 0 {
        return Ok(None);
    }
    get(db, id).await
}

/// Nome da rede Docker do projeto — o gravado. Sem gravado (banco de antes da
/// migração, ou linha inserida por fora, como faz o importer), a fórmula
/// legada, que é o nome que a rede já tem.
pub async fn network_name(db: &Db, project_id: &str) -> Result<String> {
    let stored: Option<Option<String>> =
        sqlx::query_scalar("SELECT network_name FROM project WHERE id = ?")
            .bind(project_id)
            .fetch_optional(db)
            .await?;
    Ok(stored
        .flatten()
        .unwrap_or_else(|| crate::docker::networks::legacy_project_net_for(project_id)))
}

/// `id do projeto → nome da rede` de todos os projetos (o inventário Docker
/// descobre por aqui de quem é cada rede `rp_net_*`).
pub async fn network_names(db: &Db) -> Result<std::collections::HashMap<String, String>> {
    let rows: Vec<(String, Option<String>)> =
        sqlx::query_as("SELECT id, network_name FROM project")
            .fetch_all(db)
            .await?;
    Ok(rows
        .into_iter()
        .map(|(id, net)| {
            let net = net.unwrap_or_else(|| crate::docker::networks::legacy_project_net_for(&id));
            (id, net)
        })
        .collect())
}

pub async fn list(db: &Db) -> Result<Vec<Project>> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            String,
            String,
            DateTime<Utc>,
        ),
    >(
        "SELECT id, name, description, env_vars, env_comments, created_at
             FROM project ORDER BY created_at ASC",
    )
    .fetch_all(db)
    .await?;
    rows.into_iter()
        .map(
            |(id, name, description, env_vars, env_comments, created_at)| {
                row_to_project(ProjectRow {
                    id,
                    name,
                    description,
                    env_vars,
                    env_comments,
                    created_at,
                })
            },
        )
        .collect()
}

pub async fn get(db: &Db, id: &str) -> Result<Option<Project>> {
    let row = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            String,
            String,
            DateTime<Utc>,
        ),
    >(
        "SELECT id, name, description, env_vars, env_comments, created_at
             FROM project WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(db)
    .await?;
    row.map(
        |(id, name, description, env_vars, env_comments, created_at)| {
            row_to_project(ProjectRow {
                id,
                name,
                description,
                env_vars,
                env_comments,
                created_at,
            })
        },
    )
    .transpose()
}

pub async fn update(
    db: &Db,
    id: &str,
    name: String,
    description: Option<String>,
) -> Result<Option<Project>> {
    let rows_affected = sqlx::query("UPDATE project SET name = ?, description = ? WHERE id = ?")
        .bind(&name)
        .bind(&description)
        .bind(id)
        .execute(db)
        .await?
        .rows_affected();
    if rows_affected == 0 {
        return Ok(None);
    }
    get(db, id).await
}

pub async fn delete(db: &Db, id: &str) -> Result<bool> {
    let rows_affected = sqlx::query("DELETE FROM project WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?
        .rows_affected();
    Ok(rows_affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::EnvVarValue;

    async fn mem_db() -> Db {
        let dir = std::env::temp_dir().join(format!("rustploy_test_{}", Ulid::new()));
        super::super::connect(&dir).await.unwrap()
    }

    #[tokio::test]
    async fn env_vars_e_comentarios_persistem() {
        let db = mem_db().await;
        let proj = create(&db, "domain".into(), None).await.unwrap();

        let env_vars = vec![EnvVar {
            key: "JWT_SECRET".into(),
            value: EnvVarValue::Plain("abc123".into()),
        }];
        let env_comments = vec![EnvComment {
            text: "# segredo do app".into(),
            before_key: Some("JWT_SECRET".into()),
        }];

        update_env_vars(&db, &proj.id, env_vars.clone(), env_comments.clone())
            .await
            .unwrap();

        let got = get(&db, &proj.id).await.unwrap().unwrap();
        assert_eq!(got.env_vars, env_vars);
        assert_eq!(
            got.env_comments, env_comments,
            "comentário do .env não persistiu"
        );
    }
}
