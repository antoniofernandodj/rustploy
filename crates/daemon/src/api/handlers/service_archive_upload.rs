use std::{
    fs,
    io::{self, Cursor},
    path::{Component, Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use bytes::Bytes;
use shared::{ArchiveSource, Response as RpResponse, ServiceSource};
use tracing::{info, warn};
use ulid::Ulid;

use crate::api::AppState;

pub const MAX_ZIP_BYTES: usize = 100 * 1024 * 1024;

pub async fn handle(
    state: AppState,
    service_id: String,
    bytes: Bytes,
    original_filename: Option<String>,
) -> RpResponse {
    if bytes.is_empty() {
        return RpResponse::err("InvalidArchive", "zip vazio");
    }
    if bytes.len() > MAX_ZIP_BYTES {
        return RpResponse::err("ArchiveTooLarge", "zip excede 100 MiB");
    }

    let svc = match crate::db::services::get(&state.db, &service_id).await {
        Ok(Some(s)) => s,
        Ok(None) => return RpResponse::err("NotFound", "service not found"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };

    if matches!(svc.spec.source, ServiceSource::Compose(_)) {
        return RpResponse::err(
            "InvalidServiceType",
            "upload de zip só é permitido para services do tipo application",
        );
    }

    let archive_id = format!("arc_{}", Ulid::new());
    let extract_dir = archive_extract_dir(&state.db_path, &service_id, &archive_id);
    if let Err(e) = extract_zip(&bytes, &extract_dir) {
        let _ = fs::remove_dir_all(&extract_dir);
        warn!(service_id = %service_id, archive_id = %archive_id, error = %e, "service_archive_upload: zip inválido");
        return RpResponse::err("InvalidArchive", e.to_string());
    }

    if !extract_dir.join("Dockerfile").is_file() {
        let _ = fs::remove_dir_all(&extract_dir);
        return RpResponse::err(
            "MissingDockerfile",
            "o zip precisa conter um Dockerfile na raiz",
        );
    }

    let mut spec = svc.spec.clone();
    spec.source = ServiceSource::Archive(ArchiveSource {
        archive_id: archive_id.clone(),
        original_filename,
        dockerfile_path: "Dockerfile".into(),
        build_context: ".".into(),
    });

    match crate::db::services::update_spec(&state.db, &service_id, spec).await {
        Ok(Some(updated)) => {
            info!(service_id = %service_id, archive_id = %archive_id, "service_archive_upload: archive associado ao serviço");
            RpResponse::Service(updated)
        }
        Ok(None) => RpResponse::err("NotFound", "service not found"),
        Err(e) => {
            let _ = fs::remove_dir_all(&extract_dir);
            RpResponse::err("DatabaseError", e.to_string())
        }
    }
}

pub fn archive_extract_dir(db_path: &Path, service_id: &str, archive_id: &str) -> PathBuf {
    db_path
        .join("uploads")
        .join("services")
        .join(service_id)
        .join(archive_id)
}

pub fn extract_zip(bytes: &[u8], dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).with_context(|| format!("criando {}", dest.display()))?;
    let reader = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(reader).context("arquivo não é um zip válido")?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let Some(rel) = safe_zip_path(file.name()) else {
            return Err(anyhow!("zip contém caminho inseguro: {}", file.name()));
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        let out = dest.join(rel);
        if file.is_dir() {
            fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = fs::File::create(&out)?;
        io::copy(&mut file, &mut output)?;
    }

    Ok(())
}

fn safe_zip_path(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(out)
}
