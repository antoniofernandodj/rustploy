//! Garbage collection do registry: libera do disco o que nenhuma tag alcança.
//! Disparado manualmente (`Command::RegistryGc`, botão na sub-aba Registry) e
//! pelo job diário em `main.rs` (junto do trim do event_log).
//!
//! Duas fases, ambas com a `commit_lock` do storage segurada do início ao fim
//! (ver o comentário em `storage.rs` — sem ela, um blob commitado entre o
//! snapshot e o sweep seria apagado como órfão):
//!
//! 1. **Metadados** (`db::registry::gc_metadata`): apaga manifests pendurados,
//!    refs órfãs e blobs sem ref, numa transação.
//! 2. **Disco**: varre o CAS apagando arquivos fora do conjunto vivo
//!    (`all_cas_digests`) e limpa `uploads/` órfãos com mais de 24 h (sobras
//!    de restart — sessões de upload vivem só em memória).
//!
//! Janela conhecida (aceita na Fase 1, single-admin): um push com blobs já
//! finalizados mas manifest ainda não enviado tem os blobs sem ref — um GC
//! nesse exato momento os apaga e o PUT do manifest falha com BLOB_UNKNOWN;
//! basta repetir o `docker push`.

use std::collections::HashSet;
use std::time::Duration;

use super::storage::RegistryStorage;
use crate::db::{registry as db_registry, Db};

/// Idade mínima de um arquivo órfão em `uploads/` para o GC apagar — folga
/// generosa para uploads legitimamente lentos ainda em andamento.
const UPLOAD_TTL: Duration = Duration::from_secs(24 * 3600);

pub struct GcResult {
    /// Arquivos removidos do disco (CAS + uploads órfãos).
    pub blobs_removed: u64,
    pub bytes_freed: u64,
}

pub async fn run(db: &Db, storage: &RegistryStorage) -> anyhow::Result<GcResult> {
    let _commit = storage.lock_commit().await;
    db_registry::gc_metadata(db).await?;
    let live: HashSet<String> = db_registry::all_cas_digests(db).await?.into_iter().collect();
    let (cas_removed, cas_bytes) = storage.sweep_orphan_files(&live).await?;
    let (up_removed, up_bytes) = storage.clean_stale_uploads(UPLOAD_TTL).await?;
    Ok(GcResult {
        blobs_removed: cas_removed + up_removed,
        bytes_freed: cas_bytes + up_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ulid::Ulid;

    async fn test_db() -> Db {
        let dir = std::env::temp_dir().join(format!("rustploy_test_gc_{}", Ulid::new()));
        crate::db::connect(&dir).await.unwrap()
    }

    fn tmp_root() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("rp_registry_gc_test_{}", Ulid::new()))
    }

    /// O invariante central: GC não apaga blob referenciado — nem do DB nem
    /// do disco — e apaga o órfão de ambos.
    #[tokio::test]
    async fn gc_apaga_orfao_e_preserva_referenciado() {
        let db = test_db().await;
        let root = tmp_root();
        let storage = RegistryStorage::new(root).unwrap();

        let repo = db_registry::get_or_create_repo(&db, "app").await.unwrap();
        let vivo = storage.write_blob_direct(b"camada viva").await.unwrap();
        db_registry::insert_blob(&db, &vivo.digest, vivo.size as i64).await.unwrap();
        let orfao = storage.write_blob_direct(b"camada orfa").await.unwrap();
        db_registry::insert_blob(&db, &orfao.digest, orfao.size as i64).await.unwrap();

        let manifest = storage.write_blob_direct(br#"{"schemaVersion":2}"#).await.unwrap();
        db_registry::insert_manifest(
            &db,
            &manifest.digest,
            &repo.id,
            "application/json",
            manifest.size as i64,
            &[vivo.digest.clone()],
        )
        .await
        .unwrap();
        db_registry::upsert_tag(&db, &repo.id, "latest", &manifest.digest).await.unwrap();

        let result = run(&db, &storage).await.unwrap();
        assert_eq!(result.blobs_removed, 1);
        assert_eq!(result.bytes_freed, orfao.size);
        assert!(storage.blob_exists(&vivo.digest).await);
        assert!(storage.blob_exists(&manifest.digest).await, "manifest taggeado sumiu do CAS");
        assert!(!storage.blob_exists(&orfao.digest).await);
        assert!(db_registry::blob_exists(&db, &vivo.digest).await.unwrap());
        assert!(!db_registry::blob_exists(&db, &orfao.digest).await.unwrap());

        // Segunda passada: nada mais a remover.
        let again = run(&db, &storage).await.unwrap();
        assert_eq!(again.blobs_removed, 0);
        assert_eq!(again.bytes_freed, 0);
    }

    /// Depois de remover o repo pela UI (`delete_repo`, só metadados), o GC
    /// libera os arquivos do CAS — o fluxo exato do botão "Executar GC".
    #[tokio::test]
    async fn gc_libera_disco_apos_delete_repo() {
        let db = test_db().await;
        let storage = RegistryStorage::new(tmp_root()).unwrap();

        let repo = db_registry::get_or_create_repo(&db, "hello").await.unwrap();
        let blob = storage.write_blob_direct(b"layer bytes").await.unwrap();
        db_registry::insert_blob(&db, &blob.digest, blob.size as i64).await.unwrap();
        let manifest = storage.write_blob_direct(br#"{"schemaVersion":2,"x":1}"#).await.unwrap();
        db_registry::insert_manifest(
            &db,
            &manifest.digest,
            &repo.id,
            "application/json",
            manifest.size as i64,
            &[blob.digest.clone()],
        )
        .await
        .unwrap();
        db_registry::upsert_tag(&db, &repo.id, "latest", &manifest.digest).await.unwrap();

        assert!(db_registry::delete_repo(&db, &repo.id).await.unwrap());
        let result = run(&db, &storage).await.unwrap();
        assert_eq!(result.blobs_removed, 2, "blob + manifest deviam sair do CAS");
        assert_eq!(result.bytes_freed, blob.size + manifest.size);
        assert!(!storage.blob_exists(&blob.digest).await);
        assert!(!storage.blob_exists(&manifest.digest).await);
    }

    /// `uploads/`: órfão velho sai; sessão ATIVA fica mesmo com mtime antigo;
    /// órfão recente fica (dentro do TTL de 24 h).
    #[tokio::test]
    async fn gc_limpa_uploads_orfaos_mas_nao_sessao_ativa() {
        let db = test_db().await;
        let root = tmp_root();
        let storage = RegistryStorage::new(root.clone()).unwrap();

        let old_mtime = std::time::SystemTime::now() - Duration::from_secs(25 * 3600);

        // Órfão velho (sem sessão no mapa): deve sair.
        let stale = root.join("uploads").join("stale_upload");
        std::fs::write(&stale, b"abandonado").unwrap();
        std::fs::File::options()
            .write(true)
            .open(&stale)
            .unwrap()
            .set_modified(old_mtime)
            .unwrap();

        // Sessão ativa com mtime igualmente velho: deve FICAR.
        let active_id = storage.start_upload().await.unwrap();
        storage.write_chunk(&active_id, b"em andamento").await.unwrap();
        let active_path = root.join("uploads").join(&active_id);
        std::fs::File::options()
            .write(true)
            .open(&active_path)
            .unwrap()
            .set_modified(old_mtime)
            .unwrap();

        // Órfão recente: deve ficar (ainda dentro do TTL).
        std::fs::write(root.join("uploads").join("recente"), b"novo").unwrap();

        let result = run(&db, &storage).await.unwrap();
        assert_eq!(result.blobs_removed, 1);
        assert_eq!(result.bytes_freed, "abandonado".len() as u64);
        assert!(!stale.exists());
        assert!(active_path.exists());
        assert!(root.join("uploads").join("recente").exists());
    }
}
