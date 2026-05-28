use crate::{db::Db, event_bus::EventBus};
use bollard::{container::{LogOutput, LogsOptions}, Docker};
use chrono::Utc;
use futures::StreamExt;
use shared::{protocol::LogStream, Event};
use std::{collections::HashSet, sync::Arc, time::Duration};
use tokio::{sync::Mutex, time::interval};
use tracing::{debug, warn};

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

    while let Some(item) = stream.next().await {
        let (is_stderr, bytes) = match item {
            Ok(LogOutput::StdOut { message }) => (false, message),
            Ok(LogOutput::StdErr { message }) => (true, message),
            Ok(_) => continue,
            Err(e) => {
                debug!(error = %e, container = %container_id, "logs: stream ended");
                break;
            }
        };

        let text = String::from_utf8_lossy(&bytes)
            .trim_end_matches('\n')
            .to_string();
        if text.is_empty() {
            continue;
        }

        bus.publish(Event::LogLine {
            service_id: service_id.clone(),
            container_id: container_id.clone(),
            stream: if is_stderr {
                LogStream::Stderr
            } else {
                LogStream::Stdout
            },
            line: text,
            timestamp: Utc::now(),
        });
    }
}
