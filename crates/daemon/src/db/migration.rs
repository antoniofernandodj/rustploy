//! Tabela `migration`: estado do assistente de migração (passos, log, o que
//! foi parado e as env vars trocadas — o necessário para o rollback).

use super::Db;
use crate::env_switch::EnvChange;
use anyhow::Result;
use chrono::{DateTime, Utc};
use shared::{Migration, MigrationStep};

/// Linha completa: a `Migration` pública + o que só o daemon precisa.
#[derive(Debug, Clone)]
pub struct Record {
    pub m: Migration,
    pub env_changes: Vec<EnvChange>,
    pub stopped_services: Vec<String>,
}

type Tuple = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    DateTime<Utc>,
);

const COLS: &str = "id, project_id, source_service_id, source_database, dest_database_id, env_var, status, steps, log, env_changes, stopped_services, created_at";

fn record(t: Tuple) -> Result<Record> {
    let steps: Vec<MigrationStep> = serde_json::from_str(&t.7)?;
    Ok(Record {
        m: Migration {
            id: t.0,
            project_id: t.1,
            source_service_id: t.2,
            source_database: t.3,
            dest_database_id: t.4,
            env_var: t.5,
            status: t.6,
            steps,
            log: serde_json::from_str(&t.8).unwrap_or_default(),
            created_at: t.11,
        },
        env_changes: serde_json::from_str(&t.9).unwrap_or_default(),
        stopped_services: serde_json::from_str(&t.10).unwrap_or_default(),
    })
}

pub async fn save(db: &Db, r: &Record) -> Result<()> {
    sqlx::query(&format!(
        "INSERT INTO migration ({COLS}) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET status = excluded.status, steps = excluded.steps,
           log = excluded.log, env_changes = excluded.env_changes,
           stopped_services = excluded.stopped_services"
    ))
    .bind(&r.m.id)
    .bind(&r.m.project_id)
    .bind(&r.m.source_service_id)
    .bind(&r.m.source_database)
    .bind(&r.m.dest_database_id)
    .bind(&r.m.env_var)
    .bind(&r.m.status)
    .bind(serde_json::to_string(&r.m.steps)?)
    .bind(serde_json::to_string(&r.m.log)?)
    .bind(serde_json::to_string(&r.env_changes)?)
    .bind(serde_json::to_string(&r.stopped_services)?)
    .bind(r.m.created_at)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get(db: &Db, id: &str) -> Result<Option<Record>> {
    let t: Option<Tuple> = sqlx::query_as(&format!("SELECT {COLS} FROM migration WHERE id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await?;
    t.map(record).transpose()
}

pub async fn list(db: &Db, project_id: &str) -> Result<Vec<Record>> {
    let rows: Vec<Tuple> = sqlx::query_as(&format!(
        "SELECT {COLS} FROM migration WHERE project_id = ? ORDER BY created_at DESC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(record).collect()
}

/// Há migração `Running`/`Completed` (ainda não revertida/descartada) deste banco?
pub async fn active_for_source(db: &Db, source_service_id: &str) -> Result<bool> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM migration WHERE source_service_id = ? AND status IN ('Running', 'Completed')",
    )
    .bind(source_service_id)
    .fetch_one(db)
    .await?;
    Ok(n > 0)
}

/// Migrações deixadas `Running` por um daemon que caiu no meio: viram `Failed`
/// (o dump não retoma; o usuário reverte e tenta de novo).
pub async fn fail_orphans(db: &Db) -> Result<u64> {
    Ok(sqlx::query("UPDATE migration SET status = 'Failed' WHERE status = 'Running'")
        .execute(db)
        .await?
        .rows_affected())
}
