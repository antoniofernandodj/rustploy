use crate::api::AppState;
use chrono::Utc;
use shared::{ActiveDeployInfo, DeployEngineSummary, Response as RpResponse};
use std::collections::HashMap;

pub async fn handle(state: AppState) -> RpResponse {
    let db = &state.db;

    let active_deps = match crate::db::deployments::get_non_terminal(db).await {
        Ok(d) => d,
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    let recent_deps = match crate::db::deployments::list_terminal_last_24h(db, 20).await {
        Ok(d) => d,
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    let (total_24h, successful_24h, failed_24h) =
        match crate::db::deployments::stats_last_24h(db).await {
            Ok(s) => s,
            Err(_) => (0, 0, 0),
        };

    // Cache service+project name lookups.
    let mut svc_cache: HashMap<String, (String, String)> = HashMap::new();
    let mut proj_cache: HashMap<String, String> = HashMap::new();

    let resolve = |svc_cache: &mut HashMap<String, (String, String)>,
                   _proj_cache: &mut HashMap<String, String>,
                   service_id: &str|
     -> (String, String) {
        if let Some(cached) = svc_cache.get(service_id) {
            return cached.clone();
        }
        (service_id.to_string(), String::new())
    };
    let _ = resolve; // avoid unused warning — we resolve inline below

    async fn resolve_names(
        state: &AppState,
        service_id: &str,
        svc_cache: &mut HashMap<String, (String, String)>,
        proj_cache: &mut HashMap<String, String>,
    ) -> (String, String) {
        if let Some(cached) = svc_cache.get(service_id) {
            return cached.clone();
        }
        let (sname, pid) = match crate::db::services::get(&state.db, service_id).await {
            Ok(Some(s)) => (s.spec.name, s.spec.project_id),
            _ => (service_id.to_string(), String::new()),
        };
        let pname = if pid.is_empty() {
            String::new()
        } else if let Some(p) = proj_cache.get(&pid) {
            p.clone()
        } else {
            let name = match crate::db::projects::get(&state.db, &pid).await {
                Ok(Some(p)) => p.name,
                _ => pid.clone(),
            };
            proj_cache.insert(pid.clone(), name.clone());
            name
        };
        svc_cache.insert(service_id.to_string(), (sname.clone(), pname.clone()));
        (sname, pname)
    }

    let now = Utc::now();

    let mut active = Vec::with_capacity(active_deps.len());
    for dep in active_deps {
        let (service_name, project_name) =
            resolve_names(&state, &dep.service_id, &mut svc_cache, &mut proj_cache).await;

        let elapsed_secs = (now - dep.started_at).num_seconds().max(0) as u64;

        let current_state_secs = dep
            .states_log
            .last()
            .map(|t| (now - t.at).num_seconds().max(0) as u64)
            .unwrap_or(elapsed_secs);

        let percent = dep.state.to_percent();

        active.push(ActiveDeployInfo {
            deployment_id: dep.id,
            service_id: dep.service_id,
            service_name,
            project_name,
            state: dep.state,
            percent,
            started_at: dep.started_at,
            elapsed_secs,
            current_state_secs,
        });
    }

    let mut recent = Vec::with_capacity(recent_deps.len());
    for dep in recent_deps {
        let (service_name, project_name) =
            resolve_names(&state, &dep.service_id, &mut svc_cache, &mut proj_cache).await;

        let elapsed_secs = dep
            .finished_at
            .map(|f| (f - dep.started_at).num_seconds().max(0) as u64)
            .unwrap_or_else(|| (now - dep.started_at).num_seconds().max(0) as u64);

        let current_state_secs = (now - dep.started_at).num_seconds().max(0) as u64;

        let percent = dep.state.to_percent();

        recent.push(ActiveDeployInfo {
            deployment_id: dep.id,
            service_id: dep.service_id,
            service_name,
            project_name,
            state: dep.state,
            percent,
            started_at: dep.started_at,
            elapsed_secs,
            current_state_secs,
        });
    }

    RpResponse::DeployEngineStatus(DeployEngineSummary {
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_secs: state.started_at.elapsed().as_secs(),
        active,
        recent,
        total_24h,
        successful_24h,
        failed_24h,
    })
}
