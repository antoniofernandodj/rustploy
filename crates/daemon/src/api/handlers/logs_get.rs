use crate::api::AppState;
use bollard::container::{LogOutput, LogsOptions};
use chrono::Utc;
use futures::StreamExt;
use shared::{protocol::{LogEntry, LogStream}, Response as RpResponse};

pub async fn handle(state: AppState, service_id: String, tail: usize) -> RpResponse {
    let svc = match crate::db::services::get(&state.db, &service_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return RpResponse::err("NotFound", "serviço não encontrado"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    let container_id = match &svc.live_container_id {
        Some(id) => id.clone(),
        None => return RpResponse::Logs(vec![]),
    };

    let opts = LogsOptions::<String> {
        follow: false,
        stdout: true,
        stderr: true,
        tail: tail.to_string(),
        ..Default::default()
    };

    let mut entries = vec![];
    let mut stream = state.docker.inner.logs(&container_id, Some(opts));

    while let Some(item) = stream.next().await {
        let (is_stderr, bytes) = match item {
            Ok(LogOutput::StdOut { message }) => (false, message),
            Ok(LogOutput::StdErr { message }) => (true, message),
            Ok(_) => continue,
            Err(_) => break,
        };
        let line = String::from_utf8_lossy(&bytes).trim_end_matches('\n').to_string();
        if !line.is_empty() {
            entries.push(LogEntry {
                stream: if is_stderr { LogStream::Stderr } else { LogStream::Stdout },
                line,
                timestamp: Utc::now(),
            });
        }
    }

    RpResponse::Logs(entries)
}
