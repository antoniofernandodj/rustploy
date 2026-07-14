use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use shared::{Job, Recurrence};
use ulid::Ulid;

type JobRow = (
    String,         // id
    String,         // project_id
    String,         // trigger_service_id
    String,         // name
    String,         // compose
    String,         // main_service
    bool,           // enabled
    Option<String>, // recurrence (JSON)
    Option<DateTime<Utc>>, // last_run_at
    Option<DateTime<Utc>>, // next_run_at
    DateTime<Utc>,  // created_at
);

const SELECT_COLS: &str = "id, project_id, trigger_service_id, name, compose, main_service, \
    enabled, recurrence, last_run_at, next_run_at, created_at";

fn row_to_job(row: JobRow) -> Result<Job> {
    let (
        id,
        project_id,
        trigger_service_id,
        name,
        compose,
        main_service,
        enabled,
        recurrence_json,
        last_run_at,
        next_run_at,
        created_at,
    ) = row;
    let recurrence: Option<Recurrence> = recurrence_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()?;
    // Coluna é NOT NULL (sem migração de schema): string vazia é o sentinel
    // de "sem serviço gatilho" (job autônomo), não NULL.
    let trigger_service_id = if trigger_service_id.is_empty() {
        None
    } else {
        Some(trigger_service_id)
    };
    Ok(Job {
        id,
        project_id,
        trigger_service_id,
        name,
        compose,
        main_service,
        enabled,
        recurrence,
        last_run_at,
        next_run_at,
        created_at,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn create(
    db: &Db,
    project_id: &str,
    trigger_service_id: Option<&str>,
    name: &str,
    compose: &str,
    main_service: &str,
    recurrence: Option<Recurrence>,
) -> Result<Job> {
    let id = format!("job_{}", Ulid::new());
    let now = Utc::now();
    let recurrence_json = recurrence.map(|r| serde_json::to_string(&r)).transpose()?;
    let next_run_at = recurrence.map(|r| r.next_after(now));
    sqlx::query(
        "INSERT INTO job (id, project_id, trigger_service_id, name, compose, main_service,
            enabled, recurrence, last_run_at, next_run_at, created_at)
         VALUES (?, ?, ?, ?, ?, ?, 1, ?, NULL, ?, ?)",
    )
    .bind(&id)
    .bind(project_id)
    .bind(trigger_service_id.unwrap_or(""))
    .bind(name)
    .bind(compose)
    .bind(main_service)
    .bind(&recurrence_json)
    .bind(next_run_at)
    .bind(now)
    .execute(db)
    .await?;
    Ok(Job {
        id,
        project_id: project_id.to_string(),
        trigger_service_id: trigger_service_id.map(String::from),
        name: name.to_string(),
        compose: compose.to_string(),
        main_service: main_service.to_string(),
        enabled: true,
        recurrence,
        last_run_at: None,
        next_run_at,
        created_at: now,
    })
}

pub async fn get(db: &Db, id: &str) -> Result<Option<Job>> {
    let row = sqlx::query_as::<_, JobRow>(&format!("SELECT {SELECT_COLS} FROM job WHERE id = ?"))
        .bind(id)
        .fetch_optional(db)
        .await?;
    row.map(row_to_job).transpose()
}

pub async fn list(db: &Db, project_id: &str) -> Result<Vec<Job>> {
    let rows = sqlx::query_as::<_, JobRow>(&format!(
        "SELECT {SELECT_COLS} FROM job WHERE project_id = ? ORDER BY created_at ASC"
    ))
    .bind(project_id)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(row_to_job).collect()
}

pub async fn list_all(db: &Db) -> Result<Vec<Job>> {
    let rows = sqlx::query_as::<_, JobRow>(&format!(
        "SELECT {SELECT_COLS} FROM job ORDER BY created_at ASC"
    ))
    .fetch_all(db)
    .await?;
    rows.into_iter().map(row_to_job).collect()
}

/// Jobs vencidos: habilitados, com agendamento configurado, e cujo
/// `next_run_at` já passou — usado só pelo `scheduler_loop`.
pub async fn list_due(db: &Db, now: DateTime<Utc>) -> Result<Vec<Job>> {
    let rows = sqlx::query_as::<_, JobRow>(&format!(
        "SELECT {SELECT_COLS} FROM job
         WHERE enabled = 1 AND recurrence IS NOT NULL AND next_run_at <= ?"
    ))
    .bind(now)
    .fetch_all(db)
    .await?;
    rows.into_iter().map(row_to_job).collect()
}

#[allow(clippy::too_many_arguments)]
pub async fn update(
    db: &Db,
    id: &str,
    name: &str,
    compose: &str,
    main_service: &str,
    enabled: bool,
    recurrence: Option<Recurrence>,
) -> Result<Option<Job>> {
    let recurrence_json = recurrence.map(|r| serde_json::to_string(&r)).transpose()?;
    let next_run_at = recurrence.map(|r| r.next_after(Utc::now()));
    let rows_affected = sqlx::query(
        "UPDATE job SET name = ?, compose = ?, main_service = ?, enabled = ?, recurrence = ?,
            next_run_at = ? WHERE id = ?",
    )
    .bind(name)
    .bind(compose)
    .bind(main_service)
    .bind(enabled)
    .bind(&recurrence_json)
    .bind(next_run_at)
    .bind(id)
    .execute(db)
    .await?
    .rows_affected();
    if rows_affected == 0 {
        return Ok(None);
    }
    get(db, id).await
}

/// Registra o disparo de um job: `last_run_at` = agora, `next_run_at` já
/// avançado (`None` se o job não tem agendamento — só `JobRunNow`).
pub async fn mark_fired(
    db: &Db,
    id: &str,
    last_run_at: DateTime<Utc>,
    next_run_at: Option<DateTime<Utc>>,
) -> Result<()> {
    sqlx::query("UPDATE job SET last_run_at = ?, next_run_at = ? WHERE id = ?")
        .bind(last_run_at)
        .bind(next_run_at)
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete(db: &Db, id: &str) -> Result<bool> {
    let rows_affected = sqlx::query("DELETE FROM job WHERE id = ?")
        .bind(id)
        .execute(db)
        .await?
        .rows_affected();
    Ok(rows_affected > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem_db() -> Db {
        let dir = std::env::temp_dir().join(format!("rustploy_test_{}", Ulid::new()));
        super::super::connect(&dir).await.unwrap()
    }

    #[tokio::test]
    async fn create_get_update_delete_round_trip() {
        let db = mem_db().await;
        let job = create(
            &db,
            "prj_1",
            Some("svc_1"),
            "backup",
            "version: '3'\nservices:\n  backup:\n    image: busybox\n",
            "backup",
            Some(Recurrence::IntervalHours(6)),
        )
        .await
        .unwrap();
        assert!(job.next_run_at.is_some());
        assert_eq!(job.trigger_service_id, Some("svc_1".to_string()));

        let got = get(&db, &job.id).await.unwrap().unwrap();
        assert_eq!(got.recurrence, Some(Recurrence::IntervalHours(6)));

        let updated = update(&db, &job.id, "backup2", &job.compose, "backup", false, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.name, "backup2");
        assert!(!updated.enabled);
        assert_eq!(updated.recurrence, None);
        assert_eq!(updated.next_run_at, None);

        assert!(delete(&db, &job.id).await.unwrap());
        assert!(get(&db, &job.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn job_sem_servico_gatilho_ida_e_volta_none() {
        let db = mem_db().await;
        let job = create(&db, "prj_1", None, "autonomo", "c", "m", None)
            .await
            .unwrap();
        assert_eq!(job.trigger_service_id, None);

        let got = get(&db, &job.id).await.unwrap().unwrap();
        assert_eq!(got.trigger_service_id, None);
    }

    #[tokio::test]
    async fn list_due_only_returns_enabled_scheduled_and_overdue() {
        let db = mem_db().await;
        let past = create(&db, "p", Some("s"), "past", "c", "m", Some(Recurrence::IntervalHours(1)))
            .await
            .unwrap();
        // força next_run_at pro passado, direto no banco (create já teria calculado no futuro).
        sqlx::query("UPDATE job SET next_run_at = ? WHERE id = ?")
            .bind(Utc::now() - chrono::Duration::hours(1))
            .bind(&past.id)
            .execute(&db)
            .await
            .unwrap();

        let _future = create(&db, "p", Some("s"), "future", "c", "m", Some(Recurrence::IntervalHours(6)))
            .await
            .unwrap();
        let _manual = create(&db, "p", Some("s"), "manual", "c", "m", None).await.unwrap();

        let due = list_due(&db, Utc::now()).await.unwrap();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].id, past.id);
    }
}
