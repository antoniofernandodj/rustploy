//! Proxy reverso HTTP/1.1 embutido, construído sobre hyper.
//! Lê o header `Host`, consulta a tabela de rotas (arc-swap, lock-free) e
//! encaminha a requisição para o backend correspondente via TCP.

use crate::ingress::router::{PortBackend, RouteHandle};
use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Empty};
use hyper::{body::Incoming, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, info, warn};

type ProxyBody = BoxBody<Bytes, hyper::Error>;

pub async fn start_proxy(routes: RouteHandle, http_port: u16, _https_port: u16) {
    let addr: SocketAddr = format!("0.0.0.0:{http_port}")
        .parse()
        .expect("valid addr");
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            warn!(error = %e, port = http_port, "failed to bind ingress port");
            return;
        }
    };
    tracing::info!(port = http_port, "ingress proxy listening");

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "accept error");
                continue;
            }
        };
        let routes = routes.clone();
        tokio::spawn(serve_connection(stream, routes));
    }
}

async fn serve_connection(stream: TcpStream, routes: RouteHandle) {
    use hyper::server::conn::http1;
    use hyper::service::service_fn;

    let io = TokioIo::new(stream);
    let svc = service_fn(move |req: Request<Incoming>| {
        let routes = routes.clone();
        async move { handle(req, routes).await }
    });

    if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
        debug!(error = %e, "proxy connection closed");
    }
}

async fn handle(
    req: Request<Incoming>,
    routes: RouteHandle,
) -> Result<Response<ProxyBody>, std::convert::Infallible> {
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .map(|h| h.split(':').next().unwrap_or(h).to_string())
        .unwrap_or_default();

    let backend = {
        let table = routes.load();
        table.get(&host).map(|e| e.backend_addr.clone())
    };

    let Some(backend_addr) = backend else {
        warn!(host, "no route");
        return Ok(status_response(StatusCode::NOT_FOUND));
    };

    let stream = match TcpStream::connect(&backend_addr).await {
        Ok(s) => s,
        Err(e) => {
            warn!(backend = backend_addr, error = %e, "backend connect failed");
            return Ok(status_response(StatusCode::BAD_GATEWAY));
        }
    };

    let io = TokioIo::new(stream);
    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "backend handshake failed");
            return Ok(status_response(StatusCode::BAD_GATEWAY));
        }
    };

    tokio::spawn(async move { let _ = conn.await; });

    match sender.send_request(req).await {
        Ok(resp) => {
            let (parts, body) = resp.into_parts();
            let boxed = body.map_err(|e| e).boxed();
            Ok(Response::from_parts(parts, boxed))
        }
        Err(e) => {
            warn!(error = %e, "backend request error");
            Ok(status_response(StatusCode::BAD_GATEWAY))
        }
    }
}

fn status_response(status: StatusCode) -> Response<ProxyBody> {
    Response::builder()
        .status(status)
        .body(Empty::new().map_err(|e| match e {}).boxed())
        .unwrap()
}

/// Listener HTTP dedicado para uma porta específica de serviço.
/// Usa hyper (igual ao proxy de domínio) para tratar keep-alive e fechar
/// a conexão corretamente ao final de cada resposta.
pub async fn serve_port_proxy(port: u16, backend: PortBackend) {
    let addr: SocketAddr = format!("0.0.0.0:{port}").parse().expect("valid addr");
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            warn!(port, error = %e, "port proxy: bind failed");
            return;
        }
    };
    info!(port, "port proxy: listening");
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                warn!(port, error = %e, "port proxy: accept error");
                continue;
            }
        };
        let backend = backend.clone();
        tokio::spawn(serve_port_connection(stream, backend));
    }
}

async fn serve_port_connection(stream: TcpStream, backend: PortBackend) {
    use hyper::server::conn::http1;
    use hyper::service::service_fn;

    let io = TokioIo::new(stream);
    let svc = service_fn(move |req: Request<Incoming>| {
        let backend = backend.clone();
        async move { handle_port(req, backend).await }
    });
    if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
        debug!(error = %e, "port proxy connection closed");
    }
}

async fn handle_port(
    req: Request<Incoming>,
    backend: PortBackend,
) -> Result<Response<ProxyBody>, std::convert::Infallible> {
    let Some(backend_addr) = (**backend.load()).clone() else {
        return Ok(status_response(StatusCode::SERVICE_UNAVAILABLE));
    };

    let stream = match TcpStream::connect(&backend_addr).await {
        Ok(s) => s,
        Err(e) => {
            warn!(backend = backend_addr, error = %e, "port proxy: backend connect failed");
            return Ok(status_response(StatusCode::BAD_GATEWAY));
        }
    };

    let io = TokioIo::new(stream);
    let (mut sender, conn) = match hyper::client::conn::http1::handshake(io).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "port proxy: backend handshake failed");
            return Ok(status_response(StatusCode::BAD_GATEWAY));
        }
    };
    tokio::spawn(async move { let _ = conn.await; });

    match sender.send_request(req).await {
        Ok(resp) => {
            let (parts, body) = resp.into_parts();
            Ok(Response::from_parts(parts, body.map_err(|e| e).boxed()))
        }
        Err(e) => {
            warn!(error = %e, "port proxy: backend request failed");
            Ok(status_response(StatusCode::BAD_GATEWAY))
        }
    }
}
