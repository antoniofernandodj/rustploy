use crate::{db::Db, event_bus::EventBus};
use bollard::{
    Docker,
    container::{LogOutput, LogsOptions},
};
use chrono::Utc;
use futures::StreamExt;
use shared::{Event, protocol::LogStream};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::interval};
use tracing::{debug, warn};

// Timeout para detectar streams travados: se não chegar nenhum dado em 30s
// e o container não estiver mais rodando, encerra o stream.
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(30);

pub async fn stream_loop(docker: Arc<Docker>, db: Arc<Db>, bus: Arc<EventBus>) {
    let active: Arc<Mutex<HashSet<String>>> = Arc::new(Mutex::new(HashSet::new()));
    let mut ticker = interval(Duration::from_secs(5));

    loop {
        ticker.tick().await;

        let services = match crate::db::services::get_running(&db).await {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "logs: failed to get running services");
                continue;
            }
        };

        let new_containers: Vec<(String, String)> = {
            let locked = active.lock().await;
            services
                .into_iter()
                .filter_map(|svc| {
                    let cid = svc.live_container_id?;
                    if locked.contains(&cid) {
                        return None;
                    }
                    Some((cid, svc.id))
                })
                .collect()
        };

        for (container_id, service_id) in new_containers {
            active.lock().await.insert(container_id.clone());

            let docker2 = docker.clone();
            let bus2 = bus.clone();
            let active2 = active.clone();
            let cid = container_id.clone();

            tokio::spawn(async move {
                stream_container(docker2, cid.clone(), service_id, bus2).await;
                active2.lock().await.remove(&cid);
            });
        }
    }
}

async fn stream_container(
    docker: Arc<Docker>,
    container_id: String,
    service_id: String,
    bus: Arc<EventBus>,
) {
    let opts = LogsOptions::<String> {
        follow: true,
        stdout: true,
        stderr: true,
        tail: "100".into(),
        ..Default::default()
    };

    let mut stream = docker.logs(&container_id, Some(opts));

    loop {
        match tokio::time::timeout(STREAM_IDLE_TIMEOUT, stream.next()).await {
            // Linha normal
            Ok(Some(Ok(LogOutput::StdOut { message }))) => {
                publish_log(&bus, &service_id, &container_id, false, &message);
            }
            Ok(Some(Ok(LogOutput::StdErr { message }))) => {
                publish_log(&bus, &service_id, &container_id, true, &message);
            }
            Ok(Some(Ok(_))) => continue,

            // Erro no stream (container parou, etc.)
            Ok(Some(Err(e))) => {
                debug!(error = %e, container = %container_id, "logs: stream encerrado com erro");
                break;
            }

            // Stream encerrou normalmente
            Ok(None) => {
                debug!(container = %container_id, "logs: stream encerrado");
                break;
            }

            // Timeout: nenhum dado em STREAM_IDLE_TIMEOUT segundos.
            // Verifica se o container ainda está rodando.
            Err(_) => {
                let running = docker
                    .inspect_container(
                        &container_id,
                        None::<bollard::container::InspectContainerOptions>,
                    )
                    .await
                    .ok()
                    .and_then(|info| info.state?.running)
                    .unwrap_or(false);

                if !running {
                    debug!(container = %container_id, "logs: container parou, encerrando stream travado");
                    break;
                }
                // Container ainda rodando mas silencioso — continua esperando
            }
        }
    }
}

fn publish_log(
    bus: &EventBus,
    service_id: &str,
    container_id: &str,
    is_stderr: bool,
    bytes: &[u8],
) {
    let text = String::from_utf8_lossy(bytes)
        .trim_end_matches('\n')
        .to_string();
    if text.is_empty() {
        return;
    }
    bus.publish(Event::LogLine {
        service_id: service_id.to_string(),
        container_id: container_id.to_string(),
        stream: if is_stderr {
            LogStream::Stderr
        } else {
            LogStream::Stdout
        },
        line: text,
        timestamp: Utc::now(),
    });
}
