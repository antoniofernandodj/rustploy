use crate::api::AppState;
use bollard::container::{LogOutput, LogsOptions};
use chrono::Utc;
use futures::StreamExt;
use shared::{
    EnvVarValue, Response as RpResponse, ServiceSource,
    protocol::{LogEntry, LogStream},
};
use tokio::io::AsyncWriteExt;

pub async fn handle(state: AppState, service_id: String, tail: usize) -> RpResponse {
    let svc = match crate::db::services::get(&state.db, &service_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return RpResponse::err("NotFound", "serviço não encontrado"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    if let ServiceSource::Compose(compose) = &svc.spec.source {
        let pid = &svc.spec.project_id;
        let mut env_vars: Vec<(String, String)> = Vec::new();
        for ev in &svc.spec.env_vars {
            let value = match &ev.value {
                EnvVarValue::Plain(v) => v.clone(),
                EnvVarValue::Secret(name) => {
                    state.secrets.get_raw(pid, name).await.unwrap_or_default()
                }
            };
            env_vars.push((ev.key.clone(), value));
        }
        let project_name = crate::docker::compose::compose_project_name(&service_id, &svc.spec.name);
        return compose_logs(&project_name, &compose.content, tail, &env_vars).await;
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
        let line = String::from_utf8_lossy(&bytes)
            .trim_end_matches('\n')
            .to_string();
        if !line.is_empty() {
            entries.push(LogEntry {
                stream: if is_stderr {
                    LogStream::Stderr
                } else {
                    LogStream::Stdout
                },
                line,
                timestamp: Utc::now(),
            });
        }
    }

    RpResponse::Logs(entries)
}

/// Extracts the RFC3339 timestamp from a `docker compose logs --timestamps` line.
/// Format: `service-1  | 2026-06-26T22:24:58.123456789Z message`
fn parse_compose_log_ts(line: &str) -> Option<chrono::DateTime<Utc>> {
    // Find the `| ` separator and look for a timestamp immediately after it.
    let after_pipe = line.split_once("| ")?.1;
    // The timestamp ends at the first space.
    let ts_str = after_pipe.split_whitespace().next()?;
    ts_str.parse::<chrono::DateTime<Utc>>().ok()
}

async fn compose_logs(project_name: &str, content: &str, tail: usize, env_vars: &[(String, String)]) -> RpResponse {
    use tokio::process::Command;

    let mut child = match Command::new("docker")
        .args([
            "compose",
            "-p",
            project_name,
            "-f",
            "-",
            "logs",
            "--no-color",
            "--timestamps",
            "--tail",
            &tail.to_string(),
        ])
        .envs(env_vars.iter().map(|(k, v)| (k.as_str(), v.as_str())))
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

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let stderr_str = String::from_utf8_lossy(&output.stderr);

    // stderr of `docker compose logs` contains only compose-level warnings (e.g. unset vars).
    // With the env vars now passed correctly these should not appear; if they do, still show them.
    let mut entries: Vec<LogEntry> = stdout_str
        .lines()
        .chain(stderr_str.lines())
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            // docker compose logs --timestamps emits lines like:
            //   service-1  | 2026-06-26T22:24:58.123Z message
            // Parse the RFC3339 timestamp when present; fall back to now.
            let timestamp = parse_compose_log_ts(line).unwrap_or_else(Utc::now);
            LogEntry {
                stream: LogStream::Stdout,
                line: line.to_string(),
                timestamp,
            }
        })
        .collect();

    entries.sort_by_key(|e| e.timestamp);

    RpResponse::Logs(entries)
}
