pub mod deployments;
pub mod projects;
pub mod services;

use anyhow::Result;
use surrealdb::{engine::local::RocksDb, Surreal};

pub type Db = Surreal<surrealdb::engine::local::Db>;

/// Abre (ou cria) o banco RocksDB no diretório `db_path`.
/// Os dados persistem entre reinicializações do daemon.
pub async fn connect(db_path: &std::path::Path) -> Result<Db> {
    // RocksDb espera um diretório; garantimos que ele exista.
    std::fs::create_dir_all(db_path)?;

    let db = Surreal::new::<RocksDb>(db_path).await?;
    db.use_ns("rustploy").use_db("main").await?;
    migrate(&db).await?;
    Ok(db)
}

async fn migrate(db: &Db) -> Result<()> {
    db.query(
        "
        DEFINE TABLE IF NOT EXISTS project SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS name ON project TYPE string;
        DEFINE FIELD IF NOT EXISTS description ON project TYPE option<string>;
        DEFINE FIELD IF NOT EXISTS env_vars ON project FLEXIBLE TYPE array DEFAULT [];
        DEFINE FIELD IF NOT EXISTS created_at ON project TYPE datetime;
        DEFINE INDEX IF NOT EXISTS project_name ON project COLUMNS name UNIQUE;

        DEFINE TABLE IF NOT EXISTS service SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS name ON service TYPE string;
        DEFINE FIELD IF NOT EXISTS project_id ON service TYPE string;
        DEFINE FIELD IF NOT EXISTS spec ON service FLEXIBLE TYPE object;
        DEFINE FIELD IF NOT EXISTS status ON service TYPE string;
        DEFINE FIELD IF NOT EXISTS live_container_id ON service TYPE option<string>;
        DEFINE FIELD IF NOT EXISTS created_at ON service TYPE datetime;
        DEFINE FIELD IF NOT EXISTS updated_at ON service TYPE datetime;
        DEFINE INDEX IF NOT EXISTS service_domain ON service COLUMNS spec.domain UNIQUE;

        DEFINE TABLE IF NOT EXISTS deployment SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS service_id ON deployment TYPE string;
        DEFINE FIELD IF NOT EXISTS image ON deployment TYPE string;
        DEFINE FIELD IF NOT EXISTS state ON deployment TYPE string;
        DEFINE FIELD IF NOT EXISTS states_log ON deployment FLEXIBLE TYPE array;
        DEFINE FIELD IF NOT EXISTS started_at ON deployment TYPE datetime;
        DEFINE FIELD IF NOT EXISTS finished_at ON deployment TYPE option<datetime>;

        DEFINE TABLE IF NOT EXISTS secret SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS project_id ON secret TYPE string;
        DEFINE FIELD IF NOT EXISTS key ON secret TYPE string;
        DEFINE FIELD IF NOT EXISTS value ON secret TYPE string;
        DEFINE INDEX IF NOT EXISTS secret_project_key ON secret COLUMNS project_id, key UNIQUE;

        DEFINE TABLE IF NOT EXISTS tls_cert SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS domain ON tls_cert TYPE string;
        DEFINE FIELD IF NOT EXISTS cert_pem ON tls_cert TYPE string;
        DEFINE FIELD IF NOT EXISTS key_pem ON tls_cert TYPE string;
        DEFINE FIELD IF NOT EXISTS expires_at ON tls_cert TYPE datetime;
        DEFINE INDEX IF NOT EXISTS tls_cert_domain ON tls_cert COLUMNS domain UNIQUE;
        ",
    )
    .await?;
    Ok(())
}

pub fn extract_id(thing: &surrealdb::sql::Thing) -> String {
    match &thing.id {
        surrealdb::sql::Id::String(s) => s.clone(),
        surrealdb::sql::Id::Number(n) => n.to_string(),
        other => {
            let s = other.to_string();
            s.trim_matches(['⟨', '⟩']).to_string()
        }
    }
}
