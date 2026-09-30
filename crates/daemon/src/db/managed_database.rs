//! Tabela `managed_database`: databases criados em servidores compartilhados.
//! A senha fica aqui (é a mesma que o projeto consumidor guarda na env var
//! `Plain`) para a migração poder usá-la como destino.

use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use ulid::Ulid;

#[derive(Debug, Clone)]
pub struct Row {
    pub id: String,
    pub server_service_id: String,
    pub project_id: String,
    pub name: String,
    pub password: String,
    pub env_var: String,
    pub connection_limit: Option<u32>,
    pub statement_timeout_ms: Option<u32>,
    pub created_at: DateTime<Utc>,
}

type Tuple = (
    String,
    String,
    String,
    String,
    String,
    String,
    Option<i64>,
    Option<i64>,
    DateTime<Utc>,
);

const COLS: &str = "id, server_service_id, project_id, name, password, env_var, connection_limit, statement_timeout_ms, created_at";

fn row(t: Tuple) -> Row {
    Row {
        id: t.0,
        server_service_id: t.1,
        project_id: t.2,
        name: t.3,
        password: t.4,
        env_var: t.5,
        connection_limit: t.6.map(|v| v as u32),
        statement_timeout_ms: t.7.map(|v| v as u32),
        created_at: t.8,
    }
}

pub async fn insert(db: &Db, r: &Row) -> Result<Row> {
    let id = format!("mdb_{}", Ulid::new());
    sqlx::query(&format!(
        "INSERT INTO managed_database ({COLS}) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    ))
    .bind(&id)
    .bind(&r.server_service_id)
    .bind(&r.project_id)
    .bind(&r.name)
    .bind(&r.password)
    .bind(&r.env_var)
    .bind(r.connection_limit.map(i64::from))
    .bind(r.statement_timeout_ms.map(i64::from))
    .bind(r.created_at)
    .execute(db)
    .await?;
    Ok(Row { id, ..r.clone() })
}

pub async fn list(db: &Db, server_id: &str) -> Result<Vec<Row>> {
    let rows: Vec<Tuple> = sqlx::query_as(&format!(
        "SELECT {COLS} FROM managed_database WHERE server_service_id = ? ORDER BY created_at"
    ))
    .bind(server_id)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(row).collect())
}

pub async fn get(db: &Db, id: &str) -> Result<Option<Row>> {
    let r: Option<Tuple> =
        sqlx::query_as(&format!("SELECT {COLS} FROM managed_database WHERE id = ?"))
            .bind(id)
            .fetch_optional(db)
            .await?;
    Ok(r.map(row))
}

pub async fn delete(db: &Db, id: &str) -> Result<()> {
    sqlx::query("DELETE FROM managed_database WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete_for_server(db: &Db, server_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM managed_database WHERE server_service_id = ?")
        .bind(server_id)
        .execute(db)
        .await?;
    Ok(())
}
