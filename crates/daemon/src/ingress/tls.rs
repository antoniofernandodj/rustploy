// TLS certificate management stub — full ACME integration in Phase 5
use anyhow::Result;
use tracing::info;

pub struct TlsManager;

impl TlsManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn ensure_cert(&self, domain: &str) -> Result<()> {
        info!(domain, "TLS: ensure_cert called (ACME not yet implemented)");
        Ok(())
    }

    pub async fn renew_expiring(&self) -> Result<Vec<String>> {
        Ok(vec![])
    }
}

impl Default for TlsManager {
    fn default() -> Self {
        Self::new()
    }
}
