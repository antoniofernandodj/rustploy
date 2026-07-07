//! Local persistence, stored as JSON under the user data dir: connection
//! preferences ([`Prefs`], remembered server URL/token for the login screen)
//! and window geometry ([`WindowState`], remembered size/position).

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
    shared::fallback_data_dir().join("rustploy-gui.json")
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

/// Last known window geometry, remembered across launches so the app reopens
/// at the same size and position instead of always resetting to the default
/// 1280×820. A separate file from [`Prefs`]: it's written from window-event
/// handling in `App` (`super::App::save_window_state`, not the glacier-ui
/// context), on a different lifecycle (on close, not per-field-change).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WindowState {
    pub width: f32,
    pub height: f32,
    /// Absent until the platform reports a position at least once. Some
    /// compositors (notably Wayland) never expose window position at all, in
    /// which case this stays `None` forever and the window falls back to the
    /// platform-default placement on every launch — there is no workaround.
    pub x: Option<f32>,
    pub y: Option<f32>,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: 1280.0,
            height: 820.0,
            x: None,
            y: None
        }
    }
}

fn window_state_path() -> PathBuf {
    shared::fallback_data_dir()
        .join("rustploy-gui-window.json")
}

impl WindowState {
    /// Reads the saved geometry, falling back to the default size (and no
    /// saved position) when missing/unreadable.
    pub fn load() -> Self {
        std::fs::read_to_string(window_state_path())
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())    
            .unwrap_or_default()
    }

    /// Writes the geometry to disk (best-effort; errors ignored).
    pub fn save(&self) {
        let path = window_state_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }
}
