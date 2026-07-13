//! Rotas HTTP da OCI Distribution API v2 — dispatch manual (hyper cru não tem
//! router): `match`/`rsplit_once` sobre método + segmentos de path conhecidos.
//! Loopback only, sem autenticação — Fase 1 (`docs/plano-registry-embutido.md`).

use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Frame, Incoming};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::io::AsyncReadExt;
use tracing::{error, info, warn};

use super::error::{RegistryBody, RegistryError};
use super::name;
use super::storage::{RegistryStorage, StorageError};
use crate::db::registry as registry_db;
use crate::db::Db;

/// Bind (loopback only) + prepara o storage; retorna em caso de falha (log +
/// listener desabilitado, daemon continua) — mesmo padrão de
/// `api::webhook_server::run`/`api::http_api::run`.
pub async fn run(db: Arc<Db>, storage_dir: PathBuf, port: u16) {
    let addr: std::net::SocketAddr = ([127, 0, 0, 1], port).into();
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!(port, error = %e, "registry: falha ao bind, listener desabilitado");
            return;
        }
    };
    let storage = match RegistryStorage::new(storage_dir) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            error!(error = %e, "registry: falha ao preparar storage_dir");
            return;
        }
    };
    info!(port, "registry: escutando (loopback, sem auth — Fase 1)");
    serve(listener, db, storage).await;
}

/// Loop de accept, separado de `run` para os testes de integração poderem
/// bindar em `127.0.0.1:0` e descobrir a porta real via `local_addr()`.
pub async fn serve(listener: tokio::net::TcpListener, db: Arc<Db>, storage: Arc<RegistryStorage>) {
    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "registry: accept error");
                continue;
            }
        };
        let (db, storage) = (db.clone(), storage.clone());
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let svc = service_fn(move |req| {
                let (db, storage) = (db.clone(), storage.clone());
                async move { Ok::<_, Infallible>(route(req, db, storage).await) }
            });
            if let Err(e) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, svc)
                .await
            {
                tracing::debug!(peer = %peer, error = %e, "registry: conexão encerrada");
            }
        });
    }
}

async fn route(req: Request<Incoming>, db: Arc<Db>, storage: Arc<RegistryStorage>) -> Response<RegistryBody> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();

    let result = dispatch(&path, &query, method, req, &db, &storage).await;

    match result {
        Ok(mut resp) => {
            resp.headers_mut().insert(
                "Docker-Distribution-API-Version",
                hyper::header::HeaderValue::from_static("registry/2.0"),
            );
            resp
        }
        Err(e) => e.into_response(),
    }
}

async fn dispatch(
    path: &str,
    query: &str,
    method: Method,
    req: Request<Incoming>,
    db: &Db,
    storage: &RegistryStorage,
) -> Result<Response<RegistryBody>, RegistryError> {
    let rest = match path.strip_prefix("/v2/") {
        Some(r) => r,
        None if path == "/v2" => "",
        None => return Err(RegistryError::NameInvalid(path.to_string())),
    };

    if rest.is_empty() {
        return ping();
    }
    if rest == "_catalog" {
        return catalog(db).await;
    }
    if let Some(repo_name) = rest.strip_suffix("/tags/list") {
        return tags_list(db, repo_name).await;
    }
    if let Some((repo_name, uuid)) = rest.rsplit_once("/blobs/uploads/") {
        return blob_upload(method, req, db, storage, repo_name, uuid, query).await;
    }
    if let Some((repo_name, digest)) = rest.rsplit_once("/blobs/") {
        return blob(method, storage, repo_name, digest).await;
    }
    if let Some((repo_name, reference)) = rest.rsplit_once("/manifests/") {
        return manifest(method, req, db, storage, repo_name, reference).await;
    }
    Err(RegistryError::NameInvalid(rest.to_string()))
}

fn ping() -> Result<Response<RegistryBody>, RegistryError> {
    json_response(StatusCode::OK, &serde_json::json!({}))
}

async fn catalog(db: &Db) -> Result<Response<RegistryBody>, RegistryError> {
    let names = registry_db::list_repo_names(db).await?;
    json_response(StatusCode::OK, &serde_json::json!({ "repositories": names }))
}

async fn tags_list(db: &Db, repo_name: &str) -> Result<Response<RegistryBody>, RegistryError> {
    if !name::is_valid_name(repo_name) {
        return Err(RegistryError::NameInvalid(repo_name.to_string()));
    }
    let repo = registry_db::get_repo_by_name(db, repo_name)
        .await?
        .ok_or_else(|| RegistryError::NameUnknown(repo_name.to_string()))?;
    let tags = registry_db::list_tags(db, &repo.id).await?;
    json_response(
        StatusCode::OK,
        &serde_json::json!({ "name": repo_name, "tags": tags }),
    )
}

// ── Blobs ────────────────────────────────────────────────────────────────

async fn blob(
    method: Method,
    storage: &RegistryStorage,
    repo_name: &str,
    digest: &str,
) -> Result<Response<RegistryBody>, RegistryError> {
    if !name::is_valid_name(repo_name) {
        return Err(RegistryError::NameInvalid(repo_name.to_string()));
    }
    let digest_hex = name::parse_digest(digest)
        .ok_or_else(|| RegistryError::DigestInvalid(digest.to_string()))?;

    if !storage.blob_exists(digest_hex).await {
        return Err(RegistryError::BlobUnknown(digest.to_string()));
    }
    let size = storage.blob_len(digest_hex).await?;

    if method == Method::HEAD {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Docker-Content-Digest", format!("sha256:{digest_hex}"))
            .header("Content-Length", size.to_string())
            .body(empty_body())
            .expect("valid HEAD blob response"));
    }
    if method != Method::GET {
        return Err(RegistryError::BlobUploadInvalid(format!(
            "método {method} inválido para blob"
        )));
    }

    let file = storage.open_blob(digest_hex).await?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Docker-Content-Digest", format!("sha256:{digest_hex}"))
        .header("Content-Length", size.to_string())
        .body(blob_body(file))
        .expect("valid GET blob response"))
}

/// Stream do arquivo em frames de 64KB — nunca bufferiza um blob inteiro em
/// memória (camadas de imagem podem ter GBs).
fn blob_body(file: tokio::fs::File) -> RegistryBody {
    let stream = futures::stream::unfold(file, |mut f| async move {
        let mut buf = vec![0u8; 64 * 1024];
        match f.read(&mut buf).await {
            Ok(0) => None,
            Ok(n) => {
                buf.truncate(n);
                Some((Ok::<_, Infallible>(Frame::data(Bytes::from(buf))), f))
            }
            Err(e) => {
                warn!(error = %e, "registry: erro lendo blob, encerrando stream");
                None
            }
        }
    });
    StreamBody::new(stream).boxed()
}

async fn blob_upload(
    method: Method,
    req: Request<Incoming>,
    db: &Db,
    storage: &RegistryStorage,
    repo_name: &str,
    uuid: &str,
    query: &str,
) -> Result<Response<RegistryBody>, RegistryError> {
    if !name::is_valid_name(repo_name) {
        return Err(RegistryError::NameInvalid(repo_name.to_string()));
    }
    match method {
        Method::POST if uuid.is_empty() => start_or_monolithic_upload(req, db, storage, repo_name, query).await,
        Method::PATCH => patch_upload(req, storage, repo_name, uuid).await,
        Method::PUT => put_upload(req, storage, db, repo_name, uuid, query).await,
        Method::DELETE => delete_upload(storage, uuid).await,
        _ => Err(RegistryError::BlobUploadInvalid(format!(
            "método {method} inválido para upload de blob"
        ))),
    }
}

async fn start_or_monolithic_upload(
    req: Request<Incoming>,
    db: &Db,
    storage: &RegistryStorage,
    repo_name: &str,
    query: &str,
) -> Result<Response<RegistryBody>, RegistryError> {
    registry_db::get_or_create_repo(db, repo_name).await?;

    let digest_param = query_param(query, "digest");
    let body = collect_body(req).await?;

    // Upload monolítico: `POST .../blobs/uploads/?digest=...` com o blob
    // inteiro no corpo, sem passar por PATCH/PUT.
    if let (Some(digest_full), false) = (&digest_param, body.is_empty()) {
        let expected_hex = name::parse_digest(digest_full)
            .ok_or_else(|| RegistryError::DigestInvalid(digest_full.clone()))?;
        let info = storage.write_blob_direct(&body).await.map_err(storage_err)?;
        if info.digest != expected_hex {
            return Err(RegistryError::DigestInvalid(digest_full.clone()));
        }
        registry_db::insert_blob(db, &info.digest, info.size as i64).await?;
        return Ok(created_blob_response(repo_name, &info.digest));
    }

    // Upload em chunks (fluxo normal): cria a sessão; `mount=`/`from=` são
    // aceitos e ignorados (fallback espec-compliant para upload normal).
    let upload_id = storage.start_upload().await.map_err(RegistryError::from)?;
    let mut written = 0u64;
    if !body.is_empty() {
        written = storage
            .write_chunk(&upload_id, &body)
            .await
            .map_err(storage_err)?;
    }
    Ok(upload_accepted_response(repo_name, &upload_id, written))
}

async fn patch_upload(
    req: Request<Incoming>,
    storage: &RegistryStorage,
    repo_name: &str,
    uuid: &str,
) -> Result<Response<RegistryBody>, RegistryError> {
    let mut body = req.into_body();
    let mut written = 0u64;
    loop {
        match body.frame().await {
            Some(Ok(frame)) => {
                if let Some(data) = frame.data_ref() {
                    written = storage.write_chunk(uuid, data).await.map_err(storage_err)?;
                }
            }
            Some(Err(e)) => {
                return Err(RegistryError::Internal(anyhow::anyhow!(
                    "erro lendo corpo do PATCH: {e}"
                )))
            }
            None => break,
        }
    }
    Ok(upload_accepted_response(repo_name, uuid, written))
}

async fn put_upload(
    req: Request<Incoming>,
    storage: &RegistryStorage,
    db: &Db,
    repo_name: &str,
    uuid: &str,
    query: &str,
) -> Result<Response<RegistryBody>, RegistryError> {
    let digest_param =
        query_param(query, "digest").ok_or_else(|| RegistryError::DigestInvalid("digest ausente".to_string()))?;
    let digest_hex = name::parse_digest(&digest_param)
        .ok_or_else(|| RegistryError::DigestInvalid(digest_param.clone()))?
        .to_string();

    // O PUT pode carregar o último pedaço do blob (alguns clientes não fazem
    // PATCH nenhum e mandam tudo aqui).
    let mut body = req.into_body();
    loop {
        match body.frame().await {
            Some(Ok(frame)) => {
                if let Some(data) = frame.data_ref() {
                    storage.write_chunk(uuid, data).await.map_err(storage_err)?;
                }
            }
            Some(Err(e)) => {
                return Err(RegistryError::Internal(anyhow::anyhow!(
                    "erro lendo corpo do PUT: {e}"
                )))
            }
            None => break,
        }
    }

    let info = storage
        .finalize_upload(uuid, &digest_hex)
        .await
        .map_err(storage_err)?;
    registry_db::insert_blob(db, &info.digest, info.size as i64).await?;
    Ok(created_blob_response(repo_name, &info.digest))
}

async fn delete_upload(storage: &RegistryStorage, uuid: &str) -> Result<Response<RegistryBody>, RegistryError> {
    storage.cancel_upload(uuid).await.map_err(storage_err)?;
    Ok(Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(empty_body())
        .expect("valid DELETE upload response"))
}

fn storage_err(e: StorageError) -> RegistryError {
    match e {
        StorageError::UnknownUpload => RegistryError::BlobUploadUnknown("upload session not found".into()),
        StorageError::DigestMismatch { expected, got } => {
            RegistryError::DigestInvalid(format!("expected {expected}, got {got}"))
        }
        StorageError::Io(e) => RegistryError::Internal(e.into()),
    }
}

fn upload_accepted_response(repo_name: &str, uuid: &str, written: u64) -> Response<RegistryBody> {
    Response::builder()
        .status(StatusCode::ACCEPTED)
        .header("Location", format!("/v2/{repo_name}/blobs/uploads/{uuid}"))
        .header("Docker-Upload-UUID", uuid)
        // Formato próprio da OCI Distribution Spec: "<start>-<end>", SEM o
        // prefixo de unidade "bytes=" do Range HTTP genérico (RFC 7233) — o
        // client do docker faz parse ingênuo de inteiros e quebra com
        // "expected integer" se o prefixo estiver presente (confirmado com
        // smoke test real: docker push falhava até essa correção).
        .header("Range", format!("0-{}", written.saturating_sub(1)))
        .body(empty_body())
        .expect("valid upload-accepted response")
}

fn created_blob_response(repo_name: &str, digest_hex: &str) -> Response<RegistryBody> {
    Response::builder()
        .status(StatusCode::CREATED)
        .header("Location", format!("/v2/{repo_name}/blobs/sha256:{digest_hex}"))
        .header("Docker-Content-Digest", format!("sha256:{digest_hex}"))
        .body(empty_body())
        .expect("valid blob-created response")
}

// ── Manifests ────────────────────────────────────────────────────────────

async fn manifest(
    method: Method,
    req: Request<Incoming>,
    db: &Db,
    storage: &RegistryStorage,
    repo_name: &str,
    reference: &str,
) -> Result<Response<RegistryBody>, RegistryError> {
    if !name::is_valid_name(repo_name) {
        return Err(RegistryError::NameInvalid(repo_name.to_string()));
    }
    match method {
        Method::HEAD => get_manifest(db, storage, repo_name, reference, true).await,
        Method::GET => get_manifest(db, storage, repo_name, reference, false).await,
        Method::PUT => put_manifest(req, db, storage, repo_name, reference).await,
        Method::DELETE => delete_manifest_route(db, repo_name, reference).await,
        _ => Err(RegistryError::ManifestInvalid(format!(
            "método {method} inválido para manifest"
        ))),
    }
}

async fn resolve_manifest_digest(
    db: &Db,
    repo_id: &str,
    reference: &str,
) -> Result<String, RegistryError> {
    if let Some(d) = name::parse_digest(reference) {
        return Ok(d.to_string());
    }
    if !name::is_valid_tag(reference) {
        return Err(RegistryError::ManifestInvalid(reference.to_string()));
    }
    registry_db::get_tag_digest(db, repo_id, reference)
        .await?
        .ok_or_else(|| RegistryError::ManifestUnknown(reference.to_string()))
}

async fn get_manifest(
    db: &Db,
    storage: &RegistryStorage,
    repo_name: &str,
    reference: &str,
    head_only: bool,
) -> Result<Response<RegistryBody>, RegistryError> {
    let repo = registry_db::get_repo_by_name(db, repo_name)
        .await?
        .ok_or_else(|| RegistryError::NameUnknown(repo_name.to_string()))?;

    let digest_hex = resolve_manifest_digest(db, &repo.id, reference).await?;
    let row = registry_db::get_manifest(db, &repo.id, &digest_hex)
        .await?
        .ok_or_else(|| RegistryError::ManifestUnknown(digest_hex.clone()))?;

    if head_only {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", row.media_type)
            .header("Docker-Content-Digest", format!("sha256:{digest_hex}"))
            .header("Content-Length", row.size.to_string())
            .body(empty_body())
            .expect("valid HEAD manifest response"));
    }

    let bytes = storage.read_blob(&digest_hex).await?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", row.media_type)
        .header("Docker-Content-Digest", format!("sha256:{digest_hex}"))
        .header("Content-Length", row.size.to_string())
        .body(Full::new(Bytes::from(bytes)).boxed())
        .expect("valid GET manifest response"))
}

async fn put_manifest(
    req: Request<Incoming>,
    db: &Db,
    storage: &RegistryStorage,
    repo_name: &str,
    reference: &str,
) -> Result<Response<RegistryBody>, RegistryError> {
    const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;

    let content_type = req
        .headers()
        .get(hyper::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/vnd.docker.distribution.manifest.v2+json")
        .to_string();

    let body = collect_body(req).await?;
    if body.len() > MAX_MANIFEST_BYTES {
        return Err(RegistryError::ManifestInvalid("manifest excede 4 MiB".to_string()));
    }

    let value: serde_json::Value =
        serde_json::from_slice(&body).map_err(|e| RegistryError::ManifestInvalid(format!("JSON inválido: {e}")))?;
    let refs = extract_refs(&value)?;

    let mut ref_hexes = Vec::with_capacity(refs.len());
    for r in &refs {
        let digest_hex = name::parse_digest(r).ok_or_else(|| RegistryError::DigestInvalid(r.clone()))?;
        if !registry_db::ref_blob_or_manifest_exists(db, digest_hex).await? {
            return Err(RegistryError::ManifestBlobUnknown(r.clone()));
        }
        ref_hexes.push(digest_hex.to_string());
    }

    // Digest = sha256 dos bytes CRUS recebidos, nunca re-serializados.
    let info = storage.write_blob_direct(&body).await.map_err(storage_err)?;

    let is_digest_ref = name::parse_digest(reference).is_some();
    if let Some(expected_hex) = name::parse_digest(reference) {
        if expected_hex != info.digest {
            return Err(RegistryError::DigestInvalid(reference.to_string()));
        }
    } else if !name::is_valid_tag(reference) {
        return Err(RegistryError::ManifestInvalid(reference.to_string()));
    }

    let repo = registry_db::get_or_create_repo(db, repo_name).await?;
    registry_db::insert_manifest(db, &info.digest, &repo.id, &content_type, info.size as i64, &ref_hexes).await?;

    if !is_digest_ref {
        registry_db::upsert_tag(db, &repo.id, reference, &info.digest).await?;
    }

    Ok(Response::builder()
        .status(StatusCode::CREATED)
        .header("Location", format!("/v2/{repo_name}/manifests/sha256:{}", info.digest))
        .header("Docker-Content-Digest", format!("sha256:{}", info.digest))
        .body(empty_body())
        .expect("valid manifest-created response"))
}

/// Extrai os digests referenciados por um manifest: `manifests[].digest` para
/// um index/manifest-list (multi-arch), ou `config.digest` + `layers[].digest`
/// para um manifest simples.
fn extract_refs(value: &serde_json::Value) -> Result<Vec<String>, RegistryError> {
    let mut refs = Vec::new();
    if let Some(manifests) = value.get("manifests").and_then(|v| v.as_array()) {
        for m in manifests {
            let digest = m
                .get("digest")
                .and_then(|d| d.as_str())
                .ok_or_else(|| RegistryError::ManifestInvalid("manifest index sem digest".to_string()))?;
            refs.push(digest.to_string());
        }
        return Ok(refs);
    }
    if let Some(digest) = value.get("config").and_then(|c| c.get("digest")).and_then(|d| d.as_str()) {
        refs.push(digest.to_string());
    }
    if let Some(layers) = value.get("layers").and_then(|v| v.as_array()) {
        for l in layers {
            let digest = l
                .get("digest")
                .and_then(|d| d.as_str())
                .ok_or_else(|| RegistryError::ManifestInvalid("layer sem digest".to_string()))?;
            refs.push(digest.to_string());
        }
    }
    Ok(refs)
}

async fn delete_manifest_route(
    db: &Db,
    repo_name: &str,
    reference: &str,
) -> Result<Response<RegistryBody>, RegistryError> {
    let digest_hex = name::parse_digest(reference)
        .ok_or_else(|| RegistryError::ManifestInvalid("DELETE exige digest, não tag".to_string()))?;
    let repo = registry_db::get_repo_by_name(db, repo_name)
        .await?
        .ok_or_else(|| RegistryError::NameUnknown(repo_name.to_string()))?;
    let deleted = registry_db::delete_manifest(db, &repo.id, digest_hex).await?;
    if !deleted {
        return Err(RegistryError::ManifestUnknown(reference.to_string()));
    }
    Ok(Response::builder()
        .status(StatusCode::ACCEPTED)
        .body(empty_body())
        .expect("valid DELETE manifest response"))
}

// ── Helpers ──────────────────────────────────────────────────────────────

async fn collect_body(req: Request<Incoming>) -> Result<Bytes, RegistryError> {
    req.into_body()
        .collect()
        .await
        .map(|c| c.to_bytes())
        .map_err(|e| RegistryError::Internal(anyhow::anyhow!("erro lendo corpo: {e}")))
}

fn empty_body() -> RegistryBody {
    Full::new(Bytes::new()).boxed()
}

fn json_response(status: StatusCode, value: &serde_json::Value) -> Result<Response<RegistryBody>, RegistryError> {
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(value.to_string())).boxed())
        .expect("valid JSON response"))
}

/// Extrai `key` da query string, com percent-decoding (mesmo esquema de
/// `api::webhook_server::percent_decode`, duplicado localmente — pequeno o
/// bastante para não valer uma dependência de URL parsing).
fn query_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then(|| percent_decode(v))
    })
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push((hi * 16 + lo) as u8);
                    i += 3;
                    continue;
                }
                out.push(b'%');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use ulid::Ulid;

    /// Bind em porta efêmera + DB/storage temporários, servindo em background.
    /// Sem `reqwest` — cliente hyper cru, mesmo padrão de
    /// `ingress/proxy.rs::forward` (ver `docs/plano-registry-embutido.md`).
    async fn spawn_test_registry() -> std::net::SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let dir = std::env::temp_dir().join(format!("rp_registry_http_test_{}", Ulid::new()));
        let db = Arc::new(crate::db::connect(&dir).await.unwrap());
        let storage = Arc::new(RegistryStorage::new(dir.join("registry")).unwrap());
        tokio::spawn(serve(listener, db, storage));
        addr
    }

    fn build_req(
        method: Method,
        addr: std::net::SocketAddr,
        path: &str,
        content_type: Option<&str>,
        body: Vec<u8>,
    ) -> Request<Full<Bytes>> {
        let mut builder = Request::builder()
            .method(method)
            .uri(path)
            .header("Host", addr.to_string());
        if let Some(ct) = content_type {
            builder = builder.header("Content-Type", ct);
        }
        builder.body(Full::new(Bytes::from(body))).unwrap()
    }

    async fn send(
        addr: std::net::SocketAddr,
        req: Request<Full<Bytes>>,
    ) -> (StatusCode, hyper::HeaderMap, Bytes) {
        let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let io = TokioIo::new(stream);
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await.unwrap();
        tokio::spawn(async move {
            let _ = conn.await;
        });
        let resp = sender.send_request(req).await.unwrap();
        let status = resp.status();
        let headers = resp.headers().clone();
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        (status, headers, body)
    }

    #[tokio::test]
    async fn push_pull_fluxo_completo() {
        let addr = spawn_test_registry().await;

        // 1. GET /v2/ — sem auth nesta fase.
        let (status, headers, _) = send(addr, build_req(Method::GET, addr, "/v2/", None, vec![])).await;
        assert_eq!(status, StatusCode::OK);
        assert!(headers.get("WWW-Authenticate").is_none());

        // 2. POST blobs/uploads/ — inicia sessão.
        let (status, headers, _) = send(
            addr,
            build_req(Method::POST, addr, "/v2/hello/blobs/uploads/", None, vec![]),
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        let location = headers.get("Location").unwrap().to_str().unwrap().to_string();

        // 3. PATCH — envia o chunk (blob inteiro, num chunk só).
        let chunk = b"hello world blob content".to_vec();
        let (status, headers, _) = send(addr, build_req(Method::PATCH, addr, &location, None, chunk.clone())).await;
        assert_eq!(status, StatusCode::ACCEPTED);
        let range = headers.get("Range").unwrap().to_str().unwrap().to_string();
        assert_eq!(range, format!("0-{}", chunk.len() - 1));

        // 4. PUT ?digest= — finaliza.
        let blob_digest = hex::encode(Sha256::digest(&chunk));
        let put_path = format!("{location}?digest=sha256:{blob_digest}");
        let (status, headers, _) = send(addr, build_req(Method::PUT, addr, &put_path, None, vec![])).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(
            headers.get("Docker-Content-Digest").unwrap().to_str().unwrap(),
            format!("sha256:{blob_digest}")
        );

        // 5. Manifest schema2 mínimo, reusando o mesmo blob como config e layer.
        let manifest = serde_json::json!({
            "schemaVersion": 2,
            "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
            "config": {
                "mediaType": "application/vnd.docker.container.image.v1+json",
                "size": chunk.len(),
                "digest": format!("sha256:{blob_digest}"),
            },
            "layers": [{
                "mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
                "size": chunk.len(),
                "digest": format!("sha256:{blob_digest}"),
            }],
        });
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let manifest_digest = hex::encode(Sha256::digest(&manifest_bytes));

        // 6. PUT manifests/v1 (por tag).
        let (status, headers, _) = send(
            addr,
            build_req(
                Method::PUT,
                addr,
                "/v2/hello/manifests/v1",
                Some("application/vnd.docker.distribution.manifest.v2+json"),
                manifest_bytes.clone(),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(
            headers.get("Docker-Content-Digest").unwrap().to_str().unwrap(),
            format!("sha256:{manifest_digest}")
        );

        // 7. GET manifests/v1 (por tag) — corpo idêntico byte-a-byte.
        let (status, headers, body) =
            send(addr, build_req(Method::GET, addr, "/v2/hello/manifests/v1", None, vec![])).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers.get("Content-Type").unwrap().to_str().unwrap(),
            "application/vnd.docker.distribution.manifest.v2+json"
        );
        assert_eq!(body.as_ref(), manifest_bytes.as_slice());

        // 8. GET manifests/sha256:... (por digest) — mesmo corpo.
        let by_digest = format!("/v2/hello/manifests/sha256:{manifest_digest}");
        let (status, _, body) = send(addr, build_req(Method::GET, addr, &by_digest, None, vec![])).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.as_ref(), manifest_bytes.as_slice());

        // 9. tags/list
        let (status, _, body) = send(
            addr,
            build_req(Method::GET, addr, "/v2/hello/tags/list", None, vec![]),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["name"], "hello");
        assert_eq!(v["tags"], serde_json::json!(["v1"]));

        // 10. _catalog
        let (status, _, body) = send(addr, build_req(Method::GET, addr, "/v2/_catalog", None, vec![])).await;
        assert_eq!(status, StatusCode::OK);
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["repositories"], serde_json::json!(["hello"]));
    }

    #[tokio::test]
    async fn manifest_de_repo_inexistente_e_404() {
        let addr = spawn_test_registry().await;
        let (status, _, body) = send(
            addr,
            build_req(Method::GET, addr, "/v2/nope/manifests/v1", None, vec![]),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["errors"][0]["code"], "NAME_UNKNOWN");
    }

    #[tokio::test]
    async fn uuid_de_upload_invalido_e_404() {
        let addr = spawn_test_registry().await;
        let fake_digest = "0".repeat(64);
        let path = format!("/v2/hello/blobs/uploads/does-not-exist?digest=sha256:{fake_digest}");
        let (status, _, body) = send(addr, build_req(Method::PUT, addr, &path, None, vec![])).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["errors"][0]["code"], "BLOB_UPLOAD_UNKNOWN");
    }

    #[tokio::test]
    async fn manifest_referenciando_blob_inexistente_e_400() {
        let addr = spawn_test_registry().await;
        let fake_digest = "1".repeat(64);
        let manifest = serde_json::json!({
            "schemaVersion": 2,
            "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
            "config": {
                "mediaType": "application/vnd.docker.container.image.v1+json",
                "size": 10,
                "digest": format!("sha256:{fake_digest}"),
            },
            "layers": [],
        });
        let body_bytes = serde_json::to_vec(&manifest).unwrap();
        let (status, _, body) = send(
            addr,
            build_req(
                Method::PUT,
                addr,
                "/v2/hello/manifests/v1",
                Some("application/vnd.docker.distribution.manifest.v2+json"),
                body_bytes,
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["errors"][0]["code"], "MANIFEST_BLOB_UNKNOWN");
    }
}
