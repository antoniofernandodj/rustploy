use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use shared::protocol::{BuildLogLine, LogStream};

fn stream_to_str(s: &LogStream) -> &'static str {
    match s {
        LogStream::Stdout => "Stdout",
        LogStream::Stderr => "Stderr",
    }
}

fn str_to_stream(s: &str) -> LogStream {
    match s {
        "Stderr" => LogStream::Stderr,
        _ => LogStream::Stdout,
    }
}

pub async fn append(
    db: &Db,
    job_run_id: &str,
    stream: &LogStream,
    line: &str,
    timestamp: DateTime<Utc>,
) -> Result<()> {
    sqlx::query("INSERT INTO job_log (job_run_id, stream, line, ts) VALUES (?, ?, ?, ?)")
        .bind(job_run_id)
        .bind(stream_to_str(stream))
        .bind(line)
        .bind(timestamp)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn get_for_run(db: &Db, job_run_id: &str) -> Result<Vec<BuildLogLine>> {
    let rows = sqlx::query_as::<_, (String, String, DateTime<Utc>)>(
        "SELECT stream, line, ts FROM job_log WHERE job_run_id = ? ORDER BY ts ASC, id ASC",
    )
    .bind(job_run_id)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(stream, line, ts)| BuildLogLine {
            stream: str_to_stream(&stream),
            line,
            timestamp: ts,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ulid::Ulid;

    async fn mem_db() -> Db {
        let dir = std::env::temp_dir().join(format!("rustploy_test_{}", Ulid::new()));
        super::super::connect(&dir).await.unwrap()
    }

    #[tokio::test]
    async fn append_and_read_preserves_order_and_stream() {
        let db = mem_db().await;
        append(&db, "run_1", &LogStream::Stdout, "linha 1", Utc::now())
            .await
            .unwrap();
        append(&db, "run_1", &LogStream::Stderr, "erro 1", Utc::now())
            .await
            .unwrap();

        let lines = get_for_run(&db, "run_1").await.unwrap();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].line, "linha 1");
        assert_eq!(lines[0].stream, LogStream::Stdout);
        assert_eq!(lines[1].line, "erro 1");
        assert_eq!(lines[1].stream, LogStream::Stderr);
    }
}
