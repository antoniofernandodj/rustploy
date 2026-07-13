//! Wrappers SQL do registry OCI embutido (metadados; os bytes de blob/manifest
//! vivem no CAS em disco, ver `crate::registry::storage`). Convenção espelha
//! `db/projects.rs`: funções livres `db: &Db`, `anyhow::Result`, IDs ULID.

use super::Db;
use anyhow::Result;
use chrono::{DateTime, Utc};
use ulid::Ulid;

pub struct Repo {
    pub id: String,
}

pub struct ManifestRow {
    pub media_type: String,
    pub size: i64,
}

/// Linha da lista de repositórios (sub-aba Registry).
pub struct RepoSummary {
    pub name: String,
    pub tag_count: i64,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
}

/// Linha da lista de tags de um repositório (sub-aba Registry, detalhe).
pub struct TagDetail {
    pub tag: String,
    pub digest: String,
    pub media_type: String,
    pub size_bytes: i64,
    pub updated_at: DateTime<Utc>,
}

/// Agregados globais do registry, para o cabeçalho da sub-aba Registry.
pub struct RegistrySummary {
    pub repo_count: i64,
    pub blob_count: i64,
    pub storage_bytes: i64,
}

/// Busca o repo por nome; cria (`rrepo_<ulid>`) se ainda não existir. Usado no
/// início de toda rota que recebe `<name>` e pode criar implicitamente (POST
/// upload, PUT manifest) — a spec permite repositório nascer do primeiro push.
pub async fn get_or_create_repo(db: &Db, name: &str) -> Result<Repo> {
    if let Some(r) = get_repo_by_name(db, name).await? {
        return Ok(r);
    }
    let id = format!("rrepo_{}", Ulid::new());
    sqlx::query("INSERT OR IGNORE INTO registry_repos (id, name, created_at) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(name)
        .bind(Utc::now())
        .execute(db)
        .await?;
    // INSERT OR IGNORE pode ter perdido a corrida para outra requisição
    // concorrente criando o mesmo nome — busca de novo em vez de assumir `id`.
    get_repo_by_name(db, name)
        .await?
        .ok_or_else(|| anyhow::anyhow!("registry repo '{name}' deveria existir após insert"))
}

/// Só leitura — usada nas rotas GET/HEAD que não devem criar repo
/// implicitamente (manifests/tags/blobs de leitura sobre um repo inexistente
/// é 404, não criação).
pub async fn get_repo_by_name(db: &Db, name: &str) -> Result<Option<Repo>> {
    let row = sqlx::query_as::<_, (String,)>("SELECT id FROM registry_repos WHERE name = ?")
        .bind(name)
        .fetch_optional(db)
        .await?;
    Ok(row.map(|(id,)| Repo { id }))
}

/// `GET /v2/_catalog` — nomes ordenados. Sem paginação nesta fase.
pub async fn list_repo_names(db: &Db) -> Result<Vec<String>> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM registry_repos ORDER BY name ASC")
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().map(|(n,)| n).collect())
}

/// Lista de repositórios com contagem de tags + tamanho agregado (soma dos
/// manifests do repo — aproximação, blobs podem ser compartilhados entre
/// repos via dedupe). Para a sub-aba Registry da GUI.
pub async fn list_repos(db: &Db) -> Result<Vec<RepoSummary>> {
    let rows: Vec<(String, DateTime<Utc>, i64, i64)> = sqlx::query_as(
        "SELECT r.name, r.created_at,
                (SELECT COUNT(*) FROM registry_tags t WHERE t.repo_id = r.id) AS tag_count,
                COALESCE((SELECT SUM(m.size) FROM registry_manifests m WHERE m.repo_id = r.id), 0) AS size_bytes
         FROM registry_repos r
         ORDER BY r.name ASC",
    )
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(name, created_at, tag_count, size_bytes)| RepoSummary {
            name,
            created_at,
            tag_count,
            size_bytes,
        })
        .collect())
}

/// Tags de um repositório com o manifest que cada uma aponta (digest,
/// media_type, tamanho). Para o detalhe de um repositório na sub-aba Registry.
pub async fn list_tags_detailed(db: &Db, repo_id: &str) -> Result<Vec<TagDetail>> {
    let rows: Vec<(String, String, String, i64, DateTime<Utc>)> = sqlx::query_as(
        "SELECT t.tag, t.manifest_digest, m.media_type, m.size, t.updated_at
         FROM registry_tags t
         JOIN registry_manifests m ON m.digest = t.manifest_digest AND m.repo_id = t.repo_id
         WHERE t.repo_id = ?
         ORDER BY t.tag ASC",
    )
    .bind(repo_id)
    .fetch_all(db)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(tag, digest, media_type, size_bytes, updated_at)| TagDetail {
            tag,
            digest,
            media_type,
            size_bytes,
            updated_at,
        })
        .collect())
}

/// Agregados globais (repos/blobs/tamanho total) para o cabeçalho da sub-aba
/// Registry.
pub async fn summary(db: &Db) -> Result<RegistrySummary> {
    let (repo_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM registry_repos")
        .fetch_one(db)
        .await?;
    let (blob_count, blob_bytes): (i64, i64) =
        sqlx::query_as("SELECT COUNT(*), COALESCE(SUM(size), 0) FROM registry_blobs")
            .fetch_one(db)
            .await?;
    // Um digest pode ter uma linha por repo agora (PK composta); soma cada
    // digest DISTINTO uma vez só — é o que de fato ocupa espaço no CAS.
    let (manifest_bytes,): (i64,) = sqlx::query_as(
        "SELECT COALESCE(SUM(size), 0) FROM (SELECT DISTINCT digest, size FROM registry_manifests)",
    )
    .fetch_one(db)
    .await?;
    Ok(RegistrySummary {
        repo_count,
        blob_count,
        storage_bytes: blob_bytes + manifest_bytes,
    })
}

/// Remove o repositório inteiro (todos os manifests/tags/refs) — só
/// metadados, não mexe no CAS em disco (GC de blob órfão é fase 4, fora de
/// escopo aqui). Só limpa `registry_manifest_refs` dos digests que não
/// pertencem a NENHUM outro repo (calculado antes de apagar as linhas deste
/// repo — o mesmo digest pode ser compartilhado via PK composta).
pub async fn delete_repo(db: &Db, repo_id: &str) -> Result<bool> {
    let mut tx = db.begin().await?;
    sqlx::query(
        "DELETE FROM registry_manifest_refs WHERE manifest_digest IN
         (SELECT digest FROM registry_manifests WHERE repo_id = ?
          AND digest NOT IN (SELECT digest FROM registry_manifests WHERE repo_id != ?))",
    )
    .bind(repo_id)
    .bind(repo_id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("DELETE FROM registry_tags WHERE repo_id = ?")
        .bind(repo_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM registry_manifests WHERE repo_id = ?")
        .bind(repo_id)
        .execute(&mut *tx)
        .await?;
    let rows_affected = sqlx::query("DELETE FROM registry_repos WHERE id = ?")
        .bind(repo_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    tx.commit().await?;
    Ok(rows_affected > 0)
}

/// Idempotente — uploads concorrentes do mesmo blob finalizando quase ao
/// mesmo tempo não colidem.
pub async fn insert_blob(db: &Db, digest: &str, size: i64) -> Result<()> {
    sqlx::query("INSERT OR IGNORE INTO registry_blobs (digest, size, created_at) VALUES (?, ?, ?)")
        .bind(digest)
        .bind(size)
        .bind(Utc::now())
        .execute(db)
        .await?;
    Ok(())
}

pub async fn blob_exists(db: &Db, digest: &str) -> Result<bool> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM registry_blobs WHERE digest = ?")
        .bind(digest)
        .fetch_optional(db)
        .await?;
    Ok(row.is_some())
}

/// Grava o manifest e substitui suas refs numa transação — idempotente:
/// republicar a mesma tag/digest não duplica `registry_manifest_refs`. A
/// chave é `(repo_id, digest)`: o mesmo digest pode pertencer a vários repos
/// (conteúdo idêntico pushado sob nomes diferentes) sem que um "roube" o
/// outro — ver comentário da tabela em `db/mod.rs`.
pub async fn insert_manifest(
    db: &Db,
    digest: &str,
    repo_id: &str,
    media_type: &str,
    size: i64,
    refs: &[String],
) -> Result<()> {
    let mut tx = db.begin().await?;
    sqlx::query(
        "INSERT INTO registry_manifests (repo_id, digest, media_type, size, created_at)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(repo_id, digest) DO UPDATE SET
            media_type = excluded.media_type,
            size = excluded.size",
    )
    .bind(repo_id)
    .bind(digest)
    .bind(media_type)
    .bind(size)
    .bind(Utc::now())
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM registry_manifest_refs WHERE manifest_digest = ?")
        .bind(digest)
        .execute(&mut *tx)
        .await?;
    for r in refs {
        sqlx::query(
            "INSERT OR IGNORE INTO registry_manifest_refs (manifest_digest, blob_digest) VALUES (?, ?)",
        )
        .bind(digest)
        .bind(r)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Confere que o digest pertence ao repo (multi-tenant seguro: um repo não
/// pode ler manifest de outro só porque adivinhou o digest do CAS global).
pub async fn get_manifest(db: &Db, repo_id: &str, digest: &str) -> Result<Option<ManifestRow>> {
    let row = sqlx::query_as::<_, (String, i64)>(
        "SELECT media_type, size FROM registry_manifests WHERE repo_id = ? AND digest = ?",
    )
    .bind(repo_id)
    .bind(digest)
    .fetch_optional(db)
    .await?;
    Ok(row.map(|(media_type, size)| ManifestRow { media_type, size }))
}

/// Checagem GLOBAL no CAS (não por repo) — decisão deliberada: exigir refs
/// presentes globalmente simplifica e é seguro num registry single-admin
/// (ver `docs/plano-registry-embutido.md`, "Riscos e limitações").
pub async fn ref_blob_or_manifest_exists(db: &Db, digest: &str) -> Result<bool> {
    if blob_exists(db, digest).await? {
        return Ok(true);
    }
    let row: Option<(i64,)> = sqlx::query_as("SELECT 1 FROM registry_manifests WHERE digest = ?")
        .bind(digest)
        .fetch_optional(db)
        .await?;
    Ok(row.is_some())
}

pub async fn upsert_tag(db: &Db, repo_id: &str, tag: &str, manifest_digest: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO registry_tags (repo_id, tag, manifest_digest, updated_at) VALUES (?, ?, ?, ?)
         ON CONFLICT(repo_id, tag) DO UPDATE SET
            manifest_digest = excluded.manifest_digest,
            updated_at = excluded.updated_at",
    )
    .bind(repo_id)
    .bind(tag)
    .bind(manifest_digest)
    .bind(Utc::now())
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_tag_digest(db: &Db, repo_id: &str, tag: &str) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT manifest_digest FROM registry_tags WHERE repo_id = ? AND tag = ?",
    )
    .bind(repo_id)
    .bind(tag)
    .fetch_optional(db)
    .await?;
    Ok(row.map(|(d,)| d))
}

pub async fn list_tags(db: &Db, repo_id: &str) -> Result<Vec<String>> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT tag FROM registry_tags WHERE repo_id = ? ORDER BY tag ASC")
            .bind(repo_id)
            .fetch_all(db)
            .await?;
    Ok(rows.into_iter().map(|(t,)| t).collect())
}

/// Remove o manifest deste repo e as tags dele que apontavam para ele — só
/// metadados (GC de blob órfão é fase 4, fora de escopo aqui).
/// `registry_manifest_refs` só é limpo quando NENHUM repo mais tem uma linha
/// pra esse digest (o mesmo digest pode pertencer a outro repo — PK composta).
pub async fn delete_manifest(db: &Db, repo_id: &str, digest: &str) -> Result<bool> {
    let mut tx = db.begin().await?;
    let rows_affected =
        sqlx::query("DELETE FROM registry_manifests WHERE repo_id = ? AND digest = ?")
            .bind(repo_id)
            .bind(digest)
            .execute(&mut *tx)
            .await?
            .rows_affected();
    if rows_affected == 0 {
        tx.rollback().await?;
        return Ok(false);
    }
    sqlx::query("DELETE FROM registry_tags WHERE repo_id = ? AND manifest_digest = ?")
        .bind(repo_id)
        .bind(digest)
        .execute(&mut *tx)
        .await?;
    let (still_referenced,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM registry_manifests WHERE digest = ?")
            .bind(digest)
            .fetch_one(&mut *tx)
            .await?;
    if still_referenced == 0 {
        sqlx::query("DELETE FROM registry_manifest_refs WHERE manifest_digest = ?")
            .bind(digest)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem_db() -> Db {
        let dir = std::env::temp_dir().join(format!("rustploy_test_registry_{}", Ulid::new()));
        super::super::connect(&dir).await.unwrap()
    }

    #[tokio::test]
    async fn get_or_create_repo_e_idempotente() {
        let db = mem_db().await;
        let r1 = get_or_create_repo(&db, "myorg/app").await.unwrap();
        let r2 = get_or_create_repo(&db, "myorg/app").await.unwrap();
        assert_eq!(r1.id, r2.id);
        assert_eq!(list_repo_names(&db).await.unwrap(), vec!["myorg/app"]);
    }

    #[tokio::test]
    async fn insert_manifest_substitui_refs_ao_republicar_mesma_tag() {
        let db = mem_db().await;
        let repo = get_or_create_repo(&db, "app").await.unwrap();
        insert_blob(&db, "blob1", 10).await.unwrap();
        insert_blob(&db, "blob2", 20).await.unwrap();

        insert_manifest(&db, "manifest1", &repo.id, "application/json", 5, &["blob1".into()])
            .await
            .unwrap();
        upsert_tag(&db, &repo.id, "v1", "manifest1").await.unwrap();
        assert_eq!(
            get_tag_digest(&db, &repo.id, "v1").await.unwrap(),
            Some("manifest1".to_string())
        );

        // Republica a mesma tag apontando pra outro manifest com refs diferentes.
        insert_manifest(&db, "manifest2", &repo.id, "application/json", 6, &["blob2".into()])
            .await
            .unwrap();
        upsert_tag(&db, &repo.id, "v1", "manifest2").await.unwrap();
        assert_eq!(
            get_tag_digest(&db, &repo.id, "v1").await.unwrap(),
            Some("manifest2".to_string())
        );
        assert_eq!(list_tags(&db, &repo.id).await.unwrap(), vec!["v1"]);
    }

    #[tokio::test]
    async fn get_manifest_recusa_digest_de_outro_repo() {
        let db = mem_db().await;
        let repo_a = get_or_create_repo(&db, "a").await.unwrap();
        let repo_b = get_or_create_repo(&db, "b").await.unwrap();
        insert_manifest(&db, "shared_digest", &repo_a.id, "application/json", 1, &[])
            .await
            .unwrap();

        assert!(get_manifest(&db, &repo_a.id, "shared_digest")
            .await
            .unwrap()
            .is_some());
        assert!(get_manifest(&db, &repo_b.id, "shared_digest")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn delete_manifest_remove_tags_associadas() {
        let db = mem_db().await;
        let repo = get_or_create_repo(&db, "app").await.unwrap();
        insert_manifest(&db, "m1", &repo.id, "application/json", 1, &[])
            .await
            .unwrap();
        upsert_tag(&db, &repo.id, "latest", "m1").await.unwrap();

        assert!(delete_manifest(&db, &repo.id, "m1").await.unwrap());
        assert!(get_manifest(&db, &repo.id, "m1").await.unwrap().is_none());
        assert_eq!(get_tag_digest(&db, &repo.id, "latest").await.unwrap(), None);
        // Segunda vez: nada pra apagar.
        assert!(!delete_manifest(&db, &repo.id, "m1").await.unwrap());
    }

    #[tokio::test]
    async fn ref_exists_cobre_blob_e_manifest() {
        let db = mem_db().await;
        let repo = get_or_create_repo(&db, "app").await.unwrap();
        insert_blob(&db, "blobref", 1).await.unwrap();
        insert_manifest(&db, "manifestref", &repo.id, "application/json", 1, &[])
            .await
            .unwrap();

        assert!(ref_blob_or_manifest_exists(&db, "blobref").await.unwrap());
        assert!(ref_blob_or_manifest_exists(&db, "manifestref").await.unwrap());
        assert!(!ref_blob_or_manifest_exists(&db, "nope").await.unwrap());
    }

    #[tokio::test]
    async fn list_repos_agrega_tag_count_e_size() {
        let db = mem_db().await;
        let repo = get_or_create_repo(&db, "app").await.unwrap();
        insert_manifest(&db, "m1", &repo.id, "application/json", 100, &[])
            .await
            .unwrap();
        insert_manifest(&db, "m2", &repo.id, "application/json", 250, &[])
            .await
            .unwrap();
        upsert_tag(&db, &repo.id, "v1", "m1").await.unwrap();
        upsert_tag(&db, &repo.id, "v2", "m2").await.unwrap();
        // Repositório sem tags nem manifests: aparece com zeros, não some.
        get_or_create_repo(&db, "empty").await.unwrap();

        let repos = list_repos(&db).await.unwrap();
        assert_eq!(repos.len(), 2);
        let app = repos.iter().find(|r| r.name == "app").unwrap();
        assert_eq!(app.tag_count, 2);
        assert_eq!(app.size_bytes, 350);
        let empty = repos.iter().find(|r| r.name == "empty").unwrap();
        assert_eq!(empty.tag_count, 0);
        assert_eq!(empty.size_bytes, 0);
    }

    #[tokio::test]
    async fn list_tags_detailed_junta_manifest() {
        let db = mem_db().await;
        let repo = get_or_create_repo(&db, "app").await.unwrap();
        insert_manifest(&db, "digest1", &repo.id, "application/vnd.docker.distribution.manifest.v2+json", 42, &[])
            .await
            .unwrap();
        upsert_tag(&db, &repo.id, "v1", "digest1").await.unwrap();

        let tags = list_tags_detailed(&db, &repo.id).await.unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].tag, "v1");
        assert_eq!(tags[0].digest, "digest1");
        assert_eq!(tags[0].media_type, "application/vnd.docker.distribution.manifest.v2+json");
        assert_eq!(tags[0].size_bytes, 42);
    }

    #[tokio::test]
    async fn summary_soma_blobs_e_manifests() {
        let db = mem_db().await;
        let repo = get_or_create_repo(&db, "app").await.unwrap();
        insert_blob(&db, "b1", 10).await.unwrap();
        insert_blob(&db, "b2", 20).await.unwrap();
        insert_manifest(&db, "m1", &repo.id, "application/json", 5, &[])
            .await
            .unwrap();

        let s = summary(&db).await.unwrap();
        assert_eq!(s.repo_count, 1);
        assert_eq!(s.blob_count, 2);
        assert_eq!(s.storage_bytes, 10 + 20 + 5);
    }

    /// Regressão: dois repos pushando o MESMO conteúdo (mesmo digest) não
    /// podem colidir — antes da PK composta, o segundo `insert_manifest`
    /// "roubava" a posse do primeiro repo (via `ON CONFLICT(digest)`),
    /// zerando o tamanho do primeiro em `list_repos` e quebrando
    /// `delete_manifest`/o pull real. Achado num smoke test manual.
    #[tokio::test]
    async fn manifests_com_mesmo_digest_nao_colidem_entre_repos() {
        let db = mem_db().await;
        let repo_a = get_or_create_repo(&db, "hello").await.unwrap();
        let repo_b = get_or_create_repo(&db, "other-app").await.unwrap();
        let digest = "shared";

        insert_manifest(&db, digest, &repo_a.id, "application/json", 524, &[])
            .await
            .unwrap();
        upsert_tag(&db, &repo_a.id, "v1", digest).await.unwrap();

        // Segundo repo publica o MESMO conteúdo (mesmo digest) — não deve
        // afetar a posse do primeiro.
        insert_manifest(&db, digest, &repo_b.id, "application/json", 524, &[])
            .await
            .unwrap();
        upsert_tag(&db, &repo_b.id, "latest", digest).await.unwrap();

        let repos = list_repos(&db).await.unwrap();
        let a = repos.iter().find(|r| r.name == "hello").unwrap();
        let b = repos.iter().find(|r| r.name == "other-app").unwrap();
        assert_eq!(a.size_bytes, 524, "repo A perdeu o tamanho do manifest compartilhado");
        assert_eq!(b.size_bytes, 524);

        // storage_bytes soma o digest compartilhado UMA vez (é o que ocupa no CAS).
        let s = summary(&db).await.unwrap();
        assert_eq!(s.storage_bytes, 524);

        // delete_manifest do repo A não deve mais dar NotFound, e não deve
        // afetar a tag do repo B (que ainda referencia o mesmo digest).
        assert!(delete_manifest(&db, &repo_a.id, digest).await.unwrap());
        assert!(get_manifest(&db, &repo_a.id, digest).await.unwrap().is_none());
        assert!(get_manifest(&db, &repo_b.id, digest).await.unwrap().is_some());
        assert_eq!(get_tag_digest(&db, &repo_b.id, "latest").await.unwrap(), Some(digest.to_string()));
    }

    #[tokio::test]
    async fn delete_repo_remove_tudo_e_e_idempotente() {
        let db = mem_db().await;
        let repo = get_or_create_repo(&db, "app").await.unwrap();
        insert_manifest(&db, "m1", &repo.id, "application/json", 1, &["blobref".into()])
            .await
            .unwrap();
        upsert_tag(&db, &repo.id, "v1", "m1").await.unwrap();

        assert!(delete_repo(&db, &repo.id).await.unwrap());
        assert!(get_manifest(&db, &repo.id, "m1").await.unwrap().is_none());
        assert_eq!(get_tag_digest(&db, &repo.id, "v1").await.unwrap(), None);
        assert!(list_repo_names(&db).await.unwrap().is_empty());
        // Segunda vez: nada pra apagar, não erro.
        assert!(!delete_repo(&db, &repo.id).await.unwrap());
    }

    #[tokio::test]
    async fn delete_repo_preserva_manifest_compartilhado_por_outro_repo() {
        let db = mem_db().await;
        let repo_a = get_or_create_repo(&db, "hello").await.unwrap();
        let repo_b = get_or_create_repo(&db, "other-app").await.unwrap();
        let digest = "shared";
        insert_blob(&db, "blobref", 1).await.unwrap();

        insert_manifest(&db, digest, &repo_a.id, "application/json", 1, &["blobref".into()])
            .await
            .unwrap();
        insert_manifest(&db, digest, &repo_b.id, "application/json", 1, &["blobref".into()])
            .await
            .unwrap();
        upsert_tag(&db, &repo_b.id, "latest", digest).await.unwrap();

        assert!(delete_repo(&db, &repo_a.id).await.unwrap());
        // repo_b ainda tem o manifest e sua ref — delete_repo não devia ter
        // tocado nisso, já que o digest continua em uso por outro repo.
        assert!(get_manifest(&db, &repo_b.id, digest).await.unwrap().is_some());
        assert!(ref_blob_or_manifest_exists(&db, "blobref").await.unwrap());
        let (ref_count,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM registry_manifest_refs WHERE manifest_digest = ?")
                .bind(digest)
                .fetch_one(&db)
                .await
                .unwrap();
        assert_eq!(ref_count, 1, "refs do manifest ainda usado por repo_b foram apagadas");
    }
}
