use crate::db::Db;
use chrono::{Datelike, Utc};
use serde::{Deserialize, Serialize};
use shared::{EnvVar, EnvVarValue};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::time::interval;
use tracing::{info, warn};

/// Conteúdo de um snapshot: todos os projectos e serviços com as suas env vars.
#[derive(Debug, Serialize, Deserialize)]
pub struct EnvSnapshot {
    pub created_at: chrono::DateTime<Utc>,
    pub projects: Vec<ProjectEnvEntry>,
    pub services: Vec<ServiceEnvEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectEnvEntry {
    pub id: String,
    pub name: String,
    pub env_vars: Vec<EnvVar>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceEnvEntry {
    pub id: String,
    pub name: String,
    pub project_id: String,
    pub env_vars: Vec<EnvVar>,
}

// ── Background loop ───────────────────────────────────────────────────────────

pub async fn backup_loop(db: Arc<Db>, backup_dir: PathBuf, interval_secs: u64) {
    let mut ticker = interval(Duration::from_secs(interval_secs));
    let mut last_cleanup_month = Utc::now().month();

    loop {
        ticker.tick().await;

        if let Err(e) = write_snapshot(&db, &backup_dir).await {
            warn!(error = %e, "env_backup: falha ao gravar snapshot");
        }

        // Limpeza mensal: no início de cada mês apaga snapshots do mês anterior
        let now = Utc::now();
        if now.month() != last_cleanup_month {
            last_cleanup_month = now.month();
            if let Err(e) = cleanup_old(&backup_dir).await {
                warn!(error = %e, "env_backup: falha na limpeza mensal");
            }
        }
    }
}

// ── Gravar snapshot ───────────────────────────────────────────────────────────

async fn write_snapshot(db: &Db, backup_dir: &PathBuf) -> anyhow::Result<()> {
    tokio::fs::create_dir_all(backup_dir).await?;

    let snapshot = collect_snapshot(db).await?;
    let json = serde_json::to_string_pretty(&snapshot)?;

    // Nome: env_backup_2026-06-26T00-31-29Z.json
    let filename = format!(
        "env_backup_{}.json",
        snapshot.created_at.format("%Y-%m-%dT%H-%M-%SZ")
    );
    let path = backup_dir.join(&filename);
    tokio::fs::write(&path, json.as_bytes()).await?;

    info!(file = %filename, "env_backup: snapshot gravado");
    Ok(())
}

pub async fn collect_snapshot(db: &Db) -> anyhow::Result<EnvSnapshot> {
    let projects = crate::db::projects::list(db).await?;
    let mut project_entries = Vec::new();
    let mut service_entries = Vec::new();

    for proj in &projects {
        project_entries.push(ProjectEnvEntry {
            id: proj.id.clone(),
            name: proj.name.clone(),
            env_vars: proj.env_vars.clone(),
        });

        let services = crate::db::services::list(db, &proj.id).await?;
        for svc in services {
            if !svc.spec.env_vars.is_empty() {
                service_entries.push(ServiceEnvEntry {
                    id: svc.id.clone(),
                    name: svc.spec.name.clone(),
                    project_id: proj.id.clone(),
                    env_vars: svc.spec.env_vars.clone(),
                });
            }
        }
    }

    Ok(EnvSnapshot {
        created_at: Utc::now(),
        projects: project_entries,
        services: service_entries,
    })
}

// ── Listar snapshots ──────────────────────────────────────────────────────────

pub async fn list_snapshots(backup_dir: &PathBuf) -> anyhow::Result<Vec<String>> {
    let mut names = Vec::new();
    let mut rd = match tokio::fs::read_dir(backup_dir).await {
        Ok(rd) => rd,
        Err(_) => return Ok(names), // directório ainda não existe
    };
    while let Ok(Some(entry)) = rd.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("env_backup_") && name.ends_with(".json") {
            names.push(name);
        }
    }
    names.sort_by(|a, b| b.cmp(a)); // mais recente primeiro
    Ok(names)
}

// ── Restaurar snapshot ────────────────────────────────────────────────────────

pub async fn restore_snapshot(db: &Db, backup_dir: &PathBuf, snapshot: &str) -> anyhow::Result<usize> {
    // Valida que o nome não contém traversal
    anyhow::ensure!(
        !snapshot.contains('/') && !snapshot.contains('\\'),
        "nome de snapshot inválido"
    );

    let path = backup_dir.join(snapshot);
    let bytes = tokio::fs::read(&path).await?;
    let snap: EnvSnapshot = serde_json::from_slice(&bytes)?;

    let mut restored = 0usize;

    for entry in &snap.projects {
        // Só actualiza se o projecto ainda existe com o mesmo ID
        if crate::db::projects::get(db, &entry.id).await?.is_some() {
            crate::db::projects::update_env_vars(db, &entry.id, entry.env_vars.clone()).await?;
            restored += 1;
        }
    }

    for entry in &snap.services {
        if let Ok(Some(svc)) = crate::db::services::get(db, &entry.id).await {
            let mut spec = svc.spec.clone();
            spec.env_vars = entry.env_vars.clone();
            crate::db::services::update_spec(db, &entry.id, spec).await?;
            restored += 1;
        }
    }

    info!(snapshot, restored, "env_backup: snapshot restaurado");
    Ok(restored)
}

// ── Limpeza mensal: apaga ficheiros com mais de 31 dias ──────────────────────

async fn cleanup_old(backup_dir: &PathBuf) -> anyhow::Result<()> {
    let cutoff = Utc::now() - chrono::Duration::days(31);
    let cutoff_str = cutoff.format("%Y-%m-%dT%H-%M-%SZ").to_string();

    let mut rd = match tokio::fs::read_dir(backup_dir).await {
        Ok(rd) => rd,
        Err(_) => return Ok(()),
    };

    let mut removed = 0u32;
    while let Ok(Some(entry)) = rd.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("env_backup_") || !name.ends_with(".json") {
            continue;
        }
        // O timestamp está embutido no nome: env_backup_YYYY-MM-DDTHH-MM-SSZ.json
        let ts_part = name
            .strip_prefix("env_backup_")
            .and_then(|s| s.strip_suffix(".json"))
            .unwrap_or("");
        if ts_part < cutoff_str.as_str() {
            if let Err(e) = tokio::fs::remove_file(entry.path()).await {
                warn!(file = %name, error = %e, "env_backup: falha ao remover ficheiro antigo");
            } else {
                removed += 1;
            }
        }
    }

    if removed > 0 {
        info!(removed, "env_backup: limpeza mensal concluída");
    }
    Ok(())
}

// Silenciar warnings de imports não usados nos tipos públicos
const _: () = {
    let _ = std::mem::size_of::<EnvVarValue>();
};
