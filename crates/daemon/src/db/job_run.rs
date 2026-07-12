use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use shared::JobRun;
use ulid::Ulid;

type JobRunRow = (
    String,                // id
    String,                // job_id
    DateTime<Utc>,         // started_at
    Option<DateTime<Utc>>, // finished_at
    Option<i32>,           // exit_code
    Option<bool>,          // success
);

fn row_to_job_run(row: JobRunRow) -> JobRun {
    let (id, job_id, started_at, finished_at, exit_code, success) = row;
    JobRun {
        id,
        job_id,
        started_at,
        finished_at,
        exit_code,
        success,
    }
}

const SELECT_COLS: &str = "id, job_id, started_at, finished_at, exit_code, success";

pub async fn create(db: &Db, job_id: &str) -> Result<JobRun> {
    let id = format!("jrun_{}", Ulid::new());
    let now = Utc::now();
    sqlx::query(
        "INSERT INTO job_run (id, job_id, started_at, finished_at, exit_code, success)
         VALUES (?, ?, ?, NULL, NULL, NULL)",
    )
    .bind(&id)
    .bind(job_id)
    .bind(now)
    .execute(db)
    .await?;
    Ok(JobRun {
        id,
        job_id: job_id.to_string(),
        started_at: now,
        finished_at: None,
        exit_code: None,
        success: None,
    })
}

/// Fecha uma execução com o exit code do processo `docker compose`.
/// `success = exit_code == 0`.
pub async fn finish(db: &Db, id: &str, exit_code: i32) -> Result<Option<JobRun>> {
    let finished_at = Utc::now();
    let success = exit_code == 0;
    let rows_affected =
        sqlx::query("UPDATE job_run SET finished_at = ?, exit_code = ?, success = ? WHERE id = ?")
            .bind(finished_at)
            .bind(exit_code)
            .bind(success)
            .bind(id)
            .execute(db)
            .await?
            .rows_affected();
    if rows_affected == 0 {
        return Ok(None);
    }
    get(db, id).await
}

pub async fn get(db: &Db, id: &str) -> Result<Option<JobRun>> {
    let row = sqlx::query_as::<_, JobRunRow>(&format!(
        "SELECT {SELECT_COLS} FROM job_run WHERE id = ?"
    ))
    .bind(id)
    .fetch_optional(db)
    .await?;
    Ok(row.map(row_to_job_run))
}

pub async fn list_for_job(db: &Db, job_id: &str, limit: usize) -> Result<Vec<JobRun>> {
    let rows = sqlx::query_as::<_, JobRunRow>(&format!(
        "SELECT {SELECT_COLS} FROM job_run WHERE job_id = ? ORDER BY started_at DESC LIMIT ?"
    ))
    .bind(job_id)
    .bind(limit as i64)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(row_to_job_run).collect())
}

pub async fn latest_for_job(db: &Db, job_id: &str) -> Result<Option<JobRun>> {
    let row = sqlx::query_as::<_, JobRunRow>(&format!(
        "SELECT {SELECT_COLS} FROM job_run WHERE job_id = ? ORDER BY started_at DESC LIMIT 1"
    ))
    .bind(job_id)
    .fetch_optional(db)
    .await?;
    Ok(row.map(row_to_job_run))
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem_db() -> Db {
        let dir = std::env::temp_dir().join(format!("rustploy_test_{}", Ulid::new()));
        super::super::connect(&dir).await.unwrap()
    }

    #[tokio::test]
    async fn create_finish_and_list() {
        let db = mem_db().await;
        let run = create(&db, "job_1").await.unwrap();
        assert_eq!(run.success, None);

        let finished = finish(&db, &run.id, 0).await.unwrap().unwrap();
        assert_eq!(finished.exit_code, Some(0));
        assert_eq!(finished.success, Some(true));
        assert!(finished.finished_at.is_some());

        let latest = latest_for_job(&db, "job_1").await.unwrap().unwrap();
        assert_eq!(latest.id, run.id);

        let all = list_for_job(&db, "job_1", 10).await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn nonzero_exit_code_is_failure() {
        let db = mem_db().await;
        let run = create(&db, "job_1").await.unwrap();
        let finished = finish(&db, &run.id, 1).await.unwrap().unwrap();
        assert_eq!(finished.success, Some(false));
    }
}
