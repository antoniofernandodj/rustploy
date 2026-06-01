use crate::api::AppState;
use bollard::container::{LogOutput, LogsOptions};
use chrono::Utc;
use futures::StreamExt;
use shared::{
    protocol::{LogEntry, LogStream},
    Response as RpResponse, ServiceSource,
};
use tokio::io::AsyncWriteExt;

pub async fn handle(state: AppState, service_id: String, tail: usize) -> RpResponse {
    let svc = match crate::db::services::get(&state.db, &service_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return RpResponse::err("NotFound", "serviço não encontrado"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    if let ServiceSource::Compose(compose) = &svc.spec.source {
        return compose_logs(&svc.spec.name, &compose.content, tail).await;
    }

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

async fn compose_logs(service_name: &str, content: &str, tail: usize) -> RpResponse {
    use tokio::process::Command;

    let project_name = format!("rp_{}", service_name);

    let mut child = match Command::new("docker")
        .args([
            "compose", "-p", &project_name,
            "-f", "-",
            "logs",
            "--no-color",
            "--tail", &tail.to_string(),
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return RpResponse::err("DockerError", format!("falha ao buscar logs: {e}")),
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(content.as_bytes()).await;
        stdin.shutdown().await.ok();
    }

    let output = match child.wait_with_output().await {
        Ok(o) => o,
        Err(e) => return RpResponse::err("DockerError", e.to_string()),
    };

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let entries: Vec<LogEntry> = combined
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| LogEntry {
            stream: LogStream::Stdout,
            line: line.to_string(),
            timestamp: Utc::now(),
        })
        .collect();

    RpResponse::Logs(entries)
}
