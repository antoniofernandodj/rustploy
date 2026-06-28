//! Persistence of connection preferences (remembered server URL and access
//! token), stored as JSON under the user data dir so the login screen can
//! prefill on the next launch.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Prefs {
    #[serde(default)]
    pub remember_url: bool,
    #[serde(default)]
    pub remember_token: bool,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
}

fn prefs_path() -> PathBuf {
    shared::fallback_data_dir().join("remote-ui.json")
}

impl Prefs {
    /// Reads saved preferences, falling back to defaults when missing/unreadable.
    pub fn load() -> Self {
        std::fs::read_to_string(prefs_path())
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    }

    /// Writes the preferences to disk (best-effort; errors ignored).
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
