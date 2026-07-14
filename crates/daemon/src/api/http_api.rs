//! HTTP/JSON + SSE control API — the daemon's remote administrative channel.
//!
//! Serves the same `Command`→`Response` surface the local UDS server does, but
//! over HTTP so the glacier-ui client can drive it entirely from Luau
//! (`fetch`/`sse`). It reuses [`dispatch`] wholesale — only the transport
//! changes. By default it binds loopback and serves plain HTTP, meant to sit
//! behind the ingress proxy. Alternatively, setting `api.domain` in the config
//! makes this listener terminate TLS on its own port with an ACME-provisioned
//! certificate, so the GUI can reach it directly over HTTPS.
//!
//! - `POST /api/rpc` — body is a JSON-encoded [`Command`]; the reply is a
//!   JSON-encoded [`Response`](shared::Response). One endpoint covers every
//!   command; the client reshapes the (externally tagged) JSON in Luau.
//! - `GET /api/events` — Server-Sent Events. Emits a `snapshot` record with the
//!   full dashboard state every 2s (replacing the old client-side 2s poll) plus
//!   a `bus` record for each live [`Event`](shared::Event) from the event bus
//!   (logs, metrics, deploy progress). One connection replaces poll + stream.
//!   Each record is **self-describing**: its JSON `data` carries a `kind` field
//!   (`"snapshot"`/`"bus"`) because the SSE client (glacier-ui) discards the
//!   `event:` line and only sees `data:`, so the discriminator must live inside
//!   the payload.
//! - `GET /api/health` — liveness probe (`ok`).

use std::convert::Infallible;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Full, StreamBody};
use hyper::body::{Frame, Incoming};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use shared::{ApiConfig, Command, Event, Response as RpResponse};
use tracing::{error, info, warn};

use super::public_routes;
use super::routes::dispatch;
use super::AppState;
use crate::ingress::TlsManager;

/// Unified response body: both the buffered (`Full`) replies and the streaming
/// (`StreamBody`) SSE body box to this, so a single `service_fn` can return
/// either. Error is `Infallible` since neither body ever fails to produce data.
type ApiBody = BoxBody<Bytes, Infallible>;

/// Starts the API listener. Returns on bind failure; otherwise loops forever.
/// Intended to be `tokio::spawn`ed from `main`.
///
/// When `cfg.domain` is set (non-empty), the listener terminates TLS **on this
/// same port** using a Let's Encrypt certificate provisioned through `tls`
/// (auto-renewed by the ingress' ACME loop). Otherwise it serves plain HTTP,
/// as before. The port is always taken from `cfg.port` (config.toml `[api].port`).
pub async fn run(
    state: AppState,
    cfg: ApiConfig,
    tls: Option<Arc<TlsManager>>
) {
    // Safety guard: refuse a non-loopback bind without a token.
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
            error!(
                error = %e,
                addr = %addr,
                "API: falha ao bind, listener desabilitado"
            );
            return;
        }
    };

    // TLS opcional: se `api.domain` estiver definido, a própria porta da API
    // termina HTTPS com cert automático via ACME. Provisiona/valida o cert
    // ANTES de aceitar conexões (o desafio HTTP-01 exige a porta 80 acessível).
    let acceptor: Option<tokio_rustls::TlsAcceptor> =
        match cfg.domain.as_deref().filter(|d| !d.is_empty()) {
            Some(domain) => {
                let Some(tls) = tls else {
                    error!(
                        domain,
                        "API: api.domain configurado mas TlsManager indisponível — \
                         listener NÃO iniciado"
                    );
                    return;
                };
                if let Err(e) = tls.ensure_cert(domain).await {
                    warn!(
                        domain, error = %e,
                        "API: falha ao provisionar certificado TLS — o handshake HTTPS \
                         falhará até o cert existir (confira ACME habilitado + porta 80)"
                    );
                }
                info!(domain, "API HTTP/SSE: TLS ativo (cert automático via ACME)");
                Some(tls.tls_acceptor())
            }
            None => None,
        };

    let token = Arc::new(cfg.token.clone().filter(|s| !s.is_empty()));
    info!(
        addr = %addr,
        auth = token.is_some(),
        tls = acceptor.is_some(),
        "API HTTP/SSE: escutando"
    );

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
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            match acceptor {
                Some(acc) => match acc.accept(stream).await {
                    Ok(tls_stream) => serve_conn(
                        TokioIo::new(tls_stream), state, token, peer
                    ).await,
                    Err(e) => tracing::debug!(
                        peer = %peer, error = %e, "API: TLS handshake falhou"
                    ),
                },
                None => serve_conn(TokioIo::new(stream), state, token, peer).await,
            }
        });
    }
}

/// Serves one HTTP/1.1 connection over `io` (plain TCP or a TLS stream).
async fn serve_conn<I>(
    io: I,
    state: AppState,
    token: Arc<Option<String>>,
    peer: std::net::SocketAddr
)
where
    I: hyper::rt::Read + hyper::rt::Write + Unpin + 'static,
{
    let svc = service_fn(move |req| handle(req, state.clone(), token.clone()));
    if let Err(e) = hyper::server::conn::http1::Builder::new()
        .serve_connection(io, svc)
        .await
    {
        // Long-lived SSE connections end with an error when the client
        // drops — expected, so keep it at debug.
        tracing::debug!(peer = %peer, error = %e, "API: conexão encerrada");
    }
}

async fn handle(
    req: Request<Incoming>,
    state: AppState,
    token: Arc<Option<String>>,
) -> Result<Response<ApiBody>, Infallible> {
    // Rotas públicas — servidas ANTES do gate de Bearer porque cada uma tem a
    // sua própria autenticação (token de 192 bits na URL, no webhook; `state`
    // CSRF, no callback OAuth). São chamadas por terceiros (GitHub/Gitea/Docker
    // Hub, o navegador no fim do fluxo OAuth) que não têm o token da API.
    match (req.method(), req.uri().path()) {
        (m, p) if p.starts_with("/webhook/") => {
            let (method, path) = (m.clone(), p.to_owned());
            return Ok(boxed(public_routes::webhook(&method, &path, state).await));
        }
        (&Method::GET, "/oauth/gitea/callback") => {
            return Ok(boxed(public_routes::oauth_callback(req, state).await));
        }
        _ => {}
    }

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
        // SSE dedicado de logs de runtime de um serviço: `/api/services/<id>/logs`.
        (&Method::GET, p) if p.starts_with("/api/services/") && p.ends_with("/logs") => {
            // strip_prefix + strip_suffix (não aritmética de índices) para o caso
            // sem id (`/api/services/logs`) cair fora em vez de fatiar inválido.
            let id = p
                .strip_prefix("/api/services/")
                .and_then(|s| s.strip_suffix("/logs"))
                .unwrap_or("");
            if id.is_empty() {
                text(StatusCode::NOT_FOUND, "not found")
            } else {
                service_logs(state, id.to_string())
            }
        }
        // SSE dedicado de build logs de um deployment: `/api/deployments/<id>/build-logs`.
        (&Method::GET, p)
            if p.starts_with("/api/deployments/") && p.ends_with("/build-logs") =>
        {
            let id = p
                .strip_prefix("/api/deployments/")
                .and_then(|s| s.strip_suffix("/build-logs"))
                .unwrap_or("");
            if id.is_empty() {
                text(StatusCode::NOT_FOUND, "not found")
            } else {
                deployment_build_logs(state, id.to_string())
            }
        }
        // SSE dedicado de logs de UMA execução de job: `/api/jobs/runs/<id>/logs`.
        (&Method::GET, p)
            if p.starts_with("/api/jobs/runs/") && p.ends_with("/logs") =>
        {
            let id = p
                .strip_prefix("/api/jobs/runs/")
                .and_then(|s| s.strip_suffix("/logs"))
                .unwrap_or("");
            if id.is_empty() {
                text(StatusCode::NOT_FOUND, "not found")
            } else {
                job_run_logs(state, id.to_string())
            }
        }
        _ => text(StatusCode::NOT_FOUND, "not found"),
    })
}

/// `POST /api/rpc`: decode a `Command`, run it through `dispatch`, encode the
/// `Response` back as JSON.
async fn rpc(
    req: Request<Incoming>,
    state: AppState
) -> Response<ApiBody> {
    let body = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(_) => return text(
            StatusCode::BAD_REQUEST, "erro ao ler o corpo"
        ),
    };
    let cmd: Command = match serde_json::from_slice(&body) {
        Ok(c) => c,
        Err(e) => return text(
            StatusCode::BAD_REQUEST, format!("comando inválido: {e}")
        ),
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
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        let mut bus_rx = state.bus.subscribe();
        let mut ticker = tokio::time::interval(
            std::time::Duration::from_secs(2)
        );
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        // Snapshot inicial imediato, para o cliente pintar sem esperar o 1º tick.
        if tx.send(sse_frame("snapshot", &snapshot(&state).await)).await.is_err() {
            return;
        }

        // Firehose do dashboard: NÃO carrega logs de container (`LogLine`) nem de
        // build (`BuildLog`). Um serviço verborrágico (logando por request) emite
        // centenas de linhas/s; entregá-las aqui forçaria a janela PRINCIPAL a
        // reavaliar sua árvore inteira por linha (todas as janelas do iced::daemon
        // dividem UMA thread de UI), congelando tudo. Logs vão por endpoints
        // dedicados, consumidos só pelas janelas de log ISOLADAS (motores leves):
        //   • runtime → `/api/services/{id}/logs`         (ver `service_logs`)
        //   • build   → `/api/deployments/{id}/build-logs` (ver `deployment_build_logs`)
        // Aqui ficam só snapshot + eventos não-log (métricas, deploy, status),
        // todos de baixa frequência e enviados imediatamente.
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if tx.send(
                        sse_frame("snapshot", &snapshot(&state).await)
                    ).await.is_err() {
                        break;
                    }
                }
                r = bus_rx.recv() => match r {
                    Ok(ev) => {
                        // Logs (runtime/build/job) só pelos endpoints dedicados.
                        if matches!(
                            ev,
                            Event::LogLine { .. } | Event::BuildLog { .. } | Event::JobLogLine { .. }
                        ) {
                            continue;
                        }
                        // Self-describing: the SSE client only sees `data:` (the
                        // `event:` line is dropped), so tag the payload with
                        // `kind:"bus"` and nest the event under `event`.
                        let wrapped = serde_json::json!({ "kind": "bus", "event": ev });
                        if let Ok(js) = serde_json::to_string(&wrapped) {
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

    sse_response(rx)
}

/// `GET /api/services/{id}/logs`: SSE dedicado aos logs de container de UM
/// serviço. Existe para tirar o firehose de logs da janela principal (ver
/// `events`): um produtor filtra `Event::LogLine` do serviço no bus e os
/// entrega coalescidos em lotes `bus_batch` (mesma framing de `send_log_batch`),
/// consumidos pela janela de logs (motor Glacier leve). Não manda snapshot nem o
/// tail — a janela semeia o histórico inicial via `open_window({data})`.
fn service_logs(state: AppState, service_id: String) -> Response<ApiBody> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        let mut bus_rx = state.bus.subscribe();
        const LOG_BATCH_MAX: usize = 400;
        let mut batch: Vec<Event> = Vec::new();
        let mut batch_flush = tokio::time::interval(
            std::time::Duration::from_millis(80)
        );
        batch_flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = batch_flush.tick() => {
                    if !batch.is_empty()
                        && send_log_batch(&tx, std::mem::take(&mut batch)).await.is_err()
                    {
                        break;
                    }
                }
                r = bus_rx.recv() => match r {
                    Ok(ev) => {
                        let keep = matches!(
                            &ev,
                            Event::LogLine { service_id: sid, .. } if *sid == service_id
                        );
                        if keep {
                            batch.push(ev);
                            if batch.len() >= LOG_BATCH_MAX
                                && send_log_batch(&tx, std::mem::take(&mut batch)).await.is_err()
                            {
                                break;
                            }
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        }
    });

    sse_response(rx)
}

/// `GET /api/deployments/{id}/build-logs`: SSE dedicado à saída de `docker build`
/// de UM deployment. Gêmeo de `service_logs`, mas filtrando `Event::BuildLog` por
/// `deployment_id`; existe para tirar os build logs do firehose (ver `events`) e
/// os isolar na janela de build logs (motor leve). Não manda o histórico — a
/// janela o semeia via `GetBuildLogs` em `open_window({data})`.
fn deployment_build_logs(state: AppState, deployment_id: String) -> Response<ApiBody> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        let mut bus_rx = state.bus.subscribe();
        const LOG_BATCH_MAX: usize = 400;
        let mut batch: Vec<Event> = Vec::new();
        let mut batch_flush = tokio::time::interval(
            std::time::Duration::from_millis(80)
        );
        batch_flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = batch_flush.tick() => {
                    if !batch.is_empty()
                        && send_log_batch(&tx, std::mem::take(&mut batch)).await.is_err()
                    {
                        break;
                    }
                }
                r = bus_rx.recv() => match r {
                    Ok(ev) => {
                        let keep = matches!(
                            &ev,
                            Event::BuildLog { deployment_id: did, .. } if *did == deployment_id
                        );
                        if keep {
                            batch.push(ev);
                            if batch.len() >= LOG_BATCH_MAX
                                && send_log_batch(&tx, std::mem::take(&mut batch)).await.is_err()
                            {
                                break;
                            }
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        }
    });

    sse_response(rx)
}

/// `GET /api/jobs/runs/{id}/logs`: SSE dedicado à saída de UMA execução de job
/// one-shot. Gêmeo de `deployment_build_logs`, mas filtrando `Event::JobLogLine`
/// por `job_run_id`; não manda o histórico — a janela o semeia via
/// `GetJobLogs` em `open_window({data})`.
fn job_run_logs(state: AppState, job_run_id: String) -> Response<ApiBody> {
    let (tx, rx) = tokio::sync::mpsc::channel::<Bytes>(64);

    tokio::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        let mut bus_rx = state.bus.subscribe();
        const LOG_BATCH_MAX: usize = 400;
        let mut batch: Vec<Event> = Vec::new();
        let mut batch_flush = tokio::time::interval(
            std::time::Duration::from_millis(80)
        );
        batch_flush.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = batch_flush.tick() => {
                    if !batch.is_empty()
                        && send_log_batch(&tx, std::mem::take(&mut batch)).await.is_err()
                    {
                        break;
                    }
                }
                r = bus_rx.recv() => match r {
                    Ok(ev) => {
                        let keep = matches!(
                            &ev,
                            Event::JobLogLine { job_run_id: rid, .. } if *rid == job_run_id
                        );
                        if keep {
                            batch.push(ev);
                            if batch.len() >= LOG_BATCH_MAX
                                && send_log_batch(&tx, std::mem::take(&mut batch)).await.is_err()
                            {
                                break;
                            }
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => break,
                }
            }
        }
    });

    sse_response(rx)
}

/// Response body comum dos endpoints SSE (`events`/`service_logs`/
/// `deployment_build_logs`): drena o `rx` para o corpo `text/event-stream`, com
/// buffering de proxy desligado p/ os eventos saírem na hora através do ingress.
fn sse_response(mut rx: tokio::sync::mpsc::Receiver<Bytes>) -> Response<ApiBody> {
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
pub(crate) async fn snapshot(state: &AppState) -> String {
    use serde_json::{json, Map, Value};
    let mut obj = Map::new();
    // Self-describing record (see module docs): the SSE client only sees the
    // `data:` payload, so the `snapshot`/`bus` discriminator lives here.
    obj.insert("kind".into(), Value::String("snapshot".into()));

    if let RpResponse::DaemonStatus(d) =
        dispatch(
            state.clone(),
            Command::DaemonStatus
        ).await {
            obj.insert(
                "status".into(),
                serde_json::to_value(d).unwrap_or(Value::Null)
            );
    }
    if let RpResponse::DeploymentSummaries(list) =
        dispatch(
            state.clone(),
            Command::RecentDeployments { limit: 40 }
        ).await
    {
        obj.insert(
            "deployments".into(),
            serde_json::to_value(list)
                .unwrap_or(Value::Null)
        );
    }
    if let RpResponse::Projects(projects) =
        dispatch(
            state.clone(),
            Command::ProjectList
        ).await {
        // Uma única listagem do Docker indexa todos os containers por service_id
        // e por projeto Compose, para anexar a cada serviço os containers reais
        // em execução (id + nome + estado) — cobre réplicas, staging e Compose.
        let container_index =
            crate::docker::containers::index_containers(
                &state.docker.inner
            ).await;
        let mut services = Vec::new();
        for p in &projects {
            if let RpResponse::Services(svcs) =
                dispatch(
                    state.clone(),
                    Command::ServiceList { project_id: p.id.clone() }
                ).await
            {
                for s in svcs {
                    let conts = container_index.for_service(&s.id, &s.spec.name);
                    let mut sv = serde_json::to_value(&s)
                        .unwrap_or(Value::Null);
                    if let Value::Object(m) = &mut sv {
                        m.insert(
                            "containers".into(),
                            serde_json::to_value(conts)
                                .unwrap_or(Value::Null),
                        );
                    }
                    services.push(
                        json!({ "project_name": p.name, "service": sv })
                    );
                }
            }
        }

        obj.insert(
            "projects".into(),
            serde_json::to_value(&projects).unwrap_or(Value::Null)
        );

        obj.insert(
            "services".into(),
            Value::Array(services)
        );

    }
    if let RpResponse::DockerImages(list) =
        dispatch(state.clone(), Command::DockerImages).await {
            obj.insert("docker_images".into(),
            serde_json::to_value(list).unwrap_or(Value::Null)
        );
    }
    if let RpResponse::DockerVolumes(list) =
        dispatch(state.clone(), Command::DockerVolumes).await {
            obj.insert("docker_volumes".into(),
            serde_json::to_value(list).unwrap_or(Value::Null)
        );
    }
    if let RpResponse::DockerNetworks(list) =
        dispatch(state.clone(), Command::DockerNetworks).await {
            obj.insert("docker_networks".into(),
            serde_json::to_value(list).unwrap_or(Value::Null)
        );
    }
    if let RpResponse::DockerContainers(list) =
        dispatch(state.clone(), Command::DockerContainers).await {
            obj.insert("docker_containers".into(),
            serde_json::to_value(list).unwrap_or(Value::Null)
        );
    }
    if let RpResponse::JobSummaries(list) =
        dispatch(state.clone(), Command::JobListAll).await {
            obj.insert("jobs".into(),
            serde_json::to_value(list).unwrap_or(Value::Null)
        );
    }
    if let RpResponse::DeployEngineStatus(eng) =
        dispatch(state.clone(), Command::DeployEngineStatus).await
    {
        obj.insert("engine".into(), serde_json::to_value(eng).unwrap_or(Value::Null));
    }
    if let RpResponse::RegistryStatus(info) =
        dispatch(state.clone(), Command::RegistryStatus).await
    {
        obj.insert("registry_status".into(), serde_json::to_value(info).unwrap_or(Value::Null));
    }
    if let RpResponse::RegistryRepos(list) =
        dispatch(state.clone(), Command::RegistryRepoList).await
    {
        obj.insert("registry_repos".into(), serde_json::to_value(list).unwrap_or(Value::Null));
    }

    Value::Object(obj).to_string()
}

/// Formats one SSE record. `data` is single-line JSON (`serde_json::to_string`
/// never emits raw newlines), so a single `data:` line is safe.
fn sse_frame(event: &str, data: &str) -> Bytes {
    Bytes::from(format!("event: {event}\ndata: {data}\n\n"))
}

/// Envia um lote coalescido de eventos de log como uma única frame SSE
/// `bus_batch` (ver o produtor em [`events`]): `{ "kind":"bus_batch",
/// "events":[<Event>,…] }`. O cliente (Luau `on_state`) itera `events` chamando
/// `apply_bus` para cada um. `Err(())` só quando o canal fechou (cliente saiu),
/// sinalizando o produtor a encerrar; um lote que falha ao serializar é
/// descartado sem derrubar o stream.
async fn send_log_batch(
    tx: &tokio::sync::mpsc::Sender<Bytes>,
    batch: Vec<Event>,
) -> Result<(), ()> {
    let wrapped = serde_json::json!({ "kind": "bus_batch", "events": batch });
    match serde_json::to_string(&wrapped) {
        Ok(js) => tx.send(sse_frame("bus", &js)).await.map_err(|_| ()),
        Err(_) => Ok(()),
    }
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

/// As rotas públicas (`public_routes`) devolvem `Full<Bytes>`; o listener
/// unificado fala `ApiBody`. `Full` é infalível, então o rebox é direto.
fn boxed(resp: Response<Full<Bytes>>) -> Response<ApiBody> {
    resp.map(|body| body.boxed())
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
