//! CAS (content-addressable store) do registry: blobs em disco, sessões de
//! upload em memória com hash incremental sha256. Layout:
//!
//! ```text
//! <root>/blobs/sha256/<2 primeiros hex>/<digest completo>
//! <root>/uploads/<ulid da sessão>
//! ```

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Mutex, RwLock};
use ulid::Ulid;

#[derive(Debug)]
pub struct BlobInfo {
    pub digest: String,
    pub size: u64,
}

#[derive(Debug)]
pub enum StorageError {
    UnknownUpload,
    DigestMismatch { expected: String, got: String },
    Io(std::io::Error),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::UnknownUpload => write!(f, "unknown upload session"),
            StorageError::DigestMismatch { expected, got } => {
                write!(f, "digest mismatch: expected {expected}, got {got}")
            }
            StorageError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for StorageError {}

struct UploadSession {
    file: tokio::fs::File,
    tmp_path: PathBuf,
    hasher: Sha256,
    written: u64,
}

/// Blob store content-addressable + registro de sessões de upload em voo.
///
/// `uploads` é `RwLock` no mapa (lookups por uuid, em PATCH/PUT/DELETE, são
/// muito mais frequentes que start/cancel) com `Mutex` por sessão (serializa
/// só os chunks de UM upload — o `docker` CLI nunca faz PATCH concorrente no
/// mesmo uuid — sem bloquear uploads de outras camadas em paralelo).
pub struct RegistryStorage {
    root: PathBuf,
    uploads: RwLock<HashMap<String, Mutex<UploadSession>>>,
    /// Trava de commit compartilhada entre os pontos que COMMITAM conteúdo
    /// (finalize de upload de blob, PUT de manifest — operações curtas, só o
    /// rename/insert, nunca o streaming) e o GC (`crate::registry::gc`), que a
    /// segura do snapshot de digests vivos até o fim do sweep — sem ela, um
    /// blob finalizado no meio do sweep não estaria no snapshot e seria
    /// apagado como órfão.
    commit_lock: Mutex<()>,
}

impl RegistryStorage {
    pub fn new(root: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(root.join("blobs").join("sha256"))?;
        std::fs::create_dir_all(root.join("uploads"))?;
        Ok(Self {
            root,
            uploads: RwLock::new(HashMap::new()),
            commit_lock: Mutex::new(()),
        })
    }

    /// Ver `commit_lock`. Adquirir ANTES de qualquer validação/escrita da
    /// operação de commit e soltar (drop do guard) só depois do insert nos
    /// metadados.
    pub async fn lock_commit(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.commit_lock.lock().await
    }

    pub fn digest_path(&self, digest_hex: &str) -> PathBuf {
        self.root
            .join("blobs")
            .join("sha256")
            .join(&digest_hex[..2])
            .join(digest_hex)
    }

    pub async fn blob_exists(&self, digest_hex: &str) -> bool {
        tokio::fs::try_exists(self.digest_path(digest_hex))
            .await
            .unwrap_or(false)
    }

    pub async fn open_blob(&self, digest_hex: &str) -> std::io::Result<tokio::fs::File> {
        tokio::fs::File::open(self.digest_path(digest_hex)).await
    }

    pub async fn blob_len(&self, digest_hex: &str) -> std::io::Result<u64> {
        Ok(tokio::fs::metadata(self.digest_path(digest_hex))
            .await?
            .len())
    }

    /// Inicia uma sessão de upload; retorna o ID (ULID) usado na URL
    /// (`Docker-Upload-UUID` / `/v2/<name>/blobs/uploads/<id>`).
    pub async fn start_upload(&self) -> std::io::Result<String> {
        let id = Ulid::new().to_string();
        let tmp_path = self.root.join("uploads").join(&id);
        let file = tokio::fs::File::create(&tmp_path).await?;
        self.uploads.write().await.insert(
            id.clone(),
            Mutex::new(UploadSession {
                file,
                tmp_path,
                hasher: Sha256::new(),
                written: 0,
            }),
        );
        Ok(id)
    }

    /// Append de um chunk; retorna o total de bytes já escritos (para o
    /// header `Range: bytes=0-<written-1>` da resposta 202).
    pub async fn write_chunk(&self, id: &str, data: &[u8]) -> Result<u64, StorageError> {
        let map = self.uploads.read().await;
        let session = map.get(id).ok_or(StorageError::UnknownUpload)?;
        let mut s = session.lock().await;
        s.file.write_all(data).await.map_err(StorageError::Io)?;
        s.hasher.update(data);
        s.written += data.len() as u64;
        Ok(s.written)
    }

    /// Finaliza: confere digest, fsync, rename atômico para o CAS. Se o
    /// destino já existir (upload concorrente do mesmo conteúdo já
    /// terminou), apenas descarta o temporário — idempotente.
    pub async fn finalize_upload(
        &self,
        id: &str,
        expected_digest_hex: &str,
    ) -> Result<BlobInfo, StorageError> {
        let session = self
            .uploads
            .write()
            .await
            .remove(id)
            .ok_or(StorageError::UnknownUpload)?;
        let s = session.into_inner();
        let got = hex::encode(s.hasher.finalize());
        if got != expected_digest_hex {
            let _ = tokio::fs::remove_file(&s.tmp_path).await;
            return Err(StorageError::DigestMismatch {
                expected: expected_digest_hex.to_string(),
                got,
            });
        }
        let mut file = s.file;
        file.flush().await.map_err(StorageError::Io)?;
        drop(file);

        let dest = self.digest_path(&got);
        tokio::fs::create_dir_all(dest.parent().expect("digest_path has a parent"))
            .await
            .map_err(StorageError::Io)?;
        if tokio::fs::try_exists(&dest).await.unwrap_or(false) {
            let _ = tokio::fs::remove_file(&s.tmp_path).await;
        } else if let Err(e) = tokio::fs::rename(&s.tmp_path, &dest).await {
            let _ = tokio::fs::remove_file(&s.tmp_path).await;
            return Err(StorageError::Io(e));
        }
        Ok(BlobInfo {
            digest: got,
            size: s.written,
        })
    }

    pub async fn cancel_upload(&self, id: &str) -> Result<(), StorageError> {
        let session = self
            .uploads
            .write()
            .await
            .remove(id)
            .ok_or(StorageError::UnknownUpload)?;
        let _ = tokio::fs::remove_file(&session.into_inner().tmp_path).await;
        Ok(())
    }

    /// Escreve um blob a partir de bytes já em memória, sem sessão de upload
    /// — usado pelo `PUT` de manifest (corpo lido e validado inteiro antes de
    /// gravar). Idempotente: conteúdo igual produz o mesmo digest e um
    /// segundo `write_blob_direct` do mesmo blob não corrompe o já gravado.
    pub async fn write_blob_direct(&self, data: &[u8]) -> Result<BlobInfo, StorageError> {
        let digest = hex::encode(Sha256::digest(data));
        let dest = self.digest_path(&digest);
        if !tokio::fs::try_exists(&dest).await.unwrap_or(false) {
            tokio::fs::create_dir_all(dest.parent().expect("digest_path has a parent"))
                .await
                .map_err(StorageError::Io)?;
            let tmp = dest.with_extension("tmp");
            tokio::fs::write(&tmp, data)
                .await
                .map_err(StorageError::Io)?;
            if tokio::fs::try_exists(&dest).await.unwrap_or(false) {
                let _ = tokio::fs::remove_file(&tmp).await;
            } else {
                tokio::fs::rename(&tmp, &dest)
                    .await
                    .map_err(StorageError::Io)?;
            }
        }
        Ok(BlobInfo {
            digest,
            size: data.len() as u64,
        })
    }

    /// Lê um blob inteiro em memória — usado para servir manifests (pequenos,
    /// já têm teto de 4 MiB no PUT) sem precisar de streaming.
    pub async fn read_blob(&self, digest_hex: &str) -> std::io::Result<Vec<u8>> {
        let mut file = self.open_blob(digest_hex).await?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).await?;
        Ok(buf)
    }

    /// Sweep do GC: remove do CAS todo arquivo cujo nome (digest) não está em
    /// `live` — cobre blobs órfãos e também os `.tmp` de `write_blob_direct`
    /// interrompidos. Retorna `(arquivos removidos, bytes liberados)`. Chamar
    /// SÓ com a `commit_lock` adquirida (ver `lock_commit`).
    pub async fn sweep_orphan_files(&self, live: &HashSet<String>) -> std::io::Result<(u64, u64)> {
        let base = self.root.join("blobs").join("sha256");
        let (mut removed, mut bytes) = (0u64, 0u64);
        let mut prefixes = tokio::fs::read_dir(&base).await?;
        while let Some(prefix) = prefixes.next_entry().await? {
            if !prefix.file_type().await?.is_dir() {
                continue;
            }
            let mut files = tokio::fs::read_dir(prefix.path()).await?;
            while let Some(f) = files.next_entry().await? {
                let name = f.file_name().to_string_lossy().into_owned();
                if live.contains(&name) {
                    continue;
                }
                let size = f.metadata().await.map(|m| m.len()).unwrap_or(0);
                if tokio::fs::remove_file(f.path()).await.is_ok() {
                    removed += 1;
                    bytes += size;
                }
            }
        }
        Ok((removed, bytes))
    }

    /// Remove de `uploads/` arquivos órfãos: sem sessão ativa no mapa (sobra
    /// de restart do daemon — as sessões vivem só em memória) e mais velhos
    /// que `max_age`. Retorna `(arquivos removidos, bytes liberados)`.
    pub async fn clean_stale_uploads(
        &self,
        max_age: std::time::Duration,
    ) -> std::io::Result<(u64, u64)> {
        let active: HashSet<String> = self.uploads.read().await.keys().cloned().collect();
        let now = std::time::SystemTime::now();
        let (mut removed, mut bytes) = (0u64, 0u64);
        let mut files = tokio::fs::read_dir(self.root.join("uploads")).await?;
        while let Some(f) = files.next_entry().await? {
            let name = f.file_name().to_string_lossy().into_owned();
            if active.contains(&name) {
                continue;
            }
            let Ok(meta) = f.metadata().await else {
                continue;
            };
            let old_enough = meta
                .modified()
                .ok()
                .and_then(|m| now.duration_since(m).ok())
                .is_some_and(|age| age > max_age);
            if old_enough && tokio::fs::remove_file(f.path()).await.is_ok() {
                removed += 1;
                bytes += meta.len();
            }
        }
        Ok((removed, bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root() -> PathBuf {
        std::env::temp_dir().join(format!("rp_registry_storage_test_{}", Ulid::new()))
    }

    fn sha256_hex(data: &[u8]) -> String {
        hex::encode(Sha256::digest(data))
    }

    #[tokio::test]
    async fn upload_em_chunks_produz_digest_correto() {
        let storage = RegistryStorage::new(tmp_root()).unwrap();
        let id = storage.start_upload().await.unwrap();
        let part1 = b"hello ".to_vec();
        let part2 = b"world".to_vec();
        storage.write_chunk(&id, &part1).await.unwrap();
        storage.write_chunk(&id, &part2).await.unwrap();

        let full = [part1, part2].concat();
        let expected = sha256_hex(&full);

        let info = storage.finalize_upload(&id, &expected).await.unwrap();
        assert_eq!(info.digest, expected);
        assert_eq!(info.size, full.len() as u64);
        assert!(storage.blob_exists(&expected).await);
        assert_eq!(storage.blob_len(&expected).await.unwrap(), full.len() as u64);
    }

    #[tokio::test]
    async fn digest_incorreto_e_rejeitado_e_nao_deixa_lixo() {
        let storage = RegistryStorage::new(tmp_root()).unwrap();
        let id = storage.start_upload().await.unwrap();
        storage.write_chunk(&id, b"conteudo").await.unwrap();

        let wrong = "0".repeat(64);
        let err = storage.finalize_upload(&id, &wrong).await.unwrap_err();
        assert!(matches!(err, StorageError::DigestMismatch { .. }));
        assert!(!storage.blob_exists(&wrong).await);

        // A sessão foi consumida (removida do mapa) mesmo em erro — não deve
        // sobrar entrada "presa"; uma segunda finalização é UnknownUpload.
        let err2 = storage.finalize_upload(&id, &wrong).await.unwrap_err();
        assert!(matches!(err2, StorageError::UnknownUpload));
    }

    #[tokio::test]
    async fn finalize_duas_vezes_do_mesmo_conteudo_e_idempotente() {
        let storage = RegistryStorage::new(tmp_root()).unwrap();
        let content = b"mesmo conteudo, duas sessoes concorrentes";
        let expected = sha256_hex(content);

        let id1 = storage.start_upload().await.unwrap();
        storage.write_chunk(&id1, content).await.unwrap();
        let id2 = storage.start_upload().await.unwrap();
        storage.write_chunk(&id2, content).await.unwrap();

        let info1 = storage.finalize_upload(&id1, &expected).await.unwrap();
        let info2 = storage.finalize_upload(&id2, &expected).await.unwrap();
        assert_eq!(info1.digest, info2.digest);
        assert_eq!(info1.size, info2.size);

        let bytes = storage.read_blob(&expected).await.unwrap();
        assert_eq!(bytes, content);
    }

    #[tokio::test]
    async fn cancel_remove_sessao_e_arquivo_temp() {
        let storage = RegistryStorage::new(tmp_root()).unwrap();
        let id = storage.start_upload().await.unwrap();
        storage.write_chunk(&id, b"parcial").await.unwrap();
        storage.cancel_upload(&id).await.unwrap();

        let err = storage.write_chunk(&id, b"mais").await.unwrap_err();
        assert!(matches!(err, StorageError::UnknownUpload));
    }

    #[tokio::test]
    async fn write_blob_direct_e_idempotente_para_mesmo_conteudo() {
        let storage = RegistryStorage::new(tmp_root()).unwrap();
        let data = br#"{"schemaVersion":2}"#;
        let info1 = storage.write_blob_direct(data).await.unwrap();
        let info2 = storage.write_blob_direct(data).await.unwrap();
        assert_eq!(info1.digest, info2.digest);
        let bytes = storage.read_blob(&info1.digest).await.unwrap();
        assert_eq!(bytes, data);
    }
}
