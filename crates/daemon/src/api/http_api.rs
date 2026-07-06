//! HTTP/JSON + SSE control API — the successor to the RWP binary protocol.
//!
//! Serves the same `Command`→`Response` surface the RWP/UDS servers do, but over
//! plain HTTP so the glacier-ui client can drive it entirely from Luau
//! (`fetch`/`sse`). It reuses [`dispatch`] wholesale — only the transport
//! changes. Meant to bind loopback and sit behind the ingress proxy, which
//! terminates TLS for `rustploy.chiquitos.tech` and forwards here.
//!
//! - `POST /api/rpc` — body is a JSON-encoded [`Command`]; the reply is a
//!   JSON-encoded [`Response`](shared::Response). One endpoint covers every
//!   command; the client reshapes the (externally tagged) JSON in Luau.
//! - `GET /api/events` — Server-Sent Events. Emits an `event: snapshot` with the
//!   full dashboard state every 2s (replacing the old client-side 2s poll) plus
//!   an `event: bus` for each live [`Event`](shared::Event) from the event bus
//!   (logs, metrics, deploy progress). One connection replaces poll + stream.
//! - `GET /api/health` — liveness probe (`ok`).

use std::convert::Infallible;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::body::{Frame, Incoming};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use shared::{ApiConfig, Command, Response as RpResponse};
use tracing::{error, info, warn};

use super::routes::dispatch;
use super::AppState;

/// Unified response body: both the buffered (`Full`) replies and the streaming
/// (`StreamBody`) SSE body box to this, so a single `service_fn` can return
/// either. Error is `Infallible` since neither body ever fails to produce data.
type ApiBody = BoxBody<Bytes, Infallible>;

/// Starts the API listener. Returns on bind failure; otherwise loops forever.
/// Intended to be `tokio::spawn`ed from `main`.
pub async fn run(state: AppState, cfg: ApiConfig) {
    // Safety guard mirroring RWP: refuse a non-loopback bind without a token.
    if cfg.is_public_bind() && cfg.token.as_deref().unwrap_or("").is_empty() {
        warn!(
            bind = %cfg.bind_address,
            "API: bind não-loopback sem token configurado — listener NÃO iniciado. \
             Defina api.token (ou RUSTPLOY_API_TOKEN) para expor remotamente."
        );
        return;
    }

    let addr = format!("{}:{}", cfg.bind_address, cfg.port);
    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            error!(error = %e, addr = %addr, "API: falha ao bind, listener desabilitado");
            return;
        }
    };

    let token = Arc::new(cfg.token.clone().filter(|s| !s.is_empty()));
    info!(addr = %addr, auth = token.is_some(), "API HTTP/SSE: escutando");

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                warn!(error = %e, "API: accept falhou");
                continue;
            }
        };
        let state = state.clone();
        let token = token.clone();
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let svc = service_fn(move |req| handle(req, state.clone(), token.clone()));
            if let Err(e) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, svc)
                .await
            {
                // Long-lived SSE connections end with an error when the client
                // drops — expected, so keep it at debug.
                tracing::debug!(peer = %peer, error = %e, "API: conexão encerrada");
            }
        });
    }
}

async fn handle(
    req: Request<Incoming>,
    state: AppState,
    token: Arc<Option<String>>,
) -> Result<Response<ApiBody>, Infallible> {
    // Bearer-token auth (constant-time), only when a token is configured.
    if let Some(expected) = token.as_ref() {
        if !authorized(&req, expected) {
            return Ok(text(StatusCode::UNAUTHORIZED, "unauthorized"));
        }
    }

    Ok(match (req.method(), req.uri().path()) {
        (&Method::POST, "/api/rpc") => rpc(req, state).await,
        (&Method::GET, "/api/events") => events(state),
        (&Method::GET, "/api/health") => text(StatusCode::OK, "ok"),
        _ => text(StatusCode::NOT_FOUND, "not found"),
    })
}

/// `POST /api/rpc`: decode a `Command`, run it through `dispatch`, encode the
/// `Response` back as JSON.
async fn rpc(req: Request<Incoming>, state: AppState) -> Response<ApiBody> {
    let body = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(_) => return text(StatusCode::BAD_REQUEST, "erro ao ler o corpo"),
    };
    let cmd: Command = match serde_json::from_slice(&body) {
        Ok(c) => c,
        Err(e) => return text(StatusCode::BAD_REQUEST, format!("comando inválido: {e}")),
    };
    let resp = dispatch(state, cmd).await;
    match serde_json::to_vec(&resp) {
        Ok(bytes) => json_ok(bytes),
        Err(e) => text(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("erro ao serializar resposta: {e}"),
        ),
    }
}

/// `GET /api/events`: SSE stream. A producer task feeds an mpsc channel with the
/// periodic snapshot and every live bus event; the response body streams those
/// frames out until the client disconnects (which closes the channel).
fn events(state: AppState) -> Response<ApiBody> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        let mut bus_rx = state.bus.subscribe();
        let mut ticker = tokio::time::interval(std::time::Duration::from_secs(2));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        // Snapshot inicial imediato, para o cliente pintar sem esperar o 1º tick.
        if tx.send(sse_frame("snapshot", &snapshot(&state).await)).await.is_err() {
            return;
        }

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if tx.send(sse_frame("snapshot", &snapshot(&state).await)).await.is_err() {
                        break;
                    }
                }
                r = bus_rx.recv() => match r {
                    Ok(ev) => {
                        if let Ok(js) = serde_json::to_string(&ev) {
                            if tx.send(sse_frame("bus", &js)).await.is_err() {
                                break;
                            }
                        }
                    }
                    // Slow consumer dropped events: skip and carry on.
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        }
    });

    let stream = futures::stream::poll_fn(move |cx| {
        rx.poll_recv(cx)
            .map(|opt| opt.map(|b| Ok::<_, Infallible>(Frame::data(b))))
    });

    let body = StreamBody::new(stream).boxed();
    Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "text/event-stream")
        .header(hyper::header::CACHE_CONTROL, "no-cache")
        // Disable proxy buffering so events are flushed promptly through ingress.
        .header("X-Accel-Buffering", "no")
        .body(body)
        .unwrap()
}

/// Builds the full dashboard snapshot as one JSON object, reusing `dispatch`
/// for each piece — the same bundle the old client-side 2s poll fetched. The
/// client (Luau) reshapes/filters it. `services` fans out one `ServiceList` per
/// project, tagging each service with its project name.
async fn snapshot(state: &AppState) -> String {
    use serde_json::{json, Map, Value};
    let mut obj = Map::new();

    if let RpResponse::DaemonStatus(d) = dispatch(state.clone(), Command::DaemonStatus).await {
        obj.insert("status".into(), serde_json::to_value(d).unwrap_or(Value::Null));
    }
    if let RpResponse::DeploymentSummaries(list) =
        dispatch(state.clone(), Command::RecentDeployments { limit: 40 }).await
    {
        obj.insert("deployments".into(), serde_json::to_value(list).unwrap_or(Value::Null));
    }
    if let RpResponse::Projects(projects) = dispatch(state.clone(), Command::ProjectList).await {
        let mut services = Vec::new();
        for p in &projects {
            if let RpResponse::Services(svcs) =
                dispatch(state.clone(), Command::ServiceList { project_id: p.id.clone() }).await
            {
                for s in svcs {
                    services.push(json!({ "project_name": p.name, "service": s }));
                }
            }
        }
        obj.insert("projects".into(), serde_json::to_value(&projects).unwrap_or(Value::Null));
        obj.insert("services".into(), Value::Array(services));
    }
    if let RpResponse::DockerImages(list) = dispatch(state.clone(), Command::DockerImages).await {
        obj.insert("docker_images".into(), serde_json::to_value(list).unwrap_or(Value::Null));
    }
    if let RpResponse::DockerVolumes(list) = dispatch(state.clone(), Command::DockerVolumes).await {
        obj.insert("docker_volumes".into(), serde_json::to_value(list).unwrap_or(Value::Null));
    }
    if let RpResponse::DockerNetworks(list) = dispatch(state.clone(), Command::DockerNetworks).await {
        obj.insert("docker_networks".into(), serde_json::to_value(list).unwrap_or(Value::Null));
    }
    if let RpResponse::DeployEngineStatus(eng) =
        dispatch(state.clone(), Command::DeployEngineStatus).await
    {
        obj.insert("engine".into(), serde_json::to_value(eng).unwrap_or(Value::Null));
    }

    Value::Object(obj).to_string()
}

/// Formats one SSE record. `data` is single-line JSON (`serde_json::to_string`
/// never emits raw newlines), so a single `data:` line is safe.
fn sse_frame(event: &str, data: &str) -> Bytes {
    Bytes::from(format!("event: {event}\ndata: {data}\n\n"))
}

/// Checks the `Authorization: Bearer <token>` header against `expected`.
fn authorized(req: &Request<Incoming>, expected: &str) -> bool {
    let got = req
        .headers()
        .get(hyper::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let got = got.strip_prefix("Bearer ").unwrap_or(got);
    constant_time_eq(got.as_bytes(), expected.as_bytes())
}

fn text(status: StatusCode, body: impl Into<Bytes>) -> Response<ApiBody> {
    Response::builder()
        .status(status)
        .header(hyper::header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Full::new(body.into()).boxed())
        .unwrap()
}

fn json_ok(bytes: Vec<u8>) -> Response<ApiBody> {
    Response::builder()
        .status(StatusCode::OK)
        .header(hyper::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(bytes)).boxed())
        .unwrap()
}

/// Short-circuits on length mismatch but is otherwise constant-time over the
/// compared bytes. Adequate for a static admin token.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}
