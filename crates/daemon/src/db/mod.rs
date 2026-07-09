pub mod build_logs;
pub mod daemon_settings;
pub mod event_log;
pub mod git_providers;
pub mod deployments;
pub mod projects;
pub mod services;
pub mod webhook_tokens;

use anyhow::Result;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode},
};
use std::str::FromStr;

pub type Db = SqlitePool;

pub async fn connect(db_path: &std::path::Path) -> Result<Db> {
    std::fs::create_dir_all(db_path)?;
    let db_file = db_path.join("rustploy.db");
    let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_file.display()))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .foreign_keys(true);
    let pool = SqlitePool::connect_with(opts).await?;
    migrate(&pool).await?;
    Ok(pool)
}

async fn migrate(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "
        CREATE TABLE IF NOT EXISTS project (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE,
            description TEXT,
            env_vars    TEXT NOT NULL DEFAULT '[]',
            created_at  TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS service (
            id                 TEXT PRIMARY KEY,
            name               TEXT NOT NULL,
            project_id         TEXT NOT NULL,
            spec               TEXT NOT NULL,
            status             TEXT NOT NULL DEFAULT 'Stopped',
            live_container_id  TEXT,
            created_at         TEXT NOT NULL,
            updated_at         TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_service_project ON service(project_id);

        CREATE TABLE IF NOT EXISTS deployment (
            id          TEXT PRIMARY KEY,
            service_id  TEXT NOT NULL,
            image       TEXT NOT NULL,
            state       TEXT NOT NULL DEFAULT 'Pending',
            states_log  TEXT NOT NULL DEFAULT '[]',
            started_at  TEXT NOT NULL,
            finished_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_deployment_service  ON deployment(service_id);
        CREATE INDEX IF NOT EXISTS idx_deployment_started  ON deployment(started_at);

        CREATE TABLE IF NOT EXISTS secret (
            id         TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            key        TEXT NOT NULL,
            value      TEXT NOT NULL,
            UNIQUE(project_id, key)
        );

        CREATE TABLE IF NOT EXISTS build_log (
            id            INTEGER PRIMARY KEY AUTOINCREMENT,
            deployment_id TEXT NOT NULL,
            line          TEXT NOT NULL,
            ts            TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_build_log_dep ON build_log(deployment_id);

        CREATE TABLE IF NOT EXISTS tls_cert (
            id         TEXT PRIMARY KEY,
            domain     TEXT NOT NULL UNIQUE,
            cert_pem   TEXT NOT NULL,
            key_pem    TEXT NOT NULL,
            expires_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS webhook_token (
            service_id TEXT PRIMARY KEY,
            token      TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS daemon_settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS git_provider (
            id                      TEXT PRIMARY KEY,
            kind                    TEXT NOT NULL,
            name                    TEXT NOT NULL,
            base_url                TEXT NOT NULL,
            auth_mode               TEXT NOT NULL,
            oauth_client_id         TEXT,
            oauth_client_secret_enc TEXT,
            access_token_enc        TEXT,
            refresh_token_enc       TEXT,
            account_login           TEXT,
            account_avatar          TEXT,
            created_at              TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS event_log (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            kind       TEXT NOT NULL,
            service_id TEXT,
            payload    BLOB NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS ix_event_log_svc ON event_log(service_id, id DESC);
        CREATE INDEX IF NOT EXISTS ix_event_log_id  ON event_log(id DESC);
        ",
    )
    .execute(pool)
    .await?;

    // Migrações incrementais (SQLite não tem `ADD COLUMN IF NOT EXISTS`): rode
    // o ALTER e ignore o erro "duplicate column name" quando já foi aplicado.
    add_column_if_missing(
        pool,
        "ALTER TABLE project ADD COLUMN env_comments TEXT NOT NULL DEFAULT '[]'",
    )
    .await?;

    Ok(())
}

/// Executa um `ALTER TABLE ... ADD COLUMN`, tratando como no-op se a coluna já
/// existe (SQLite responde "duplicate column name").
async fn add_column_if_missing(pool: &SqlitePool, sql: &str) -> Result<()> {
    match sqlx::query(sql).execute(pool).await {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("duplicate column name") => Ok(()),
        Err(e) => Err(e.into()),
    }
}
