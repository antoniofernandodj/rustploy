//! Token interno usado pelo próprio deploy executor pra puxar imagens do
//! registry embutido, sem ação manual do usuário. Regenerado a cada boot
//! (ver comentário em db/registry_tokens.rs sobre por que não é fixo).

use crate::db::{registry_tokens, Db};
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::sync::Arc;

pub async fn ensure(db: &Db) -> Result<Arc<str>> {
    let mut bytes = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .unwrap_or_default();
    let secret = hex::encode(bytes);
    let hash = hex::encode(Sha256::digest(secret.as_bytes()));
    registry_tokens::upsert_internal(db, &hash).await?;
    Ok(Arc::from(secret.as_str()))
}
