pub mod handlers;
pub mod routes;
pub mod stream;

use crate::{
    db::Db,
    docker::DockerClient,
    event_bus::EventBus,
    ingress::IngressController,
    secrets::SecretsManager,
};
use axum::{
    body::Bytes,
    extract::FromRequest,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::{de::DeserializeOwned, Serialize};
use std::{path::PathBuf, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
    pub docker: Arc<DockerClient>,
    pub ingress: Arc<IngressController>,
    pub bus: Arc<EventBus>,
    pub secrets: Arc<SecretsManager>,
    pub db_path: PathBuf,
    pub drain_secs: u64,
    pub started_at: std::time::Instant,
}

impl AppState {
    pub fn new(
        db: Arc<Db>,
        docker: Arc<DockerClient>,
        ingress: Arc<IngressController>,
        bus: Arc<EventBus>,
        secrets: Arc<SecretsManager>,
        db_path: PathBuf,
        drain_secs: u64,
    ) -> Self {
        Self {
            db,
            docker,
            ingress,
            bus,
            secrets,
            db_path,
            drain_secs,
            started_at: std::time::Instant::now(),
        }
    }
}

pub struct Bincode<T>(pub T);

impl<T: Serialize> IntoResponse for Bincode<T> {
    fn into_response(self) -> Response {
        match bincode::serialize(&self.0) {
            Ok(bytes) => {
                ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response()
            }
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[axum::async_trait]
impl<S, T> FromRequest<S> for Bincode<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(
        req: axum::http::Request<axum::body::Body>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        let val = bincode::deserialize(&bytes)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("bincode: {e}")))?;
        Ok(Bincode(val))
    }
}
