//! Envelope de erro da OCI Distribution Spec:
//! `{"errors":[{"code","message","detail"}]}`, sempre com o header
//! `Docker-Distribution-API-Version: registry/2.0` que o `docker` CLI exige
//! em toda resposta (sucesso ou erro).

use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full};
use hyper::{Response, StatusCode};

/// Body unificado das respostas do registry — mesmo padrão de `ApiBody` em
/// `api/http_api.rs`.
pub type RegistryBody = BoxBody<Bytes, Infallible>;

#[derive(Debug)]
pub enum RegistryError {
    NameInvalid(String),
    NameUnknown(String),
    BlobUnknown(String),
    BlobUploadUnknown(String),
    BlobUploadInvalid(String),
    DigestInvalid(String),
    ManifestUnknown(String),
    ManifestInvalid(String),
    ManifestBlobUnknown(String),
    Internal(anyhow::Error),
}

impl RegistryError {
    fn code(&self) -> &'static str {
        match self {
            RegistryError::NameInvalid(_) => "NAME_INVALID",
            RegistryError::NameUnknown(_) => "NAME_UNKNOWN",
            RegistryError::BlobUnknown(_) => "BLOB_UNKNOWN",
            RegistryError::BlobUploadUnknown(_) => "BLOB_UPLOAD_UNKNOWN",
            RegistryError::BlobUploadInvalid(_) => "BLOB_UPLOAD_INVALID",
            RegistryError::DigestInvalid(_) => "DIGEST_INVALID",
            RegistryError::ManifestUnknown(_) => "MANIFEST_UNKNOWN",
            RegistryError::ManifestInvalid(_) => "MANIFEST_INVALID",
            RegistryError::ManifestBlobUnknown(_) => "MANIFEST_BLOB_UNKNOWN",
            RegistryError::Internal(_) => "UNKNOWN",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            RegistryError::NameUnknown(_)
            | RegistryError::BlobUnknown(_)
            | RegistryError::BlobUploadUnknown(_)
            | RegistryError::ManifestUnknown(_) => StatusCode::NOT_FOUND,
            RegistryError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        }
    }

    fn message(&self) -> String {
        match self {
            RegistryError::NameInvalid(s) => format!("invalid repository name: {s}"),
            RegistryError::NameUnknown(s) => format!("repository not found: {s}"),
            RegistryError::BlobUnknown(s) => format!("blob unknown: {s}"),
            RegistryError::BlobUploadUnknown(s) => format!("blob upload unknown: {s}"),
            RegistryError::BlobUploadInvalid(s) => format!("blob upload invalid: {s}"),
            RegistryError::DigestInvalid(s) => format!("digest invalid: {s}"),
            RegistryError::ManifestUnknown(s) => format!("manifest unknown: {s}"),
            RegistryError::ManifestInvalid(s) => format!("manifest invalid: {s}"),
            RegistryError::ManifestBlobUnknown(s) => format!("manifest blob unknown: {s}"),
            RegistryError::Internal(e) => format!("internal error: {e}"),
        }
    }

    pub fn into_response(self) -> Response<RegistryBody> {
        if let RegistryError::Internal(e) = &self {
            tracing::error!(error = %e, "registry: erro interno");
        }
        let body = serde_json::json!({
            "errors": [{ "code": self.code(), "message": self.message(), "detail": null }]
        });
        Response::builder()
            .status(self.status())
            .header("Content-Type", "application/json")
            .header("Docker-Distribution-API-Version", "registry/2.0")
            .body(Full::new(Bytes::from(body.to_string())).boxed())
            .expect("static registry error response")
    }
}

impl From<anyhow::Error> for RegistryError {
    fn from(e: anyhow::Error) -> Self {
        RegistryError::Internal(e)
    }
}

impl From<std::io::Error> for RegistryError {
    fn from(e: std::io::Error) -> Self {
        RegistryError::Internal(e.into())
    }
}

impl From<sqlx::Error> for RegistryError {
    fn from(e: sqlx::Error) -> Self {
        RegistryError::Internal(e.into())
    }
}
