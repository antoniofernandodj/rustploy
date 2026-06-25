use crate::{db::Db, event_bus::EventBus};
use bollard::Docker;
use chrono::Utc;
use shared::{ContainerMetricsPoint, Event, SystemMetricsPoint};
use std::{collections::HashMap, sync::Arc, time::Duration};
use sysinfo::System;
use tokio::time::interval;
use tracing::warn;

pub async fn collect_loop(
    docker: Arc<Docker>,
    db: Arc<Db>,
    bus: Arc<EventBus>,
    interval_secs: u64,
) {
    let mut ticker = interval(Duration::from_secs(interval_secs));
    let mut prev_cpu: HashMap<String, (u64, u64)> = HashMap::new();
    let mut sys = System::new_all();

    loop {
        ticker.tick().await;

        // ── Métricas do SO ───────────────────────────────────────────────────
        bus.publish(Event::SystemMetrics(collect_system_metrics(&mut sys)));

        // ── Métricas por container ───────────────────────────────────────────
        let services = match crate::db::services::get_running(&db).await {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "metrics: failed to get running services");
                continue;
            }
        };

        for svc in services {
            let Some(container_id) = &svc.live_container_id else {
                continue;
            };

            match collect_container_metrics(&docker, container_id, &svc.id, &mut prev_cpu).await {
                Ok(metrics) => bus.publish(Event::ContainerMetrics(metrics)),
                Err(e) => warn!(
                    service = svc.spec.name,
                    error = %e,
                    "metrics collection failed"
                ),
            }
        }
    }
}

fn collect_system_metrics(sys: &mut System) -> SystemMetricsPoint {
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    let cpu_percent = sys.global_cpu_usage() as f64;
    let mem_used = sys.used_memory();
    let mem_total = sys.total_memory();

    // Disco: usa o mount com mais espaço total (normalmente /)
    let (disk_used, disk_total) = {
        use sysinfo::Disks;
        let disks = Disks::new_with_refreshed_list();
        disks
            .iter()
            .max_by_key(|d| d.total_space() as u64)
            .map(|d| (d.total_space() - d.available_space(), d.total_space()))
            .unwrap_or((0u64, 0u64))
    };

    let load = System::load_average();

    SystemMetricsPoint {
        cpu_percent,
        mem_used_bytes: mem_used,
        mem_total_bytes: mem_total,
        disk_used_bytes: disk_used,
        disk_total_bytes: disk_total,
        load_avg_1: load.one,
        load_avg_5: load.five,
        load_avg_15: load.fifteen,
        timestamp: Utc::now(),
    }
}

async fn collect_container_metrics(
    docker: &Docker,
    container_id: &str,
    service_id: &str,
    _prev_cpu: &mut HashMap<String, (u64, u64)>,
) -> anyhow::Result<ContainerMetricsPoint> {
    use bollard::container::StatsOptions;
    use futures::StreamExt;

    let mut stream = docker.stats(
        container_id,
        Some(StatsOptions {
            stream: false,
            one_shot: true,
        }),
    );

    let stats = stream
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("no stats"))??;

    let cpu_delta = stats
        .cpu_stats
        .cpu_usage
        .total_usage
        .saturating_sub(stats.precpu_stats.cpu_usage.total_usage);
    let system_delta = stats
        .cpu_stats
        .system_cpu_usage
        .unwrap_or(0)
        .saturating_sub(stats.precpu_stats.system_cpu_usage.unwrap_or(0));
    let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;
    let cpu_percent = if system_delta > 0 {
        (cpu_delta as f64 / system_delta as f64) * num_cpus * 100.0
    } else {
        0.0
    };

    let mem_used = stats.memory_stats.usage.unwrap_or(0);
    let mem_limit = stats.memory_stats.limit.unwrap_or(0);

    let (net_rx, net_tx) = stats
        .networks
        .as_ref()
        .map(|nets| {
            nets.values().fold((0u64, 0u64), |(rx, tx), net| {
                (rx + net.rx_bytes, tx + net.tx_bytes)
            })
        })
        .unwrap_or((0, 0));

    Ok(ContainerMetricsPoint {
        service_id: service_id.to_string(),
        container_id: container_id.to_string(),
        cpu_percent,
        mem_used_bytes: mem_used,
        mem_limit_bytes: mem_limit,
        net_rx_bytes: net_rx,
        net_tx_bytes: net_tx,
        timestamp: Utc::now(),
    })
}
