use anyhow::{Result, anyhow};
use std::{
    io::{Read, Write},
    path::Path,
    sync::Arc,
};
use ulid::Ulid;

use crate::db::Db;

pub struct SecretsManager {
    passphrase: String,
    db: Arc<Db>,
}

impl SecretsManager {
    pub fn new(master_key_path: &Path, db: Arc<Db>) -> Result<Self> {
        let passphrase = if master_key_path.exists() {
            std::fs::read_to_string(master_key_path)?.trim().to_string()
        } else {
            let key = generate_key()?;
            if let Some(parent) = master_key_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(master_key_path, &key)?;
            key
        };
        Ok(Self { passphrase, db })
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        let secret = age::secrecy::SecretString::new(self.passphrase.clone().into());
        let encryptor = age::Encryptor::with_user_passphrase(secret);
        let mut encrypted = Vec::new();
        {
            let mut writer = encryptor.wrap_output(&mut encrypted)?;
            writer.write_all(plaintext.as_bytes())?;
            writer.finish()?;
        }
        Ok(hex::encode(&encrypted))
    }

    pub fn decrypt(&self, ciphertext_hex: &str) -> Result<String> {
        let ciphertext = hex::decode(ciphertext_hex)?;
        let decryptor = match age::Decryptor::new(&ciphertext[..])? {
            age::Decryptor::Passphrase(d) => d,
            _ => return Err(anyhow!("unexpected decryptor type")),
        };
        let secret = age::secrecy::SecretString::new(self.passphrase.clone().into());
        let mut decrypted = Vec::new();
        let mut reader = decryptor.decrypt(&secret, None)?;
        reader.read_to_end(&mut decrypted)?;
        Ok(String::from_utf8(decrypted)?)
    }

    pub async fn get_raw(&self, project_id: &str, name: &str) -> Result<String> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM secret WHERE project_id = ? AND key = ?",
        )
        .bind(project_id)
        .bind(name)
        .fetch_optional(&*self.db)
        .await?;

        let encrypted = row
            .ok_or_else(|| anyhow!("secret '{name}' not found in project '{project_id}'"))?
            .0;
        self.decrypt(&encrypted)
    }

    pub async fn set(&self, project_id: &str, name: &str, value: &str) -> Result<()> {
        let id = Ulid::new().to_string();
        let encrypted = self.encrypt(value)?;
        sqlx::query(
            "INSERT INTO secret (id, project_id, key, value) VALUES (?, ?, ?, ?)
             ON CONFLICT(project_id, key) DO UPDATE SET value = excluded.value",
        )
        .bind(&id)
        .bind(project_id)
        .bind(name)
        .bind(&encrypted)
        .execute(&*self.db)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, project_id: &str, name: &str) -> Result<()> {
        sqlx::query("DELETE FROM secret WHERE project_id = ? AND key = ?")
            .bind(project_id)
            .bind(name)
            .execute(&*self.db)
            .await?;
        Ok(())
    }

    pub async fn list_names(&self, project_id: &str) -> Result<Vec<String>> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT key FROM secret WHERE project_id = ? ORDER BY key")
                .bind(project_id)
                .fetch_all(&*self.db)
                .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }
}

fn generate_key() -> Result<String> {
    let mut buf = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut buf).map(|_| ()))
        .map_err(|e| anyhow!("failed to read from /dev/urandom: {e}"))?;
    Ok(hex::encode(buf))
}
