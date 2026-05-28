use crate::api::AppState;
use shared::{DeploymentSummary, Response as RpResponse};
use std::collections::HashMap;

pub async fn handle(state: AppState, limit: usize) -> RpResponse {
    let deps = match crate::db::deployments::list_recent(&state.db, limit).await {
        Ok(d) => d,
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    // Reconcile before building summaries — corrects stale Live entries in DB.
    let deps = super::reconcile::fix_stale_live(&state, deps).await;

    // Resolve service and project names with a small cache to avoid N+1 queries.
    let mut svc_cache: HashMap<String, (String, String)> = HashMap::new(); // id → (name, project_id)
    let mut proj_cache: HashMap<String, String> = HashMap::new(); // id → name
    let mut summaries = Vec::with_capacity(deps.len());

    for dep in deps {
        let (service_name, project_name) = if let Some(cached) = svc_cache.get(&dep.service_id) {
            cached.clone()
        } else {
            let (sname, pid) = match crate::db::services::get(&state.db, &dep.service_id).await {
                Ok(Some(s)) => (s.spec.name.clone(), s.spec.project_id.clone()),
                _ => (dep.service_id.clone(), String::new()),
            };
            let pname = if pid.is_empty() {
                String::new()
            } else if let Some(p) = proj_cache.get(&pid) {
                p.clone()
            } else {
                let name = match crate::db::projects::get(&state.db, &pid).await {
                    Ok(Some(p)) => p.name.clone(),
                    _ => pid.clone(),
                };
                proj_cache.insert(pid.clone(), name.clone());
                name
            };
            svc_cache.insert(dep.service_id.clone(), (sname.clone(), pname.clone()));
            (sname, pname)
        };

        summaries.push(DeploymentSummary { deployment: dep, service_name, project_name });
    }

    RpResponse::DeploymentSummaries(summaries)
}
