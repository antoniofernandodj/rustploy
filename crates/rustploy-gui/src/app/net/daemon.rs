//! Daemon-wide operations that aren't scoped to one project/service: the
//! topbar "Stop All" and the Settings screen's web-server fields.

use super::{with_outcome_toast, RwpClient};
use shared::{Command, Response};

pub struct Daemon {
    client: RwpClient,
}

impl Daemon {
    pub fn new(client: RwpClient) -> Self {
        Self { client }
    }

    /// Stops every running service across all projects (topbar "Stop All").
    /// Stops every rustploy-managed service in one round trip
    /// (`Command::StopAllManaged`) — the daemon reuses `service_stop`'s real
    /// Docker-level container lookup for each one, so it doesn't miss services
    /// whose DB status has drifted from what's actually running. Never
    /// touches containers unrelated to rustploy on the same Docker host.
    pub async fn stop_all(self) -> Vec<(String, String)> {
        let msg = match self.client.rpc(Command::StopAllManaged).await {
            Ok(Response::StopAllResult { count }) => format!("{count} serviço(s) parado(s)"),
            Ok(other) => super::view::resp_msg(&other),
            Err(e) => format!("erro: {e}"),
        };
        with_outcome_toast(vec![("status_line".into(), msg.clone())], &msg)
    }

    /// Persists the daemon settings (Settings screen). Empty strings clear a field.
    pub async fn save_settings(self, domain: String, email: String) -> Vec<(String, String)> {
        let opt = |s: String| if s.trim().is_empty() { None } else { Some(s) };
        let cmd = Command::SetDaemonSettings {
            webhook_base_url: opt(domain),
            acme_email: opt(email),
        };
        let msg = match self.client.rpc(cmd).await {
            Ok(Response::Ok) => "configurações salvas".to_string(),
            Ok(other) => format!("{other:?}"),
            Err(e) => format!("erro: {e}"),
        };
        with_outcome_toast(vec![("settings_msg".into(), msg.clone())], &msg)
    }
}
