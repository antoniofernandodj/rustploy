use crate::db::Db;
use anyhow::Result;
use shared::Event;
use tracing::warn;

/// Persiste um evento no log. Só grava tipos relevantes para replay (estado
/// de deploy e status de serviço); logs de build, métricas e linhas de
/// container já têm tabelas próprias ou são efémeros.
pub async fn append(db: &Db, event: &Event) -> Result<()> {
    let (kind, service_id) = match event {
        Event::DeployStateChanged { service_id, .. } => ("DeployStateChanged", Some(service_id.as_str())),
        Event::DeployProgress { service_id, .. } => ("DeployProgress", Some(service_id.as_str())),
        Event::ServiceStatusChanged { service_id, .. } => ("ServiceStatusChanged", Some(service_id.as_str())),
        _ => return Ok(()),
    };

    let payload = postcard::to_allocvec(event)?;
    sqlx::query(
        "INSERT INTO event_log (kind, service_id, payload, created_at) VALUES (?, ?, ?, datetime('now'))",
    )
    .bind(kind)
    .bind(service_id)
    .bind(&payload)
    .execute(db)
    .await?;
    Ok(())
}

/// Devolve os `limit` eventos mais recentes, filtrados por serviço se fornecido,
/// em ordem cronológica (mais antigo primeiro) para replay correcto.
pub async fn recent(db: &Db, service_id: Option<&str>, limit: i64) -> Result<Vec<Event>> {
    let rows: Vec<(Vec<u8>,)> = match service_id {
        Some(sid) => sqlx::query_as(
            "SELECT payload FROM event_log WHERE service_id = ?
             ORDER BY id DESC LIMIT ?",
        )
        .bind(sid)
        .bind(limit)
        .fetch_all(db)
        .await?,
        None => sqlx::query_as(
            "SELECT payload FROM event_log ORDER BY id DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(db)
        .await?,
    };

    let mut events: Vec<Event> = rows
        .into_iter()
        .filter_map(|(bytes,)| match postcard::from_bytes::<Event>(&bytes) {
            Ok(ev) => Some(ev),
            Err(e) => {
                warn!(error = %e, "event_log: falha ao desserializar evento");
                None
            }
        })
        .collect();

    events.reverse(); // cronológico (mais antigo primeiro)
    Ok(events)
}

/// Remove eventos com mais de `keep_days` dias. Chamado periodicamente para
/// evitar crescimento ilimitado da tabela.
pub async fn trim(db: &Db, keep_days: i64) -> Result<u64> {
    let result = sqlx::query(
        "DELETE FROM event_log WHERE created_at < datetime('now', ? || ' days')",
    )
    .bind(format!("-{keep_days}"))
    .execute(db)
    .await?;
    Ok(result.rows_affected())
}
