//! `Command::Migration*`: assistente de migração para banco compartilhado
//! (ver `crate::migration`).

use crate::{
    api::{AppState, handlers},
    db,
    migration::{self, valid_source_db},
    shared_db::{self, Engine},
};
use shared::{MigrationStartReq, Response as RpResponse};

fn err(code: &str, m: impl Into<String>) -> RpResponse {
    RpResponse::err(code, m.into())
}

pub async fn start(state: AppState, mut req: MigrationStartReq) -> RpResponse {
    let source = match db::services::get(&state.db, &req.source_service_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return err("NotFound", "banco antigo não encontrado"),
        Err(e) => return err("DatabaseError", e.to_string()),
    };
    let Some(engine) = Engine::from_kind(source.spec.db_kind.as_deref()) else {
        return err("Unsupported", "só bancos Postgres, MySQL, MariaDB e MongoDB podem migrar");
    };
    if source.spec.shared.is_some() {
        return err("InvalidRequest", "este serviço já é um servidor compartilhado");
    }
    if !valid_source_db(&req.source_database) {
        return err("InvalidName", "nome do database de origem inválido (use letras, dígitos e _)");
    }
    let dest = match db::managed_database::get(&state.db, &req.dest_database_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return err("NotFound", "database de destino não encontrado"),
        Err(e) => return err("DatabaseError", e.to_string()),
    };
    if dest.project_id != source.spec.project_id {
        return err("InvalidRequest", "o database de destino pertence a outro projeto");
    }
    if req.env_var.trim().is_empty() {
        req.env_var = engine.default_env_var().to_string();
    }
    req.env_var = req.env_var.trim().to_string();
    if !req.env_var.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return err("InvalidName", "nome de env var inválido");
    }
    if let Err(e) = shared_db::validate_name(&dest.name) {
        return err("InvalidName", e.to_string());
    }
    match db::migration::active_for_source(&state.db, &source.id).await {
        Ok(true) => return err("AlreadyRunning", "já existe uma migração em andamento ou concluída para este banco; reverta ou descarte-a antes"),
        Ok(false) => {}
        Err(e) => return err("DatabaseError", e.to_string()),
    }
    let rec = migration::new_record(&req, &source.spec.project_id, &req.env_var);
    if let Err(e) = db::migration::save(&state.db, &rec).await {
        return err("DatabaseError", e.to_string());
    }
    let out = rec.m.clone();
    tokio::spawn(migration::run(state, rec));
    RpResponse::Migration(out)
}

pub async fn get(state: AppState, id: String) -> RpResponse {
    match db::migration::get(&state.db, &id).await {
        Ok(Some(r)) => RpResponse::Migration(r.m),
        Ok(None) => err("NotFound", "migração não encontrada"),
        Err(e) => err("DatabaseError", e.to_string()),
    }
}

pub async fn list(state: AppState, project_id: String) -> RpResponse {
    match db::migration::list(&state.db, &project_id).await {
        Ok(v) => RpResponse::Migrations(v.into_iter().map(|r| r.m).collect()),
        Err(e) => err("DatabaseError", e.to_string()),
    }
}

/// Reverte uma migração concluída: env var e serviços voltam ao banco antigo.
/// O que foi escrito no banco novo depois da migração se perde.
pub async fn rollback(state: AppState, id: String) -> RpResponse {
    let mut rec = match db::migration::get(&state.db, &id).await {
        Ok(Some(r)) => r,
        Ok(None) => return err("NotFound", "migração não encontrada"),
        Err(e) => return err("DatabaseError", e.to_string()),
    };
    if rec.m.status != "Completed" {
        return err("InvalidState", "só uma migração concluída pode ser revertida");
    }
    if let Err(e) = migration::undo(&state, &mut rec, true).await {
        return err("RollbackError", e.to_string());
    }
    rec.m.status = "RolledBack".into();
    if let Err(e) = db::migration::save(&state.db, &rec).await {
        return err("DatabaseError", e.to_string());
    }
    RpResponse::Migration(rec.m)
}

/// Descarta o banco antigo (já parado) — **apaga os dados dele**.
pub async fn discard(state: AppState, id: String) -> RpResponse {
    let mut rec = match db::migration::get(&state.db, &id).await {
        Ok(Some(r)) => r,
        Ok(None) => return err("NotFound", "migração não encontrada"),
        Err(e) => return err("DatabaseError", e.to_string()),
    };
    if rec.m.status != "Completed" {
        return err("InvalidState", "só uma migração concluída permite descartar o banco antigo");
    }
    let sid = rec.m.source_service_id.clone();
    let _ = handlers::service_stop::handle(state.clone(), sid.clone()).await;
    if let RpResponse::Err { message, .. } = handlers::service_delete::handle(state.clone(), sid).await {
        return err("DeleteError", message);
    }
    rec.m.status = "Discarded".into();
    if let Err(e) = db::migration::save(&state.db, &rec).await {
        return err("DatabaseError", e.to_string());
    }
    RpResponse::Migration(rec.m)
}
