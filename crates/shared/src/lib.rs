pub mod config;
pub mod manifest;
pub mod models;
pub mod protocol;
pub mod templates;
pub mod wizard;

pub use config::{fallback_data_dir, user_home, ApiConfig, RegistryConfig, RustployConfig};

/// Unique Docker Compose project name for a rustploy service.
/// Incorporates the first 8 chars of the service ULID to avoid collisions
/// between services with the same user-facing name in different projects.
pub fn compose_project_name(svc_id: &str, svc_name: &str) -> String {
    let id_part = svc_id
        .strip_prefix("svc_")
        .unwrap_or(svc_id)
        .get(..8)
        .unwrap_or(svc_id)
        .to_lowercase();
    let safe: String = svc_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    let safe = safe.trim_matches('_');
    format!("rp_{id_part}_{safe}")
}
pub use manifest::{
    format_dotenv, parse_dotenv, ActionVerb, ApplyReport, ProjectEntry, ProjectManifest,
    ResourceAction, ResourceActionKind, ServerManifest, ServiceManifest,
};
pub use models::*;
pub use protocol::{ClientFrame, Command, Event, Response};
pub use wizard::WizardCreateReq;
