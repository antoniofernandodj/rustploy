//! Tabela `shared_access`: quais projetos podem alcançar cada servidor de banco
//! compartilhado.

use super::Db;
use anyhow::Result;
use chrono::Utc;

pub async fn grant(db: &Db, server_id: &str, project_id: &str) -> Result<()> {
    sqlx::query(
        "INSERT OR IGNORE INTO shared_access (server_service_id, project_id, created_at)
         VALUES (?, ?, ?)",
    )
    .bind(server_id)
    .bind(project_id)
    .bind(Utc::now())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn revoke(db: &Db, server_id: &str, project_id: &str) -> Result<bool> {
    let n = sqlx::query("DELETE FROM shared_access WHERE server_service_id = ? AND project_id = ?")
        .bind(server_id)
        .bind(project_id)
        .execute(db)
        .await?
        .rows_affected();
    Ok(n > 0)
}

/// Projetos autorizados num servidor, em ordem de concessão.
pub async fn projects_of(db: &Db, server_id: &str) -> Result<Vec<String>> {
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT project_id FROM shared_access WHERE server_service_id = ? ORDER BY created_at",
    )
    .bind(server_id)
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|(p,)| p).collect())
}

pub async fn delete_for_server(db: &Db, server_id: &str) -> Result<()> {
    sqlx::query("DELETE FROM shared_access WHERE server_service_id = ?")
        .bind(server_id)
        .execute(db)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn grant_e_idempotente_e_revoke_remove() {
        let dir = std::env::temp_dir().join(format!("rustploy_test_shared_{}", ulid::Ulid::new()));
        let db = crate::db::connect(&dir).await.unwrap();
        grant(&db, "svc_a", "prj_1").await.unwrap();
        grant(&db, "svc_a", "prj_1").await.unwrap();
        grant(&db, "svc_a", "prj_2").await.unwrap();
        assert_eq!(projects_of(&db, "svc_a").await.unwrap().len(), 2);
        assert!(revoke(&db, "svc_a", "prj_1").await.unwrap());
        assert!(!revoke(&db, "svc_a", "prj_1").await.unwrap());
        assert_eq!(projects_of(&db, "svc_a").await.unwrap(), vec!["prj_2"]);
        delete_for_server(&db, "svc_a").await.unwrap();
        assert!(projects_of(&db, "svc_a").await.unwrap().is_empty());
    }
}
