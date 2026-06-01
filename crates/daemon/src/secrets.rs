use anyhow::{Result, anyhow};
use std::{
    io::{Read, Write},
    path::Path,
};

pub struct SecretsManager {
    passphrase: String,
}

impl SecretsManager {
    pub fn new(master_key_path: &Path) -> Result<Self> {
        let passphrase = if master_key_path.exists() {
            std::fs::read_to_string(master_key_path)?.trim().to_string()
        } else {
            let key = generate_key();
            if let Some(parent) = master_key_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(master_key_path, &key)?;
            key
        };
        Ok(Self { passphrase })
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

    pub async fn get_raw(&self, _secret_name: &str) -> Result<String> {
        Err(anyhow!("secret lookup not yet wired to db"))
    }
}

fn generate_key() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut hasher = DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut hasher);
    std::process::id().hash(&mut hasher);

    format!(
        "{:016x}{:016x}",
        hasher.finish(),
        hasher.finish() ^ 0xdeadbeef
    )
}
