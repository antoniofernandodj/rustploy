//! Persistence of remote-client connection preferences: the optionally
//! remembered server URL (`rwp://host[:port]`) and access token. Stored as JSON
//! under the user data dir so the connect screen can prefill on the next launch.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemotePrefs {
    #[serde(default, alias = "remember_address")]
    pub remember_url: bool,
    #[serde(default)]
    pub remember_token: bool,
    #[serde(default, alias = "address")]
    pub url: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
}

fn prefs_path() -> PathBuf {
    let data_dir = shared::fallback_data_dir();
    data_dir.join("remote-client.json")
}

impl RemotePrefs {
    /// Reads the saved preferences, falling back to defaults when the file is
    /// missing or unreadable.
    pub fn load() -> Self {
        std::fs::read_to_string(prefs_path())
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    /// Writes the preferences to disk (best-effort; errors are ignored).
    pub fn save(&self) {
        let path = prefs_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }
}
