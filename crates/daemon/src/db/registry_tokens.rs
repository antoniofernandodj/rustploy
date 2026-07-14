//! Tokens de acesso do registry OCI embutido (Basic auth — ver
//! `crate::registry::auth`). Só o SHA-256 do segredo é persistido; o valor em
//! texto plano só existe na resposta de criação (`Response::RegistryTokenCreated`),
//! uma única vez. Convenção espelha `db/webhook_tokens.rs`.

use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use ulid::Ulid;

/// Nome reservado do token interno usado pelo próprio deploy executor pra
/// puxar imagens do registry embutido (ver `crate::registry::internal_token`).
/// Nunca visto por humano, não aparece em `list()`, não pode ser criado via
/// `RegistryTokenCreate`.
pub const RP_INTERNAL: &str = "rp-internal";

pub struct TokenInfo {
    pub name: String,
    pub scope: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

pub async fn create(db: &Db, name: &str, token_sha256: &str, scope: &str) -> Result<()> {
    let id = format!("rtok_{}", Ulid::new());
    sqlx::query(
        "INSERT INTO registry_tokens (id, name, token_sha256, scope, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(name)
    .bind(token_sha256)
    .bind(scope)
    .bind(Utc::now())
    .execute(db)
    .await?;
    Ok(())
}

/// Cria ou atualiza o token interno `rp-internal`, regenerado a cada boot do
/// daemon (ver `crate::registry::internal_token::ensure`). Idempotente: o
/// hash mais recente sempre vence.
pub async fn upsert_internal(db: &Db, token_sha256: &str) -> Result<()> {
    let id = format!("rtok_{}", Ulid::new());
    sqlx::query(
        "INSERT INTO registry_tokens (id, name, token_sha256, scope, created_at)
         VALUES (?, ?, ?, 'pull', ?)
         ON CONFLICT(name) DO UPDATE SET
            token_sha256 = excluded.token_sha256,
            created_at   = excluded.created_at",
    )
    .bind(id)
    .bind(RP_INTERNAL)
    .bind(token_sha256)
    .bind(Utc::now())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn list(db: &Db) -> Result<Vec<TokenInfo>> {
    let rows: Vec<(String, String, DateTime<Utc>, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT name, scope, created_at, last_used_at FROM registry_tokens WHERE name != 'rp-internal' ORDER BY name ASC",
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(name, scope, created_at, last_used_at)| TokenInfo {
            name,
            scope,
            created_at,
            last_used_at,
        })
        .collect())
}

pub async fn revoke(db: &Db, name: &str) -> Result<bool> {
    let rows_affected = sqlx::query("DELETE FROM registry_tokens WHERE name = ?")
        .bind(name)
        .execute(db)
        .await?
        .rows_affected();
    Ok(rows_affected > 0)
}

/// Retorna o escopo do token cujo hash bate, se existir. Usado no caminho
/// quente de auth (toda requisição do registry) — não precisa ser
/// constant-time: é um lookup de igualdade por hash de segredo de alta
/// entropia, não uma comparação de string curta em memória.
pub async fn verify_scope(db: &Db, token_sha256: &str) -> Result<Option<String>> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT scope FROM registry_tokens WHERE token_sha256 = ?")
            .bind(token_sha256)
            .fetch_optional(db)
            .await?;
    Ok(row.map(|(s,)| s))
}

/// Best-effort, chamado em background (`tokio::spawn`) pelo caminho de auth —
/// não deve atrasar a resposta da requisição autenticada.
pub async fn touch_last_used(db: &Db, token_sha256: &str) -> Result<()> {
    sqlx::query("UPDATE registry_tokens SET last_used_at = ? WHERE token_sha256 = ?")
        .bind(Utc::now())
        .bind(token_sha256)
        .execute(db)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem_db() -> Db {
        let dir = std::env::temp_dir().join(format!("rustploy_test_registry_tokens_{}", Ulid::new()));
        super::super::connect(&dir).await.unwrap()
    }

    #[tokio::test]
    async fn create_e_list() {
        let db = mem_db().await;
        create(&db, "ci", "hash1", "push").await.unwrap();
        create(&db, "readonly", "hash2", "pull").await.unwrap();

        let tokens = list(&db).await.unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].name, "ci");
        assert_eq!(tokens[0].scope, "push");
        assert!(tokens[0].last_used_at.is_none());
        assert_eq!(tokens[1].name, "readonly");
    }

    #[tokio::test]
    async fn revoke_e_idempotente() {
        let db = mem_db().await;
        create(&db, "ci", "hash1", "push").await.unwrap();
        assert!(revoke(&db, "ci").await.unwrap());
        assert!(list(&db).await.unwrap().is_empty());
        // Segunda vez: nada pra apagar.
        assert!(!revoke(&db, "ci").await.unwrap());
    }

    #[tokio::test]
    async fn verify_scope_hash_desconhecido_e_conhecido() {
        let db = mem_db().await;
        create(&db, "ci", "hash1", "push").await.unwrap();
        assert_eq!(verify_scope(&db, "hash1").await.unwrap(), Some("push".to_string()));
        assert_eq!(verify_scope(&db, "nope").await.unwrap(), None);
    }

    #[tokio::test]
    async fn touch_last_used_atualiza_o_campo() {
        let db = mem_db().await;
        create(&db, "ci", "hash1", "push").await.unwrap();
        touch_last_used(&db, "hash1").await.unwrap();
        let tokens = list(&db).await.unwrap();
        assert!(tokens[0].last_used_at.is_some());
    }

    #[tokio::test]
    async fn nome_duplicado_da_erro() {
        let db = mem_db().await;
        create(&db, "ci", "hash1", "push").await.unwrap();
        let err = create(&db, "ci", "hash2", "pull").await.unwrap_err();
        assert!(err.to_string().contains("UNIQUE constraint failed"));
    }

    #[tokio::test]
    async fn upsert_internal_e_idempotente() {
        let db = mem_db().await;
        upsert_internal(&db, "hash1").await.unwrap();
        upsert_internal(&db, "hash2").await.unwrap();
        assert_eq!(
            verify_scope(&db, "hash2").await.unwrap(),
            Some("pull".to_string())
        );
        assert_eq!(verify_scope(&db, "hash1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn list_nao_retorna_rp_internal() {
        let db = mem_db().await;
        upsert_internal(&db, "hash1").await.unwrap();
        create(&db, "ci", "hash2", "push").await.unwrap();

        let tokens = list(&db).await.unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].name, "ci");
    }
}
