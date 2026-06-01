use crate::{db::Db, event_bus::EventBus};
use anyhow::{anyhow, Result};
use chrono::Utc;
use shared::Event;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{error, info};

pub async fn compose_up(
    content: &str,
    project_name: &str,
    service_id: &str,
    deployment_id: &str,
    bus: &Arc<EventBus>,
    db: &Arc<Db>,
) -> Result<()> {
    info!(project = %project_name, "compose_up: iniciando docker compose up");

    let mut child = Command::new("docker")
        .args([
            "compose", "-p", project_name,
            "-f", "-",
            "up", "-d", "--build", "--remove-orphans",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("falha ao iniciar docker compose: {e}"))?;

    // Escreve o YAML e fecha stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(content.as_bytes()).await
            .map_err(|e| anyhow!("falha ao escrever no stdin: {e}"))?;
        stdin.shutdown().await.ok();
    }

    // Lê stdout e stderr em tempo real, emitindo BuildLog por linha
    let stdout = BufReader::new(child.stdout.take().unwrap());
    let stderr = BufReader::new(child.stderr.take().unwrap());

    let bus_s = bus.clone();
    let db_s = db.clone();
    let sid = service_id.to_string();
    let did = deployment_id.to_string();

    let read_stdout = async move {
        let mut lines = stdout.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() { continue; }
            let ts = Utc::now();
            bus_s.publish(Event::BuildLog {
                deployment_id: did.clone(),
                service_id: sid.clone(),
                line: line.clone(),
                timestamp: ts,
            });
            let _ = crate::db::build_logs::append(&db_s, &did, &line, ts).await;
        }
    };

    let bus_e = bus.clone();
    let db_e = db.clone();
    let sid_e = service_id.to_string();
    let did_e = deployment_id.to_string();

    let read_stderr = async move {
        let mut lines = stderr.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() { continue; }
            let ts = Utc::now();
            bus_e.publish(Event::BuildLog {
                deployment_id: did_e.clone(),
                service_id: sid_e.clone(),
                line: line.clone(),
                timestamp: ts,
            });
            let _ = crate::db::build_logs::append(&db_e, &did_e, &line, ts).await;
        }
    };

    tokio::join!(read_stdout, read_stderr);

    let status = child.wait().await
        .map_err(|e| anyhow!("falha ao aguardar docker compose: {e}"))?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        error!(code, project = %project_name, "compose_up: falhou");
        return Err(anyhow!("docker compose up terminou com código {code}"));
    }

    info!(project = %project_name, "compose_up: concluído");
    Ok(())
}

pub async fn compose_down(content: &str, project_name: &str) -> Result<()> {
    info!(project = %project_name, "compose_down: iniciando");

    let mut child = Command::new("docker")
        .args([
            "compose", "-p", project_name,
            "-f", "-",
            "down", "--remove-orphans",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("falha ao iniciar docker compose down: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(content.as_bytes()).await;
        stdin.shutdown().await.ok();
    }

    let output = child.wait_with_output().await
        .map_err(|e| anyhow!("falha ao aguardar compose down: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("docker compose down falhou: {}", stderr.trim()));
    }

    info!(project = %project_name, "compose_down: concluído");
    Ok(())
}
