use crate::api::AppState;
use shared::Response as RpResponse;

pub async fn list(state: AppState) -> RpResponse {
    match crate::env_backup::list_snapshots(&state.backup_dir).await {
        Ok(names) => RpResponse::EnvBackupSnapshots(names),
        Err(e) => RpResponse::err("BackupError", e.to_string()),
    }
}

pub async fn restore(state: AppState, snapshot: String) -> RpResponse {
    match crate::env_backup::restore_snapshot(&state.db, &state.backup_dir, &snapshot).await {
        Ok(n) => {
            tracing::info!(snapshot, restored = n, "env vars restauradas");
            RpResponse::Ok
        }
        Err(e) => RpResponse::err("RestoreError", e.to_string()),
    }
}
