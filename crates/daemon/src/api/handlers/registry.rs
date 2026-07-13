//! Sub-aba Docker > Registry: navegação (repositórios/tags) e delete do
//! registry OCI embutido (`crate::registry`). Somente leitura + delete — criar
//! conteúdo só acontece via `docker push` externo, fora deste protocolo.
//! Delete é só de metadados (o CAS em disco só é limpo num GC futuro, fase 4).

use crate::api::AppState;
use crate::db::registry as db_registry;
use shared::{RegistryRepoInfo, RegistryStatusInfo, RegistryTagInfo, Response as RpResponse, RustployConfig};

pub async fn status(state: AppState) -> RpResponse {
    let cfg = &RustployConfig::global().registry;
    match db_registry::summary(&state.db).await {
        Ok(s) => RpResponse::RegistryStatus(RegistryStatusInfo {
            enabled: cfg.enabled,
            port: cfg.port,
            domain: cfg.domain.clone(),
            repo_count: s.repo_count,
            blob_count: s.blob_count,
            storage_bytes: s.storage_bytes,
        }),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}

pub async fn repo_list(state: AppState) -> RpResponse {
    match db_registry::list_repos(&state.db).await {
        Ok(rows) => RpResponse::RegistryRepos(
            rows.into_iter()
                .map(|r| RegistryRepoInfo {
                    name: r.name,
                    tag_count: r.tag_count,
                    size_bytes: r.size_bytes,
                    created_at: r.created_at,
                })
                .collect(),
        ),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}

pub async fn tag_list(state: AppState, repo: String) -> RpResponse {
    let repo_row = match db_registry::get_repo_by_name(&state.db, &repo).await {
        Ok(Some(r)) => r,
        Ok(None) => return RpResponse::err("NotFound", "repositório não encontrado"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };
    match db_registry::list_tags_detailed(&state.db, &repo_row.id).await {
        Ok(rows) => RpResponse::RegistryTags(
            rows.into_iter()
                .map(|t| RegistryTagInfo {
                    tag: t.tag,
                    digest: t.digest,
                    media_type: t.media_type,
                    size_bytes: t.size_bytes,
                    updated_at: t.updated_at,
                })
                .collect(),
        ),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}

pub async fn tag_delete(state: AppState, repo: String, tag: String) -> RpResponse {
    let repo_row = match db_registry::get_repo_by_name(&state.db, &repo).await {
        Ok(Some(r)) => r,
        Ok(None) => return RpResponse::err("NotFound", "repositório não encontrado"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };
    let digest = match db_registry::get_tag_digest(&state.db, &repo_row.id, &tag).await {
        Ok(Some(d)) => d,
        Ok(None) => return RpResponse::err("NotFound", "tag não encontrada"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };
    match db_registry::delete_manifest(&state.db, &repo_row.id, &digest).await {
        Ok(true) => RpResponse::Ok,
        Ok(false) => RpResponse::err("NotFound", "manifest não encontrado"),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}

pub async fn repo_delete(state: AppState, repo: String) -> RpResponse {
    let repo_row = match db_registry::get_repo_by_name(&state.db, &repo).await {
        Ok(Some(r)) => r,
        Ok(None) => return RpResponse::err("NotFound", "repositório não encontrado"),
        Err(e) => return RpResponse::err("DatabaseError", e.to_string()),
    };
    match db_registry::delete_repo(&state.db, &repo_row.id).await {
        Ok(true) => RpResponse::Ok,
        Ok(false) => RpResponse::err("NotFound", "repositório não encontrado"),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
