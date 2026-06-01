use std::convert::Infallible;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Method, Request, Response, StatusCode, body::Incoming};
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tracing::{error, info, warn};

use super::AppState;
use crate::db::webhook_tokens;

pub async fn run(state: AppState, port: u16) {
    let addr: std::net::SocketAddr = ([0, 0, 0, 0], port).into();
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            error!(port, error = %e, "webhook server: falha ao bind");
            return;
        }
    };
    info!(port, "webhook server: escutando");

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(c) => c,
            Err(e) => {
                warn!(error = %e, "webhook server: accept error");
                continue;
            }
        };
        let state = state.clone();
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            if let Err(e) = hyper::server::conn::http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(move |req| handle(req, state.clone())),
                )
                .await
            {
                warn!(peer = %peer, error = %e, "webhook server: connection error");
            }
        });
    }
}

async fn handle(req: Request<Incoming>, state: AppState) -> Result<Response<Full<Bytes>>, Infallible> {
    // Aceita apenas POST /webhook/{service_id}/{token}
    if req.method() != Method::POST {
        return Ok(resp(StatusCode::METHOD_NOT_ALLOWED, "method not allowed"));
    }

    let path = req.uri().path().to_owned();
    let parts: Vec<&str> = path.trim_start_matches('/').splitn(3, '/').collect();
    if parts.len() != 3 || parts[0] != "webhook" {
        return Ok(resp(StatusCode::NOT_FOUND, "not found"));
    }

    let service_id = parts[1].to_string();
    let provided_token = parts[2].to_string();

    let stored = match webhook_tokens::get(&state.db, &service_id).await {
        Ok(Some(t)) => t,
        Ok(None) => return Ok(resp(StatusCode::UNAUTHORIZED, "invalid token")),
        Err(e) => {
            error!(service_id = %service_id, error = %e, "webhook: db error");
            return Ok(resp(StatusCode::INTERNAL_SERVER_ERROR, "internal error"));
        }
    };

    if stored != provided_token {
        return Ok(resp(StatusCode::UNAUTHORIZED, "invalid token"));
    }

    info!(service_id = %service_id, "webhook: disparando deploy");
    tokio::spawn(async move {
        crate::api::handlers::deploy_start::handle(state, service_id).await;
    });

    Ok(resp(StatusCode::OK, "deploy triggered"))
}

fn resp(status: StatusCode, body: &'static str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}
