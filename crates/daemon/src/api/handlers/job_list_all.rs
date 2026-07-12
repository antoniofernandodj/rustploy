use crate::api::AppState;
use shared::{JobSummary, Response as RpResponse};
use std::collections::HashMap;

pub async fn handle(state: AppState) -> RpResponse {
    let jobs = match crate::db::job::list_all(&state.db).await {
        Ok(j) => j,
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    // Resolve nomes de projeto/serviço com cache — evita N+1 pra cada job repetido.
    let mut proj_cache: HashMap<String, String> = HashMap::new();
    let mut svc_cache: HashMap<String, String> = HashMap::new();
    let mut summaries = Vec::with_capacity(jobs.len());

    for job in jobs {
        let project_name = if let Some(n) = proj_cache.get(&job.project_id) {
            n.clone()
        } else {
            let name = match crate::db::projects::get(&state.db, &job.project_id).await {
                Ok(Some(p)) => p.name,
                _ => job.project_id.clone(),
            };
            proj_cache.insert(job.project_id.clone(), name.clone());
            name
        };

        let trigger_service_name = if let Some(n) = svc_cache.get(&job.trigger_service_id) {
            n.clone()
        } else {
            let name = match crate::db::services::get(&state.db, &job.trigger_service_id).await {
                Ok(Some(s)) => s.spec.name,
                _ => job.trigger_service_id.clone(),
            };
            svc_cache.insert(job.trigger_service_id.clone(), name.clone());
            name
        };

        let last_run = match crate::db::job_run::latest_for_job(&state.db, &job.id).await {
            Ok(r) => r,
            Err(_) => None,
        };

        summaries.push(JobSummary {
            job,
            project_name,
            trigger_service_name,
            last_run,
        });
    }

    RpResponse::JobSummaries(summaries)
}
