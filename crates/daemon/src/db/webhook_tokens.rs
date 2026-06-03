use anyhow::Result;
use chrono::Utc;

use super::Db;

pub async fn get(db: &Db, service_id: &str) -> Result<Option<String>> {
    let row =
        sqlx::query_scalar::<_, String>("SELECT token FROM webhook_token WHERE service_id = ?")
            .bind(service_id)
            .fetch_optional(db)
            .await?;
    Ok(row)
}

pub async fn upsert(db: &Db, service_id: &str, token: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO webhook_token (service_id, token, created_at)
         VALUES (?, ?, ?)
         ON CONFLICT(service_id) DO UPDATE SET token = excluded.token, created_at = excluded.created_at",
    )
    .bind(service_id)
    .bind(token)
    .bind(Utc::now().to_rfc3339())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete(db: &Db, service_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM webhook_token WHERE service_id = ?")
        .bind(service_id)
        .execute(db)
        .await?;
    Ok(())
}

pub fn generate_token() -> String {
    use std::io::Read;
    let mut bytes = [0u8; 24];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes).map(|_| ()))
        .unwrap_or_default();
    hex::encode(bytes)
}
