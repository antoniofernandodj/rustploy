use anyhow::Result;

use super::Db;

pub async fn get(db: &Db, key: &str) -> Result<Option<String>> {
    let row = sqlx::query_scalar::<_, String>(
        "SELECT value FROM daemon_settings WHERE key = ?",
    )
    .bind(key)
    .fetch_optional(db)
    .await?;
    Ok(row)
}

pub async fn set(db: &Db, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO daemon_settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete(db: &Db, key: &str) -> Result<()> {
    sqlx::query("DELETE FROM daemon_settings WHERE key = ?")
        .bind(key)
        .execute(db)
        .await?;
    Ok(())
}

pub const KEY_WEBHOOK_BASE_URL: &str = "webhook_base_url";
