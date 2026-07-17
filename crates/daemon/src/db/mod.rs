pub mod build_logs;
pub mod daemon_settings;
pub mod git_providers;
pub mod deployments;
pub mod job;
pub mod job_log;
pub mod job_run;
pub mod projects;
pub mod registry;
pub mod registry_tokens;
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

        CREATE TABLE IF NOT EXISTS job (
            id                 TEXT PRIMARY KEY,
            project_id         TEXT NOT NULL,
            trigger_service_id TEXT NOT NULL,
            name               TEXT NOT NULL,
            compose            TEXT NOT NULL,
            main_service       TEXT NOT NULL,
            enabled            INTEGER NOT NULL DEFAULT 1,
            recurrence         TEXT,
            last_run_at        TEXT,
            next_run_at        TEXT,
            created_at         TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_job_project  ON job(project_id);
        CREATE INDEX IF NOT EXISTS idx_job_next_run ON job(next_run_at);

        CREATE TABLE IF NOT EXISTS job_run (
            id          TEXT PRIMARY KEY,
            job_id      TEXT NOT NULL,
            started_at  TEXT NOT NULL,
            finished_at TEXT,
            exit_code   INTEGER,
            success     INTEGER
        );

        CREATE INDEX IF NOT EXISTS idx_job_run_job ON job_run(job_id);

        CREATE TABLE IF NOT EXISTS job_log (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            job_run_id TEXT NOT NULL,
            stream     TEXT NOT NULL DEFAULT 'Stdout',
            line       TEXT NOT NULL,
            ts         TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_job_log_run ON job_log(job_run_id);

        CREATE TABLE IF NOT EXISTS registry_repos (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS registry_blobs (
            digest     TEXT PRIMARY KEY,
            size       INTEGER NOT NULL,
            created_at TEXT NOT NULL
        );

        -- PK composta (repo_id, digest): o MESMO conteúdo (mesmo digest) pode
        -- legitimamente pertencer a repos diferentes (duas tags/repos apontando
        -- pra imagem idêntica produzem manifests byte-a-byte iguais). Com
        -- `digest` como PK sozinho, o segundo repo a fazer push do mesmo
        -- conteúdo 'roubava' a posse do primeiro (ON CONFLICT sobrescrevia
        -- repo_id), quebrando list_repos (tamanho zerado), delete_manifest
        -- (404) e até o `docker pull` real do repo que perdia a posse.
        CREATE TABLE IF NOT EXISTS registry_manifests (
            repo_id    TEXT NOT NULL,
            digest     TEXT NOT NULL,
            media_type TEXT NOT NULL,
            size       INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            PRIMARY KEY (repo_id, digest)
        );

        CREATE INDEX IF NOT EXISTS idx_registry_manifests_digest ON registry_manifests(digest);

        CREATE TABLE IF NOT EXISTS registry_tags (
            repo_id         TEXT NOT NULL,
            tag             TEXT NOT NULL,
            manifest_digest TEXT NOT NULL,
            updated_at      TEXT NOT NULL,
            PRIMARY KEY (repo_id, tag)
        );

        CREATE TABLE IF NOT EXISTS registry_manifest_refs (
            manifest_digest TEXT NOT NULL,
            blob_digest     TEXT NOT NULL,
            PRIMARY KEY (manifest_digest, blob_digest)
        );

        CREATE TABLE IF NOT EXISTS registry_tokens (
            id           TEXT PRIMARY KEY,
            name         TEXT NOT NULL UNIQUE,
            token_sha256 TEXT NOT NULL,
            scope        TEXT NOT NULL,
            created_at   TEXT NOT NULL,
            last_used_at TEXT
        );
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
