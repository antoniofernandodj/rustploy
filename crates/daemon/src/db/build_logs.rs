use crate::db::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared::protocol::{BuildLogLine, LogStream};

#[derive(Debug, Serialize, Deserialize)]
struct BuildLogRecord {
    deployment_id: String,
    line: String,
    ts: surrealdb::sql::Datetime,
}

pub async fn append(db: &Db, deployment_id: &str, line: &str, timestamp: DateTime<Utc>) -> Result<()> {
    db.query("CREATE build_log SET deployment_id = $dep, line = $line, ts = $ts")
        .bind(("dep", deployment_id.to_string()))
        .bind(("line", line.to_string()))
        .bind(("ts", surrealdb::sql::Datetime::from(timestamp)))
        .await?;
    Ok(())
}

pub async fn get_for_deployment(db: &Db, deployment_id: &str) -> Result<Vec<BuildLogLine>> {
    let mut result = db
        .query("SELECT * FROM build_log WHERE deployment_id = $dep ORDER BY ts ASC")
        .bind(("dep", deployment_id.to_string()))
        .await?;
    let records: Vec<BuildLogRecord> = result.take(0)?;
    Ok(records
        .into_iter()
        .map(|r| BuildLogLine {
            stream: LogStream::Stdout,
            line: r.line,
            timestamp: r.ts.into(),
        })
        .collect())
}
