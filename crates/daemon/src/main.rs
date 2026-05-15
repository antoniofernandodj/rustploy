use axum::{
    async_trait,
    body::Bytes,
    extract::FromRequest,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use serde::{de::DeserializeOwned, Serialize};
use shared::Message;
use std::path::PathBuf;
use tokio::net::UnixListener;

// Wrapper Bincode para Axum
pub struct Bincode<T>(pub T);

impl<T> IntoResponse for Bincode<T> where T: Serialize {
    fn into_response(self) -> Response {
        match bincode::serialize(&self.0) {
            Ok(bytes) => (
                [(header::CONTENT_TYPE, "application/octet-stream")],
                bytes,
            ).into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        }
    }
}

#[async_trait]
impl<S, T> FromRequest<S> for Bincode<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: axum::http::Request<axum::body::Body>, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let res = bincode::deserialize(&bytes)
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Bincode error: {}", e)))?;

        Ok(Bincode(res))
    }
}


async fn echo_handler(Bincode(msg): Bincode<Message>) -> Bincode<Message> {
    println!("Received: {:?}", msg);
    Bincode(msg)
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from("/tmp/rustploy_echo.sock");
    
    // Remove o socket antigo se existir
    if path.exists() {
        std::fs::remove_file(&path)?;
    }

    let listener = UnixListener::bind(&path)?;
    println!("Server listening on UDS: {:?}", path);

    let app = Router::new()
        .route("/", post(echo_handler));

    loop {
        let (stream, _) = listener.accept().await?;
        let stream = TokioIo::new(stream);
        let service = hyper_util::service::TowerToHyperService::new(app.clone());

        tokio::spawn(async move {
            if let Err(err) = auto::Builder::new(TokioExecutor::new())
                .serve_connection(stream, service)
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
