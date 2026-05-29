use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use shared::protocol::{BuildLogLine, LogStream};

pub async fn append(db: &Db, deployment_id: &str, line: &str, timestamp: DateTime<Utc>) -> Result<()> {
    sqlx::query(
        "INSERT INTO build_log (deployment_id, line, ts) VALUES (?, ?, ?)",
    )
    .bind(deployment_id)
    .bind(line)
    .bind(timestamp)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_for_deployment(db: &Db, deployment_id: &str) -> Result<Vec<BuildLogLine>> {
    let rows = sqlx::query_as::<_, (String, DateTime<Utc>)>(
        "SELECT line, ts FROM build_log WHERE deployment_id = ? ORDER BY ts ASC",
    )
    .bind(deployment_id)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(line, ts)| BuildLogLine {
            stream: LogStream::Stdout,
            line,
            timestamp: ts,
        })
        .collect())
}
