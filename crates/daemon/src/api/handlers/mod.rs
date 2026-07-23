/// Converte um erro de banco (sqlx) numa mensagem amigável para o usuário.
/// Reconhece as violações de constraint mais comuns (UNIQUE) e devolve algo
/// legível em vez do texto cru do SQLite (`(code: 2067) UNIQUE constraint
/// failed: project.name`). `subject` é o rótulo do recurso (ex.: "projeto").
pub fn humanize_db_error(err: &(impl std::fmt::Display + ?Sized), subject: &str) -> String {
    let s = err.to_string();
    if s.contains("UNIQUE constraint failed") {
        return format!("Já existe um {subject} com esse nome.");
    }
    if s.contains("FOREIGN KEY constraint failed") {
        return format!("Operação inválida no {subject}: referência inexistente.");
    }
    // Fallback: mensagem genérica, sem vazar o SQL cru.
    format!("Falha ao salvar o {subject}. Tente novamente.")
}

pub mod daemon_status;
pub mod docker_inventory;
pub mod docker_prune;
pub mod docker_remove;
pub mod registry;
pub mod secret_delete;
pub mod secret_list;
pub mod secret_set;
pub mod deploy_abort;
pub mod deploy_delete;
pub mod env_backup;
pub mod deploy_engine_status;
pub mod deploy_history;
pub mod deploy_queue_pause;
pub mod deploy_queue_promote;
pub mod deploy_queue_reorder;
pub mod deploy_rollback;
pub mod deploy_start;
pub mod get_build_logs;
pub mod git_branch_list;
pub mod git_oauth_start;
pub mod git_provider_create;
pub mod git_provider_delete;
pub mod git_provider_list;
pub mod git_repo_list;
pub mod get_daemon_settings;
pub mod get_job_logs;
pub mod get_webhook_url;
pub mod job_create;
pub mod job_delete;
pub mod job_list;
pub mod job_list_all;
pub mod job_run_history;
pub mod job_run_now;
pub mod job_update;
pub mod logs_get;
pub mod manifest_apply;
pub mod manifest_export;
pub mod manifest_export_all;
pub mod manifest_import;
pub mod ping;
pub mod project_create;
pub mod project_delete;
pub mod project_env_set;
pub mod project_list;
pub mod project_update;
pub mod recent_deployments;
pub mod reconcile;
pub mod regenerate_webhook_token;
pub mod service_create;
pub mod service_archive_upload;
pub mod service_delete;
pub mod service_get;
pub mod service_list;
pub mod service_reload;
pub mod service_stop;
pub mod service_update;
pub mod set_daemon_settings;
pub mod wizard;
