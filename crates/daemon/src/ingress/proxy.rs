//! Proxy reverso HTTP/1.1 embutido, construído sobre hyper.
//! Lê o header `Host`, consulta a tabela de rotas (arc-swap, lock-free) e
//! encaminha a requisição para o backend correspondente via TCP.
//!
//! HTTPS: listener TLS separado com SNI — seleciona o certificado correto por domínio.
//! HTTP:  redireciona para HTTPS, exceto o path /.well-known/acme-challenge/* (ACME HTTP-01).

use crate::ingress::{
    router::{PortBackend, RouteHandle},
    tls::{ChallengeStore, TlsManager},
};
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full, combinators::BoxBody};
use hyper::{Request, Response, StatusCode, body::Incoming};
use hyper_util::rt::TokioIo;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, info, warn};

type ProxyBody = BoxBody<Bytes, hyper::Error>;

/// Inicia o proxy de ingress.
///
/// - Sempre sobe listener HTTP em `http_port`.
/// - Se `tls` for Some, sobe também listener HTTPS em `https_port` e
///   redireciona HTTP → HTTPS (preservando o path ACME para desafios).
pub async fn start_proxy(
    routes: RouteHandle,
    http_port: u16,
    https_port: u16,
    tls: Option<Arc<TlsManager>>,
) {
    // ── Listener HTTPS ────────────────────────────────────────────────────────
    if let Some(ref tls_mgr) = tls {
        let routes_https = routes.clone();
        let acceptor = tls_mgr.tls_acceptor();
        let https_addr: SocketAddr = format!("0.0.0.0:{https_port}").parse().unwrap();
        tokio::spawn(async move {
            start_https_listener(routes_https, acceptor, https_addr).await;
        });
    }

    // ── Listener HTTP ─────────────────────────────────────────────────────────
    let addr: SocketAddr = format!("0.0.0.0:{http_port}").parse().unwrap();
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            warn!(error = %e, port = http_port, "failed to bind HTTP ingress port");
            return;
        }
    };
    info!(port = http_port, "HTTP ingress proxy listening");

    let challenges: Option<ChallengeStore> = tls.as_ref().map(|t| t.challenges.clone());
    let redirect_https = tls.is_some();

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "HTTP accept error");
                continue;
            }
        };
        let routes = routes.clone();
        let challenges = challenges.clone();
        tokio::spawn(serve_http_connection(
            stream,
            routes,
            challenges,
            redirect_https,
        ));
    }
}

// ─── HTTP ─────────────────────────────────────────────────────────────────────

async fn serve_http_connection(
    stream: TcpStream,
    routes: RouteHandle,
    challenges: Option<ChallengeStore>,
    redirect_https: bool,
) {
    use hyper::server::conn::http1;
    use hyper::service::service_fn;

    let io = TokioIo::new(stream);
    let svc = service_fn(move |req: Request<Incoming>| {
        let routes = routes.clone();
        let challenges = challenges.clone();
        async move { handle_http(req, routes, challenges, redirect_https).await }
    });

    if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
        debug!(error = %e, "HTTP proxy connection closed");
    }
}

async fn handle_http(
    req: Request<Incoming>,
    routes: RouteHandle,
    challenges: Option<ChallengeStore>,
    redirect_https: bool,
) -> Result<Response<ProxyBody>, std::convert::Infallible> {
    let path = req.uri().path().to_string();

    // ── ACME HTTP-01 challenge ────────────────────────────────────────────────
    if let Some(token) = path.strip_prefix("/.well-known/acme-challenge/") {
        if let Some(store) = &challenges {
            if let Some(key_auth) = store.lock().unwrap().get(token).cloned() {
                debug!(token = %token, "ACME: servindo challenge");
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("content-type", "text/plain")
                    .body(text_body(key_auth))
                    .unwrap());
            }
        }
        return Ok(status_response(StatusCode::NOT_FOUND));
    }

    // ── Redirect HTTP → HTTPS ─────────────────────────────────────────────────
    if redirect_https {
        let host = req
            .headers()
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("localhost");
        let pq = req
            .uri()
            .path_and_query()
            .map(|p| p.as_str())
            .unwrap_or("/");
        let location = format!("https://{host}{pq}");
        return Ok(Response::builder()
            .status(StatusCode::MOVED_PERMANENTLY)
            .header("location", location)
            .body(empty_body())
            .unwrap());
    }

    // ── Proxy normal (sem TLS ativo) ──────────────────────────────────────────
    handle(req, routes).await
}

// ─── HTTPS ────────────────────────────────────────────────────────────────────

async fn start_https_listener(
    routes: RouteHandle,
    acceptor: tokio_rustls::TlsAcceptor,
    addr: SocketAddr,
) {
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            warn!(error = %e, %addr, "failed to bind HTTPS ingress port");
            return;
        }
    };
    info!(%addr, "HTTPS ingress proxy listening");

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "HTTPS accept error");
                continue;
            }
        };
        let routes = routes.clone();
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            match acceptor.accept(stream).await {
                Ok(tls) => serve_https_connection(tls, routes).await,
                Err(e) => debug!(error = %e, "TLS handshake failed"),
            }
        });
    }
}

async fn serve_https_connection(
    stream: tokio_rustls::server::TlsStream<TcpStream>,
    routes: RouteHandle,
) {
    use hyper::server::conn::http1;
    use hyper::service::service_fn;

    let io = TokioIo::new(stream);
    let svc = service_fn(move |req: Request<Incoming>| {
        let routes = routes.clone();
        async move { handle(req, routes).await }
    });

    if let Err(e) = http1::Builder::new().serve_connection(io, svc).await {
        debug!(error = %e, "HTTPS proxy connection closed");
    }
}

// ─── Proxy core (HTTP e HTTPS) ────────────────────────────────────────────────

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
        table.get(&host).and_then(|e| e.next_backend())
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

    tokio::spawn(async move {
        let _ = conn.await;
    });

    match sender.send_request(req).await {
        Ok(resp) => {
            let (parts, body) = resp.into_parts();
            Ok(Response::from_parts(parts, body.map_err(|e| e).boxed()))
        }
        Err(e) => {
            warn!(error = %e, "backend request error");
            Ok(status_response(StatusCode::BAD_GATEWAY))
        }
    }
}

// ─── Port proxy (sem TLS, igual ao anterior) ─────────────────────────────────

/// Listener HTTP dedicado para uma porta específica de serviço.
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
    let Some(backend_addr) = (**backend.load()).as_ref().and_then(|b| b.next()) else {
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
    tokio::spawn(async move {
        let _ = conn.await;
    });

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

// ─── Body helpers ─────────────────────────────────────────────────────────────

fn empty_body() -> ProxyBody {
    Empty::new().map_err(|e| match e {}).boxed()
}

fn text_body(s: String) -> ProxyBody {
    Full::new(Bytes::from(s)).map_err(|e| match e {}).boxed()
}

fn status_response(status: StatusCode) -> Response<ProxyBody> {
    Response::builder()
        .status(status)
        .body(empty_body())
        .unwrap()
}
