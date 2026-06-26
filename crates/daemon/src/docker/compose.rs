use crate::{db::Db, event_bus::EventBus};
use anyhow::{Result, anyhow};
use chrono::Utc;
use shared::Event;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{error, info};

/// Local alias used inside the compose file to refer to the project network.
const PROJECT_NET_ALIAS: &str = "rp_project_net";

/// Unique Docker Compose project name for a rustploy service.
///
/// Incorporates the first 8 chars of the service ULID so two services with the
/// same user-facing name but in different projects never share a compose project
/// (and therefore never share container names).
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

/// Injects the project's (external) Docker network into a compose YAML so that
/// every service in the stack joins it automatically. The user writes a normal
/// compose file; we rewrite it transparently before `docker compose up`.
///
/// - Adds a top-level `networks: { rp_project_net: { external: true, name: <network_name> } }`
///   entry (idempotent — skipped if already present).
/// - Appends `rp_project_net` to every service's `networks:` list (or as a
///   `rp_project_net: {}` key if the service uses the mapping form).
pub fn inject_project_network(content: &str, network_name: &str) -> Result<String> {
    use serde_yaml::Value;

    let mut doc: Value =
        serde_yaml::from_str(content).map_err(|e| anyhow!("compose YAML inválido: {e}"))?;

    // An empty document parses to Null; treat it as an empty mapping.
    if doc.is_null() {
        doc = Value::Mapping(serde_yaml::Mapping::new());
    }

    let root = doc
        .as_mapping_mut()
        .ok_or_else(|| anyhow!("compose YAML não é um mapping no nível raiz"))?;

    let alias_key = Value::String(PROJECT_NET_ALIAS.to_string());

    // --- Top-level networks block -------------------------------------------
    let networks_key = Value::String("networks".to_string());
    if !root.contains_key(&networks_key) {
        root.insert(
            networks_key.clone(),
            Value::Mapping(serde_yaml::Mapping::new()),
        );
    }
    let networks = root
        .get_mut(&networks_key)
        .and_then(Value::as_mapping_mut)
        .ok_or_else(|| anyhow!("`networks:` no compose não é um mapping"))?;

    // Idempotency: skip if our alias already exists, or if some entry already
    // points at this real network name.
    let already_present = networks.contains_key(&alias_key)
        || networks.values().any(|v| {
            v.as_mapping()
                .and_then(|m| m.get(Value::String("name".to_string())))
                .and_then(Value::as_str)
                == Some(network_name)
        });

    if !already_present {
        let mut entry = serde_yaml::Mapping::new();
        entry.insert(Value::String("external".to_string()), Value::Bool(true));
        entry.insert(
            Value::String("name".to_string()),
            Value::String(network_name.to_string()),
        );
        networks.insert(alias_key.clone(), Value::Mapping(entry));
    }

    // --- Per-service networks ------------------------------------------------
    let services_key = Value::String("services".to_string());
    if let Some(services_val) = root.get_mut(&services_key) {
        let services = services_val
            .as_mapping_mut()
            .ok_or_else(|| anyhow!("`services:
` no compose não é um mapping"))?;

        let svc_net_key = Value::String("networks".to_string());
        for (_, svc) in services.iter_mut() {
            let Some(svc_map) = svc.as_mapping_mut() else {
                // Skip null/empty service definitions; nothing to attach to.
                continue;
            };

            match svc_map.get_mut(&svc_net_key) {
                Some(Value::Sequence(seq)) => {
                    let already = seq.iter().any(|v| v.as_str() == Some(PROJECT_NET_ALIAS));
                    if !already {
                        seq.push(Value::String(PROJECT_NET_ALIAS.to_string()));
                    }
                }
                Some(Value::Mapping(map)) => {
                    if !map.contains_key(&alias_key) {
                        map.insert(
                            alias_key.clone(),
                            Value::Mapping(serde_yaml::Mapping::new()),
                        );
                    }
                }
                Some(other) if other.is_null() => {
                    *other = Value::Sequence(vec![Value::String(PROJECT_NET_ALIAS.to_string())]);
                }
                Some(_) => {
                    // Unexpected scalar form for `networks:`; leave untouched.
                }
                None => {
                    svc_map.insert(
                        svc_net_key.clone(),
                        Value::Sequence(vec![Value::String(PROJECT_NET_ALIAS.to_string())]),
                    );
                }
            }
        }
    }

    serde_yaml::to_string(&doc).map_err(|e| anyhow!("falha ao serializar compose YAML: {e}"))
}

pub async fn compose_up(
    content: &str,
    project_name: &str,
    service_id: &str,
    deployment_id: &str,
    network_name: &str,
    bus: &Arc<EventBus>,
    db: &Arc<Db>,
    env_vars: &[(String, String)],
) -> Result<()> {
    info!(project = %project_name, "compose_up: iniciando docker compose up");

    let content = inject_project_network(content, network_name)?;
    let content = content.as_str();

    let mut child = Command::new("docker")
        .args([
            "compose",
            "-p",
            project_name,
            "-f",
            "-",
            "up",
            "-d",
            "--build",
            "--remove-orphans",
        ])
        .envs(env_vars.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("falha ao iniciar docker compose: {e}"))?;

    // Escreve o YAML e fecha stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(content.as_bytes())
            .await
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
            if line.trim().is_empty() {
                continue;
            }
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
            if line.trim().is_empty() {
                continue;
            }
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

    let status = child
        .wait()
        .await
        .map_err(|e| anyhow!("falha ao aguardar docker compose: {e}"))?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        error!(code, project = %project_name, "compose_up: falhou");
        return Err(anyhow!("docker compose up terminou com código {code}"));
    }

    info!(project = %project_name, "compose_up: concluído");
    Ok(())
}

pub async fn compose_down(content: &str, project_name: &str, _network_name: &str, env_vars: &[(String, String)]) -> Result<()> {
    info!(project = %project_name, "compose_down: iniciando");

    let mut child = Command::new("docker")
        .args([
            "compose",
            "-p",
            project_name,
            "-f",
            "-",
            "down",
            "--remove-orphans",
        ])
        .envs(env_vars.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!("falha ao iniciar docker compose down: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(content.as_bytes()).await;
        stdin.shutdown().await.ok();
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| anyhow!("falha ao aguardar compose down: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("docker compose down falhou: {}", stderr.trim()));
    }

    info!(project = %project_name, "compose_down: concluído");
    Ok(())
}
