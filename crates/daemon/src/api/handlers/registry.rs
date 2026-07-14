//! Sub-aba Docker > Registry: navegação (repositórios/tags), delete e GC do
//! registry OCI embutido (`crate::registry`). Criar conteúdo só acontece via
//! `docker push` externo, fora deste protocolo. Delete é só de metadados; o
//! disco é liberado pelo GC (`RegistryGc` — botão na UI + job diário).

use crate::api::AppState;
use crate::db::registry as db_registry;
use crate::db::registry_tokens as db_tokens;
use sha2::{Digest, Sha256};
use shared::{
    RegistryRepoInfo, RegistryStatusInfo, RegistryTagInfo, RegistryTokenInfo, Response as RpResponse,
    RustployConfig,
};
use std::io::Read;

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

pub async fn gc(state: AppState) -> RpResponse {
    let Some(storage) = state.registry_storage.clone() else {
        return RpResponse::err(
            "RegistryDisabled",
            "registry desabilitado na config ([registry] enabled = true)",
        );
    };
    match crate::registry::gc::run(&state.db, &storage).await {
        Ok(r) => RpResponse::RegistryGcResult {
            blobs_removed: r.blobs_removed,
            bytes_freed: r.bytes_freed,
        },
        Err(e) => RpResponse::err("GcError", e.to_string()),
    }
}

/// Gera 32 bytes aleatórios via `/dev/urandom` (mesmo padrão de
/// `secrets.rs::generate_key`/`db::webhook_tokens::generate_token`).
fn generate_secret() -> String {
    let mut bytes = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .unwrap_or_default();
    hex::encode(bytes)
}

pub async fn token_create(state: AppState, name: String, scope: String) -> RpResponse {
    if name == db_tokens::RP_INTERNAL {
        return RpResponse::err("ReservedName", "\"rp-internal\" é um nome reservado do sistema");
    }
    if scope != "pull" && scope != "push" {
        return RpResponse::err("InvalidScope", "scope deve ser \"pull\" ou \"push\"");
    }
    let secret = generate_secret();
    let hash = hex::encode(Sha256::digest(secret.as_bytes()));
    match db_tokens::create(&state.db, &name, &hash, &scope).await {
        Ok(()) => RpResponse::RegistryTokenCreated { name, secret },
        Err(e) => RpResponse::err(
            "Conflict",
            crate::api::handlers::humanize_db_error(&e, "token"),
        ),
    }
}

pub async fn token_list(state: AppState) -> RpResponse {
    match db_tokens::list(&state.db).await {
        Ok(rows) => RpResponse::RegistryTokens(
            rows.into_iter()
                .map(|t| RegistryTokenInfo {
                    name: t.name,
                    scope: t.scope,
                    created_at: t.created_at,
                    last_used_at: t.last_used_at,
                })
                .collect(),
        ),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}

pub async fn token_revoke(state: AppState, name: String) -> RpResponse {
    match db_tokens::revoke(&state.db, &name).await {
        Ok(true) => RpResponse::Ok,
        Ok(false) => RpResponse::err("NotFound", "token não encontrado"),
        Err(e) => RpResponse::err("DatabaseError", e.to_string()),
    }
}
