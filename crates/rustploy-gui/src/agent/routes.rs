//! Servidor hyper da API de agente e os handlers de cada rota.
//!
//! Servidor **e** cliente são hyper (ver `client.rs`): o daemon do rustploy já
//! serve a própria API assim, e não faz sentido carregar um segundo framework
//! HTTP dentro do app de desktop só para servir sete rotas em loopback.
//!
//! Todas as rotas devolvem JSON, inclusive os erros — um agente não deveria
//! precisar distinguir "corpo de erro em texto" de "corpo de resposta em JSON"
//! no meio de um fluxo. O formato de erro é sempre
//! `{"error": {"code": "...", "message": "..."}}`.

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{AUTHORIZATION, CONTENT_TYPE};
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use serde_json::{json, Value};

use glacier_ui::ExternalSender;

use super::client::{response_error, response_kind, response_payload, Remote, RemoteError};
use super::session::Session;
use super::ui::ConnectOutcome;
use super::{actions, catalog, handoff, servers, ui, SharedSession};

/// Teto do corpo de uma requisição. Generoso porque um `ManifestApply` ou um
/// `ServiceUpdate` de fonte Compose carrega YAML de verdade (dezenas de KB), e
/// mesquinho o bastante para um cliente maluco não comer a RAM do app.
const MAX_BODY: usize = 32 * 1024 * 1024;

/// De quanto em quanto tempo o `wait` de um deploy reconsulta o daemon.
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Teto do `timeout_s` aceito em `POST /agent/deploys`.
const MAX_WAIT: Duration = Duration::from_secs(3600);

/// Estado compartilhado por todas as conexões.
struct Ctx {
    session: SharedSession,
    remote: Remote,
    /// Canal para injetar ações no motor da janela (glacier-ui 0.58.6+). É o
    /// que torna a GUI dirigível: sem ele, login/navegação só por clique.
    ui: ExternalSender,
    /// Token exigido no `Authorization` das rotas desta API (não o do daemon).
    token: String,
    addr: SocketAddr,
}

/// Erro já no formato em que ele sai pela rede.
struct ApiFail {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiFail {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self { status, code, message: message.into() }
    }

    /// A janela não está conectada a daemon nenhum. Vale um status próprio
    /// (503) e uma mensagem que diz o que fazer, porque é o erro que um agente
    /// mais vai encontrar: o app abriu, mas ninguém logou ainda.
    fn desconectado() -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "not_connected",
            "a janela do Rustploy não está conectada a nenhum daemon — \
             conecte-se na tela de login do app e tente de novo",
        )
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    fn into_response(self) -> Response<Full<Bytes>> {
        json_response(
            self.status,
            &json!({ "error": { "code": self.code, "message": self.message } }),
        )
    }
}

impl From<RemoteError> for ApiFail {
    fn from(e: RemoteError) -> Self {
        match e {
            // 502: quem falhou foi o salto daqui para o daemon, não o pedido do
            // agente — a distinção importa para ele saber se adianta repetir.
            RemoteError::Transport(msg) => {
                ApiFail::new(StatusCode::BAD_GATEWAY, "daemon_unreachable", msg)
            }
            RemoteError::Status { status: 401, .. } => ApiFail::new(
                StatusCode::BAD_GATEWAY,
                "daemon_unauthorized",
                "o daemon recusou o token da sessão da GUI (401) — \
                 reconecte na tela de login do app",
            ),
            RemoteError::Status { status, body } => ApiFail::new(
                StatusCode::BAD_GATEWAY,
                "daemon_http_error",
                format!("daemon respondeu HTTP {status}: {body}"),
            ),
            RemoteError::Decode(msg) => {
                ApiFail::new(StatusCode::BAD_GATEWAY, "daemon_bad_response", msg)
            }
        }
    }
}

// ── servidor ────────────────────────────────────────────────────────────────

/// Sobe o listener e serve até o processo morrer.
pub(super) async fn serve(
    addr: SocketAddr,
    session: SharedSession,
    ui: ExternalSender,
) -> Result<(), String> {
    let (listener, addr) = bind(addr).await?;

    let remote = Remote::new().map_err(|e| format!("cliente HTTP: {e}"))?;
    let token = handoff::generate_token();

    let ctx = Arc::new(Ctx { session, remote, ui, token, addr });

    if let Err(e) = handoff::write(addr, &ctx.token, remote_url(&ctx).as_deref()) {
        // Sem handoff a API sobe do mesmo jeito, mas ninguém a descobre — vale
        // um aviso alto, não um encerramento.
        eprintln!(
            "[agent-api] no ar em http://{addr}, mas falhou ao gravar {}: {e}",
            handoff::path().display()
        );
    } else {
        eprintln!(
            "[agent-api] no ar em http://{addr} — handoff em {}",
            handoff::path().display()
        );
    }

    tokio::spawn(watch_session(ctx.clone()));

    accept_loop(listener, ctx).await
}

/// O laço de aceitação, separado de [`serve`] para os testes poderem exercitar
/// o roteamento sem gerar token nem escrever o handoff no data dir do usuário.
async fn accept_loop(listener: tokio::net::TcpListener, ctx: Arc<Ctx>) -> Result<(), String> {
    loop {
        let (stream, _peer) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[agent-api] accept falhou: {e}");
                continue;
            }
        };
        let ctx = ctx.clone();
        tokio::spawn(async move {
            let svc = service_fn(move |req| handle(req, ctx.clone()));
            if let Err(e) = hyper::server::conn::http1::Builder::new()
                .serve_connection(TokioIo::new(stream), svc)
                .await
            {
                // Cliente que desiste no meio é rotina; nada a relatar.
                let _ = e;
            }
        });
    }
}

/// Tenta o endereço pedido; se a porta estiver ocupada (outro app, ou uma
/// instância anterior ainda encerrando), cai para uma porta efêmera no mesmo
/// IP. O handoff é que diz onde a API realmente ficou — por isso ele existe.
async fn bind(preferido: SocketAddr) -> Result<(tokio::net::TcpListener, SocketAddr), String> {
    match tokio::net::TcpListener::bind(preferido).await {
        Ok(l) => {
            let addr = l.local_addr().unwrap_or(preferido);
            Ok((l, addr))
        }
        Err(e) => {
            eprintln!("[agent-api] porta {preferido} indisponível ({e}) — usando porta efêmera");
            let alternativo = SocketAddr::new(preferido.ip(), 0);
            let l = tokio::net::TcpListener::bind(alternativo)
                .await
                .map_err(|e| format!("bind em {alternativo}: {e}"))?;
            let addr = l.local_addr().map_err(|e| format!("local_addr: {e}"))?;
            Ok((l, addr))
        }
    }
}

fn remote_url(ctx: &Ctx) -> Option<String> {
    ctx.session.get().map(|s| s.base_url)
}

/// Mantém o campo `remote_url`/`connected` do handoff em dia.
///
/// A sessão muda na thread da UI (login, logout, troca de servidor) e o handoff
/// é escrito aqui, na thread da API — um poll curto é o acoplamento mais barato
/// entre as duas, e não há nada a perder em detectar a mudança 2s depois.
async fn watch_session(ctx: Arc<Ctx>) {
    let mut ultimo = remote_url(&ctx);
    loop {
        tokio::time::sleep(POLL_INTERVAL).await;
        let atual = remote_url(&ctx);
        if atual != ultimo {
            let _ = handoff::write(ctx.addr, &ctx.token, atual.as_deref());
            ultimo = atual;
        }
    }
}

// ── roteamento ──────────────────────────────────────────────────────────────

async fn handle(req: Request<Incoming>, ctx: Arc<Ctx>) -> Result<Response<Full<Bytes>>, Infallible> {
    let metodo = req.method().clone();
    let caminho = req.uri().path().to_owned();
    let query = req.uri().query().unwrap_or_default().to_owned();

    // Liveness fica FORA do gate de token: serve para um agente saber se o app
    // está no ar antes de ter lido o handoff, e não revela nada — nem o
    // endereço do daemon remoto, que é dado do usuário.
    if metodo == Method::GET && caminho == "/agent/health" {
        return Ok(json_response(
            StatusCode::OK,
            &json!({
                "ok": true,
                "app": "rustploy-gui",
                "connected": ctx.session.get().is_some(),
            }),
        ));
    }

    if let Err(fail) = check_token(&req, &ctx.token) {
        return Ok(fail.into_response());
    }

    let resultado = match (&metodo, caminho.as_str()) {
        (&Method::GET, "/agent/schema") => Ok(json_response(StatusCode::OK, &catalog::schema())),
        (&Method::GET, "/agent/status") => status(&ctx).await,
        (&Method::GET, "/agent/services") => services(&ctx).await,
        (&Method::GET, "/agent/deploys") => deploys(&ctx, &query).await,
        (&Method::POST, "/agent/deploys") => start_deploy(&ctx, req).await,
        (&Method::POST, "/agent/rpc") => rpc_passthrough(&ctx, req).await,

        // ── controle da própria janela (ver `ui.rs`) ──────────────────────
        (&Method::GET, "/agent/servers") => Ok(json_response(StatusCode::OK, &servers::as_json())),
        (&Method::POST, "/agent/connect") => connect(&ctx, req).await,
        (&Method::POST, "/agent/disconnect") => disconnect(&ctx),
        (&Method::GET, "/agent/ui") => Ok(ui_state(&ctx, &query)),
        (&Method::GET, "/agent/ui/actions") => {
            Ok(json_response(StatusCode::OK, &actions::index()))
        }
        (&Method::POST, "/agent/ui/action") => ui_action(&ctx, req).await,
        (&Method::POST, "/agent/ui/context") => ui_context(&ctx, req).await,

        (&Method::POST, p) => match archive_path(p) {
            Some(id) => upload_archive(&ctx, &id, req).await,
            None => Err(not_found(&metodo, p)),
        },
        (&Method::GET, p) => match build_log_path(p) {
            Some(id) => build_logs(&ctx, &id, &query).await,
            None => Err(not_found(&metodo, p)),
        },
        (m, p) => Err(not_found(m, p)),
    };

    Ok(match resultado {
        Ok(resp) => resp,
        Err(fail) => fail.into_response(),
    })
}

fn not_found(metodo: &Method, caminho: &str) -> ApiFail {
    ApiFail::new(
        StatusCode::NOT_FOUND,
        "no_such_route",
        format!("{metodo} {caminho} não existe — veja GET /agent/schema"),
    )
}

/// `/agent/deploys/<id>/logs` → `<id>`.
fn build_log_path(p: &str) -> Option<String> {
    let resto = p.strip_prefix("/agent/deploys/")?.strip_suffix("/logs")?;
    if resto.is_empty() || resto.contains('/') {
        return None;
    }
    Some(resto.to_owned())
}

/// `/agent/services/<id>/archive` → `<id>`.
fn archive_path(p: &str) -> Option<String> {
    let resto = p.strip_prefix("/agent/services/")?.strip_suffix("/archive")?;
    if resto.is_empty() || resto.contains('/') {
        return None;
    }
    Some(resto.to_owned())
}

/// Bearer da API de agente (não o do daemon). Comparação em tempo constante:
/// o token é curto e local, mas comparar segredo com `==` é o tipo de detalhe
/// que não vale economizar.
fn check_token(req: &Request<Incoming>, esperado: &str) -> Result<(), ApiFail> {
    let recebido = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .unwrap_or_default();

    if ct_eq(recebido.as_bytes(), esperado.as_bytes()) {
        Ok(())
    } else {
        Err(ApiFail::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            format!(
                "mande Authorization: Bearer <token>. O token desta execução está em {}",
                handoff::path().display()
            ),
        ))
    }
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

// ── handlers ────────────────────────────────────────────────────────────────

/// Executa um `Command` no daemon e já converte `Response::Err` em falha.
async fn rpc(ctx: &Ctx, cmd: Value) -> Result<Value, ApiFail> {
    let session: Session = ctx.session.get().ok_or_else(ApiFail::desconectado)?;
    let resposta = ctx.remote.rpc(&session, &cmd).await?;

    if let Some((code, message)) = response_error(&resposta) {
        return Err(ApiFail::new(
            StatusCode::BAD_REQUEST,
            "daemon_command_error",
            format!("{code}: {message}"),
        ));
    }

    Ok(resposta)
}

/// Como [`rpc`], mas exige que a resposta seja a variante esperada e devolve o
/// payload dela. Uma variante inesperada é bug de protocolo, não do agente.
async fn rpc_expect(ctx: &Ctx, cmd: Value, variante: &str) -> Result<Value, ApiFail> {
    let resposta = rpc(ctx, cmd).await?;

    match response_payload(&resposta) {
        Some(p) if response_kind(&resposta) == Some(variante) => Ok(p.clone()),
        _ => Err(ApiFail::new(
            StatusCode::BAD_GATEWAY,
            "unexpected_response",
            format!(
                "esperava Response::{variante}, veio {}",
                response_kind(&resposta).unwrap_or("algo indecifrável")
            ),
        )),
    }
}

/// `GET /agent/status` — a janela, o daemon e a fila de deploys num lugar só.
async fn status(ctx: &Ctx) -> Result<Response<Full<Bytes>>, ApiFail> {
    let session = ctx.session.get().ok_or_else(ApiFail::desconectado)?;
    let daemon = rpc_expect(ctx, json!("DaemonStatus"), "DaemonStatus").await?;
    let engine = rpc_expect(ctx, json!("DeployEngineStatus"), "DeployEngineStatus").await?;

    Ok(json_response(
        StatusCode::OK,
        &json!({
            "connected": true,
            "remote_url": session.base_url,
            "daemon": daemon,
            "deploy_engine": engine,
        }),
    ))
}

/// `GET /agent/services` — índice achatado projeto→serviço.
///
/// Existe porque o caminho cru para "qual é o id do serviço chamado X?" é
/// `ProjectList` seguido de um `ServiceList` por projeto, e a alternativa de uma
/// chamada só (`Snapshot`) devolve o dashboard inteiro — Docker, jobs, registry,
/// métricas. Aqui vai só o que identifica um serviço.
async fn services(ctx: &Ctx) -> Result<Response<Full<Bytes>>, ApiFail> {
    let snapshot = snapshot(ctx).await?;
    let lista = service_index(&snapshot);

    Ok(json_response(
        StatusCode::OK,
        &json!({ "count": lista.len(), "services": lista }),
    ))
}

/// `Snapshot` devolve `Response::Snapshot(String)` — JSON **dentro** de uma
/// string, não um objeto. Este helper desembrulha as duas camadas.
async fn snapshot(ctx: &Ctx) -> Result<Value, ApiFail> {
    let cru = rpc_expect(ctx, json!("Snapshot"), "Snapshot").await?;
    let texto = cru.as_str().ok_or_else(|| {
        ApiFail::new(
            StatusCode::BAD_GATEWAY,
            "unexpected_response",
            "Response::Snapshot não veio como string",
        )
    })?;

    serde_json::from_str(texto).map_err(|e| {
        ApiFail::new(
            StatusCode::BAD_GATEWAY,
            "unexpected_response",
            format!("snapshot não é JSON válido: {e}"),
        )
    })
}

/// `GET /agent/deploys` — últimos deploys com o desfecho já resolvido.
async fn deploys(ctx: &Ctx, query: &str) -> Result<Response<Full<Bytes>>, ApiFail> {
    let limit = num_param(query, "limit").unwrap_or(20).clamp(1, 200);

    let sumarios = rpc_expect(
        ctx,
        json!({ "RecentDeployments": { "limit": limit } }),
        "DeploymentSummaries",
    )
    .await?;

    let lista: Vec<Value> = sumarios
        .as_array()
        .map(|a| a.iter().map(compact_summary).collect())
        .unwrap_or_default();

    Ok(json_response(
        StatusCode::OK,
        &json!({ "count": lista.len(), "deployments": lista }),
    ))
}

/// `POST /agent/deploys` — dispara um deploy e, com `wait`, só responde quando
/// ele terminou.
///
/// É a rota que motivou este módulo. Sem ela, "o deploy funcionou?" custa ao
/// agente um `DeployStart`, um laço de `DeployHistory` filtrando por id, um
/// `GetBuildLogs` inteiro e o conhecimento de que a causa da falha mora no
/// `states_log` e não no estado — que é exatamente o conhecimento que ninguém
/// tem na primeira vez.
async fn start_deploy(ctx: &Ctx, req: Request<Incoming>) -> Result<Response<Full<Bytes>>, ApiFail> {
    let corpo = read_json(req).await?;

    let service_id = resolve_service(ctx, &corpo).await?;
    let esperar = corpo.get("wait").and_then(Value::as_bool).unwrap_or(true);
    let log_tail = corpo
        .get("log_tail")
        .and_then(Value::as_u64)
        .unwrap_or(40)
        .clamp(0, 500) as usize;
    let limite = corpo
        .get("timeout_s")
        .and_then(Value::as_u64)
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(900))
        .min(MAX_WAIT);

    let dep = rpc_expect(
        ctx,
        json!({ "DeployStart": { "service_id": service_id } }),
        "Deployment",
    )
    .await?;

    let deployment_id = dep
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();

    if !esperar {
        return Ok(json_response(
            StatusCode::ACCEPTED,
            &json!({
                "deployment_id": deployment_id,
                "service_id": service_id,
                "state": dep.get("state").cloned().unwrap_or(Value::Null),
                "waited": false,
                "hint": "acompanhe com GET /agent/deploys ou \
                         GET /agent/deploys/<deployment_id>/logs?after=<n>",
            }),
        ));
    }

    let (final_dep, expirou) =
        wait_for_outcome(ctx, &service_id, &deployment_id, limite).await?;

    let (log, total) = if log_tail > 0 {
        let linhas = build_log_lines(ctx, &deployment_id).await.unwrap_or_default();
        let total = linhas.len();
        let inicio = total.saturating_sub(log_tail);
        (linhas[inicio..].to_vec(), total)
    } else {
        (Vec::new(), 0)
    };

    let mut out = compact_deployment(&final_dep);
    if let Value::Object(m) = &mut out {
        m.insert("waited".into(), json!(true));
        m.insert("timed_out".into(), json!(expirou));
        m.insert("log_tail".into(), json!(log));
        m.insert("log_cursor".into(), json!(total));
    }

    Ok(json_response(StatusCode::OK, &out))
}

/// Descobre o `service_id` do corpo: aceita o id direto ou o nome do serviço.
///
/// Aceitar nome é o que faz a rota utilizável de cabeça — o nome é o que o
/// usuário diz ("sobe o stand-imob"), o ULID não.
async fn resolve_service(ctx: &Ctx, corpo: &Value) -> Result<String, ApiFail> {
    if let Some(id) = corpo.get("service_id").and_then(Value::as_str) {
        let id = id.trim();
        if !id.is_empty() {
            return Ok(id.to_owned());
        }
    }

    let nome = corpo
        .get("service")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ApiFail::bad_request("informe \"service_id\" ou \"service\" (nome)"))?;

    let indice = service_index(&snapshot(ctx).await?);
    let casos: Vec<&Value> = indice
        .iter()
        .filter(|s| {
            s.get("name")
                .and_then(Value::as_str)
                .map(|n| n.eq_ignore_ascii_case(nome))
                .unwrap_or(false)
        })
        .collect();

    match casos.as_slice() {
        [um] => Ok(um
            .get("service_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()),
        [] => Err(ApiFail::new(
            StatusCode::NOT_FOUND,
            "no_such_service",
            format!("nenhum serviço chamado {nome:?} — veja GET /agent/services"),
        )),
        // Nome de serviço é único por projeto, não globalmente: com ambiguidade
        // a rota não escolhe por conta própria.
        varios => Err(ApiFail::bad_request(format!(
            "{:?} existe em {} projetos — mande \"service_id\". Candidatos: {}",
            nome,
            varios.len(),
            varios
                .iter()
                .filter_map(|s| {
                    Some(format!(
                        "{}={}",
                        s.get("project")?.as_str()?,
                        s.get("service_id")?.as_str()?
                    ))
                })
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Poll até o deployment chegar a um estado terminal (ou o prazo acabar).
///
/// Poll, e não SSE: manter uma conexão de eventos aberta aqui significaria
/// consumir e reemitir o firehose do daemon só para observar um id. O deploy
/// mais rápido leva segundos; 2s de granularidade não custam nada.
async fn wait_for_outcome(
    ctx: &Ctx,
    service_id: &str,
    deployment_id: &str,
    limite: Duration,
) -> Result<(Value, bool), ApiFail> {
    let comeco = Instant::now();
    let mut ultimo: Option<Value> = None;

    loop {
        let historico = rpc_expect(
            ctx,
            json!({ "DeployHistory": { "service_id": service_id, "limit": 10 } }),
            "Deployments",
        )
        .await?;

        let atual = historico.as_array().and_then(|a| {
            a.iter()
                .find(|d| d.get("id").and_then(Value::as_str) == Some(deployment_id))
                .cloned()
        });

        if let Some(dep) = atual {
            if is_terminal(&dep) {
                return Ok((dep, false));
            }
            ultimo = Some(dep);
        }

        if comeco.elapsed() >= limite {
            // Expirou: devolve o último estado conhecido em vez de erro seco —
            // "ainda em BuildingImage depois de 15 min" é informação útil, e o
            // agente decide se espera mais ou investiga.
            let dep = ultimo.unwrap_or_else(|| {
                json!({ "id": deployment_id, "service_id": service_id, "state": "Unknown" })
            });
            return Ok((dep, true));
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// `GET /agent/deploys/<id>/logs` — build log com cursor.
///
/// O daemon só sabe devolver o log inteiro (`GetBuildLogs` não tem cursor), e um
/// build de verdade passa de mil linhas. A fatia acontece aqui: o tráfego caro
/// é o desta ponte para o agente, não o da ponte para o daemon na mesma sessão.
/// `after` é o índice da última linha já vista — a tabela do daemon só recebe
/// append e é ordenada por timestamp, então o índice é um cursor estável.
async fn build_logs(
    ctx: &Ctx,
    deployment_id: &str,
    query: &str,
) -> Result<Response<Full<Bytes>>, ApiFail> {
    let linhas = build_log_lines(ctx, deployment_id).await?;
    let total = linhas.len();

    let after = num_param(query, "after").unwrap_or(0).min(total);
    let limit = num_param(query, "limit").unwrap_or(500).clamp(1, 5000);
    let fim = (after + limit).min(total);

    Ok(json_response(
        StatusCode::OK,
        &json!({
            "deployment_id": deployment_id,
            "total": total,
            "after": after,
            "next_after": fim,
            "has_more": fim < total,
            "lines": &linhas[after..fim],
        }),
    ))
}

/// Texto de cada linha do build log, na ordem em que foi gravada.
async fn build_log_lines(ctx: &Ctx, deployment_id: &str) -> Result<Vec<String>, ApiFail> {
    let bruto = rpc_expect(
        ctx,
        json!({ "GetBuildLogs": { "deployment_id": deployment_id } }),
        "BuildLogs",
    )
    .await?;

    Ok(bruto
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|l| l.get("line").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default())
}

/// `POST /agent/rpc` — qualquer `Command` do protocolo, sem tradução.
///
/// A válvula de escape que mantém as rotas de conveniência honestas: elas
/// existem para os caminhos frequentes, não para virarem a única porta. Tudo o
/// que a GUI faz, um agente faz por aqui.
async fn rpc_passthrough(
    ctx: &Ctx,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, ApiFail> {
    let cmd = read_json(req).await?;
    let resposta = rpc(ctx, cmd).await?;
    Ok(json_response(StatusCode::OK, &resposta))
}

// ── controle da janela ──────────────────────────────────────────────────────

/// `POST /agent/connect` — entra na sessão pela própria tela de login.
///
/// Era o último ponto cego de verdade: sem isto, a ponte só servia depois que
/// um humano tivesse clicado Connect, e um agente numa máquina sem ninguém na
/// frente ficava preso no 503.
///
/// O `token` é opcional: omitido, a ponte busca o salvo para aquela URL
/// (`servers.rs`), de modo que o segredo nunca precisa atravessar a rede.
async fn connect(ctx: &Ctx, req: Request<Incoming>) -> Result<Response<Full<Bytes>>, ApiFail> {
    let corpo = read_json(req).await?;

    let url = corpo
        .get("url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .ok_or_else(|| ApiFail::bad_request("informe \"url\" (ex.: https://rustploy.exemplo.com)"))?;

    let token = corpo
        .get("token")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .or_else(|| servers::token_for(url));

    match ui::connect(&ctx.ui, &ctx.session, url, token.as_deref()).await {
        ConnectOutcome::Conectado { remote_url } => Ok(json_response(
            StatusCode::OK,
            &json!({ "connected": true, "remote_url": remote_url }),
        )),
        // 502: quem recusou foi o daemon do outro lado (token errado, host
        // inalcançável), não o pedido do agente.
        ConnectOutcome::Recusado { motivo } => Err(ApiFail::new(
            StatusCode::BAD_GATEWAY,
            "connect_refused",
            motivo,
        )),
        ConnectOutcome::Timeout => Err(ApiFail::new(
            StatusCode::GATEWAY_TIMEOUT,
            "connect_timeout",
            "a janela não concluiu o login no prazo — o daemon pode estar \
             inalcançável, ou a janela ocupada num diálogo modal",
        )),
    }
}

/// `POST /agent/disconnect` — sai da sessão.
fn disconnect(ctx: &Ctx) -> Result<Response<Full<Bytes>>, ApiFail> {
    ui::disconnect(&ctx.ui);
    // Sem esperar: `disconnect()` na Luau é síncrono e não tem como falhar, e a
    // consequência (sessão sumindo) fica visível em `GET /agent/ui`.
    Ok(json_response(
        StatusCode::ACCEPTED,
        &json!({ "ok": true, "hint": "confirme com GET /agent/ui" }),
    ))
}

/// `GET /agent/ui` — o que a janela está mostrando agora.
fn ui_state(ctx: &Ctx, query: &str) -> Response<Full<Bytes>> {
    if let Some(pedidas) = str_param(query, "keys") {
        return json_response(StatusCode::OK, &ui::keys(&ctx.session, &pedidas));
    }
    if str_param(query, "all").is_some() {
        return json_response(StatusCode::OK, &ui::all_keys(&ctx.session));
    }
    json_response(StatusCode::OK, &ui::state(&ctx.session))
}

/// `POST /agent/ui/action` — dispara qualquer ação da UI pelo nome.
///
/// A chave-mestra: todo botão, aba e formulário da GUI é uma função Luau global
/// (`views/scripts/handlers/*.luau`), e este endpoint chama qualquer uma delas
/// pelo mesmo caminho de um clique. Cobre as ~154 ações existentes e as futuras
/// sem precisar de uma lista aqui — que envelheceria na primeira tela nova.
async fn ui_action(ctx: &Ctx, req: Request<Incoming>) -> Result<Response<Full<Bytes>>, ApiFail> {
    let corpo = read_json(req).await?;

    let acao = corpo
        .get("action")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|a| !a.is_empty())
        .ok_or_else(|| ApiFail::bad_request("informe \"action\" (o nome da função Luau)"))?;

    // Com valor = `onChange` de um campo; sem valor = clique de botão. A
    // distinção é a mesma que os templates fazem, então a ação recebe
    // exatamente o que receberia da UI.
    let enviado = match corpo.get("value").and_then(Value::as_str) {
        Some(v) => ctx.ui.action(acao, v),
        None => ctx.ui.click(acao),
    };

    if !enviado {
        return Err(ApiFail::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "ui_gone",
            "a janela não está mais recebendo ações (o app encerrou?)",
        ));
    }

    // 202, não 200: a ação foi ENTREGUE ao motor, e o efeito dela é assíncrono
    // (várias fazem RPC). Prometer que "deu certo" aqui seria mentira.
    Ok(json_response(
        StatusCode::ACCEPTED,
        &json!({
            "dispatched": acao,
            "hint": "o efeito é assíncrono — confirme em GET /agent/ui"
        }),
    ))
}

/// `POST /agent/ui/context` — escreve chaves no contexto da janela.
///
/// O par de baixo nível do `ui/action`: preenche campo de formulário, marca
/// seleção, muda de aba. Útil quando a ação que você quer disparar espera algo
/// já escrito no contexto.
async fn ui_context(ctx: &Ctx, req: Request<Incoming>) -> Result<Response<Full<Bytes>>, ApiFail> {
    let corpo = read_json(req).await?;

    let obj = corpo
        .as_object()
        .ok_or_else(|| ApiFail::bad_request("o corpo deve ser um objeto de pares chave→valor"))?;

    if obj.is_empty() {
        return Err(ApiFail::bad_request("nenhuma chave informada"));
    }

    let pares: Vec<(String, String)> = obj
        .iter()
        .map(|(k, v)| {
            // Número/booleano viram texto: o contexto do motor é sempre
            // chave→string, e recusar por tipo seria pedantismo inútil.
            let texto = match v {
                Value::String(s) => s.clone(),
                outro => outro.to_string(),
            };
            (k.clone(), texto)
        })
        .collect();

    let nomes: Vec<&String> = pares.iter().map(|(k, _)| k).collect();
    if !ctx.ui.patch(pares.clone()) {
        return Err(ApiFail::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "ui_gone",
            "a janela não está mais recebendo ações (o app encerrou?)",
        ));
    }

    Ok(json_response(
        StatusCode::ACCEPTED,
        &json!({ "patched": nomes }),
    ))
}

/// `POST /agent/services/<id>/archive` — sobe um zip local para o serviço.
///
/// Recebe o CAMINHO do arquivo, não os bytes: quem chama está na mesma máquina
/// (a ponte é loopback), e mandar dezenas de MB em base64 por HTTP para depois
/// a ponte remontar seria custo puro. O corpo é `{"path": "/caminho/app.zip"}`.
async fn upload_archive(
    ctx: &Ctx,
    service_id: &str,
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, ApiFail> {
    let corpo = read_json(req).await?;

    let caminho = corpo
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .ok_or_else(|| ApiFail::bad_request("informe \"path\" (caminho local do .zip)"))?;

    let bytes = std::fs::read(caminho)
        .map_err(|e| ApiFail::bad_request(format!("não consegui ler {caminho}: {e}")))?;

    let nome = std::path::Path::new(caminho)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "upload.zip".into());

    let session: Session = ctx.session.get().ok_or_else(ApiFail::desconectado)?;
    let tamanho = bytes.len();
    let resposta = ctx
        .remote
        .upload_archive(&session, service_id, &nome, bytes)
        .await?;

    if let Some((code, message)) = response_error(&resposta) {
        return Err(ApiFail::new(
            StatusCode::BAD_REQUEST,
            "daemon_command_error",
            format!("{code}: {message}"),
        ));
    }

    Ok(json_response(
        StatusCode::OK,
        &json!({
            "service_id": service_id,
            "filename": nome,
            "bytes": tamanho,
            "daemon_response": resposta,
        }),
    ))
}

// ── moldagem das respostas do daemon ────────────────────────────────────────

/// `DeploymentSummary` → linha compacta com o desfecho resolvido.
fn compact_summary(sum: &Value) -> Value {
    let dep = sum.get("deployment").unwrap_or(&Value::Null);
    let mut out = compact_deployment(dep);
    if let Value::Object(m) = &mut out {
        if let Some(n) = sum.get("service_name") {
            m.insert("service".into(), n.clone());
        }
        if let Some(p) = sum.get("project_name") {
            m.insert("project".into(), p.clone());
        }
    }
    out
}

/// `Deployment` → o que interessa a quem só quer saber como acabou.
fn compact_deployment(dep: &Value) -> Value {
    let estado = dep.get("state").and_then(Value::as_str).unwrap_or("Unknown");

    json!({
        "deployment_id": dep.get("id").cloned().unwrap_or(Value::Null),
        "service_id": dep.get("service_id").cloned().unwrap_or(Value::Null),
        "image": dep.get("image").cloned().unwrap_or(Value::Null),
        "state": estado,
        "ok": outcome_ok(estado),
        "error": failure_reason(dep),
        "started_at": dep.get("started_at").cloned().unwrap_or(Value::Null),
        "finished_at": dep.get("finished_at").cloned().unwrap_or(Value::Null),
    })
}

/// `true` = no ar, `false` = falhou, `null` = ainda não decidiu.
///
/// `Stopped`/`Pruning` são terminais sem serem desfecho: o primeiro é o serviço
/// derrubado de propósito, o segundo é um deployment antigo que outro mais novo
/// substituiu. Chamar qualquer um dos dois de "falha" seria mentira.
fn outcome_ok(estado: &str) -> Value {
    match estado {
        "Live" => json!(true),
        "Failed" => json!(false),
        _ => Value::Null,
    }
}

fn is_terminal(dep: &Value) -> bool {
    matches!(
        dep.get("state").and_then(Value::as_str),
        Some("Live" | "Stopped" | "Failed" | "Pruning")
    )
}

/// A causa da falha, tirada do `states_log`.
///
/// É onde o daemon a grava: a transição que ENTROU em `RollingBack` carrega a
/// mensagem do step que quebrou (o texto do `docker build`, o healthcheck que
/// não passou…). Mensagens de outras transições são ignoradas de propósito —
/// `Pruning` traz "superseded by newer deployment", que não é erro nenhum.
fn failure_reason(dep: &Value) -> Value {
    let Some(log) = dep.get("states_log").and_then(Value::as_array) else {
        return Value::Null;
    };

    log.iter()
        .rev()
        .find(|t| {
            matches!(
                t.get("to").and_then(Value::as_str),
                Some("RollingBack" | "Failed")
            ) && t.get("message").map(|m| !m.is_null()).unwrap_or(false)
        })
        .and_then(|t| t.get("message").cloned())
        .unwrap_or(Value::Null)
}

/// Achata o snapshot no índice de serviços que um agente precisa para agir:
/// id, nome, projeto, status e de onde a imagem vem.
fn service_index(snapshot: &Value) -> Vec<Value> {
    let Some(lista) = snapshot.get("services").and_then(Value::as_array) else {
        return Vec::new();
    };

    lista
        .iter()
        .filter_map(|entrada| {
            let svc = entrada.get("service")?;
            let spec = svc.get("spec")?;

            Some(json!({
                "service_id": svc.get("id").cloned().unwrap_or(Value::Null),
                "name": spec.get("name").cloned().unwrap_or(Value::Null),
                "project": entrada.get("project_name").cloned().unwrap_or(Value::Null),
                "project_id": spec.get("project_id").cloned().unwrap_or(Value::Null),
                "status": status_label(svc.get("status")),
                "source": source_label(spec.get("source")),
                "port": spec.get("port").cloned().unwrap_or(Value::Null),
            }))
        })
        .collect()
}

/// `ServiceStatus` é externally-tagged e só a variante `Error` tem campo — vira
/// `"Running"` ou `"Error: <causa>"`. Depois da correção do log de deploy essa
/// causa é o motivo real da falha, não mais a string fixa "deploy failed".
fn status_label(status: Option<&Value>) -> Value {
    match status {
        Some(Value::String(s)) => json!(s),
        Some(Value::Object(m)) if m.len() == 1 => {
            let (k, v) = m.iter().next().unwrap();
            match v.as_str() {
                Some(detalhe) if !detalhe.is_empty() => json!(format!("{k}: {detalhe}")),
                _ => json!(k),
            }
        }
        _ => Value::Null,
    }
}

/// `ServiceSource` achatado no que identifica a origem, sem arrastar o
/// `compose.content` inteiro (dezenas de KB) para dentro de uma listagem.
fn source_label(source: Option<&Value>) -> Value {
    match source {
        Some(Value::Object(m)) if m.len() == 1 => {
            let (kind, v) = m.iter().next().unwrap();
            match kind.as_str() {
                "Registry" => json!({
                    "kind": "Registry",
                    "image": v.get("image").cloned().unwrap_or(Value::Null),
                }),
                "Git" => json!({
                    "kind": "Git",
                    "url": v.get("url").cloned().unwrap_or(Value::Null),
                    "branch": v.get("branch").cloned().unwrap_or(Value::Null),
                    "dockerfile_path": v.get("dockerfile_path").cloned().unwrap_or(Value::Null),
                    "build_context": v.get("build_context").cloned().unwrap_or(Value::Null),
                }),
                "Archive" => json!({
                    "kind": "Archive",
                    "dockerfile_path": v.get("dockerfile_path").cloned().unwrap_or(Value::Null),
                    "original_filename": v.get("original_filename").cloned().unwrap_or(Value::Null),
                }),
                outro => json!({ "kind": outro }),
            }
        }
        Some(Value::String(s)) => json!({ "kind": s }),
        _ => Value::Null,
    }
}

// ── utilidades de HTTP ──────────────────────────────────────────────────────

async fn read_json(req: Request<Incoming>) -> Result<Value, ApiFail> {
    let bytes = req
        .into_body()
        .collect()
        .await
        .map_err(|e| ApiFail::bad_request(format!("corpo ilegível: {e}")))?
        .to_bytes();

    if bytes.len() > MAX_BODY {
        return Err(ApiFail::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            "body_too_large",
            format!("corpo acima de {} MB", MAX_BODY / (1024 * 1024)),
        ));
    }
    if bytes.is_empty() {
        return Ok(Value::Object(Default::default()));
    }

    serde_json::from_slice(&bytes)
        .map_err(|e| ApiFail::bad_request(format!("corpo não é JSON válido: {e}")))
}

fn json_response(status: StatusCode, body: &Value) -> Response<Full<Bytes>> {
    let texto = serde_json::to_string_pretty(body).unwrap_or_else(|_| "{}".into());
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "application/json; charset=utf-8")
        .body(Full::new(Bytes::from(texto)))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::from_static(b"{}"))))
}

/// Parâmetro numérico da query string. Sem percent-decoding porque nenhum
/// parâmetro numérico precisa — os campos que aceitam texto livre (nome de
/// serviço) vão no corpo, justamente para não dependerem disso.
fn num_param(query: &str, chave: &str) -> Option<usize> {
    query.split('&').find_map(|par| {
        let (k, v) = par.split_once('=')?;
        (k == chave).then(|| v.parse().ok())?
    })
}

/// Parâmetro textual da query string. Sem percent-decoding: os únicos usos são
/// listas de nomes de chave (`keys=screen,view`) e flags (`all=1`), nenhum dos
/// quais precisa de escape.
fn str_param(query: &str, chave: &str) -> Option<String> {
    query.split('&').find_map(|par| {
        let (k, v) = par.split_once('=')?;
        (k == chave).then(|| v.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rota_de_archive_extrai_o_id() {
        assert_eq!(
            archive_path("/agent/services/svc_01H/archive").as_deref(),
            Some("svc_01H")
        );
        assert_eq!(archive_path("/agent/services//archive"), None);
        assert_eq!(archive_path("/agent/services/a/b/archive"), None);
        assert_eq!(archive_path("/agent/deploys/x/logs"), None);
    }

    #[test]
    fn parametro_textual_da_query() {
        assert_eq!(str_param("keys=screen,view", "keys").as_deref(), Some("screen,view"));
        assert_eq!(str_param("all=1", "all").as_deref(), Some("1"));
        assert_eq!(str_param("keys=x", "all"), None);
    }

    #[test]
    fn rota_de_log_extrai_o_id() {
        assert_eq!(
            build_log_path("/agent/deploys/dep_01H/logs").as_deref(),
            Some("dep_01H")
        );
        assert_eq!(build_log_path("/agent/deploys//logs"), None);
        assert_eq!(build_log_path("/agent/deploys/a/b/logs"), None);
        assert_eq!(build_log_path("/agent/deploys"), None);
    }

    #[test]
    fn comparacao_de_token_rejeita_tamanho_e_conteudo() {
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"ab"));
    }

    #[test]
    fn parametro_numerico_da_query() {
        assert_eq!(num_param("after=10&limit=5", "after"), Some(10));
        assert_eq!(num_param("after=10&limit=5", "limit"), Some(5));
        assert_eq!(num_param("after=nao&limit=5", "after"), None);
        assert_eq!(num_param("", "after"), None);
    }

    /// O desfecho: só `Live` e `Failed` decidem. Um deployment substituído por
    /// outro mais novo (`Pruning`) não é falha.
    #[test]
    fn desfecho_so_e_decidido_por_live_ou_failed() {
        assert_eq!(outcome_ok("Live"), json!(true));
        assert_eq!(outcome_ok("Failed"), json!(false));
        assert_eq!(outcome_ok("Pruning"), Value::Null);
        assert_eq!(outcome_ok("Stopped"), Value::Null);
        assert_eq!(outcome_ok("BuildingImage"), Value::Null);
    }

    /// A regressão que este módulo inteiro serve: a causa da falha sai do
    /// `states_log`, e não se deixa confundir pela mensagem de "superseded".
    #[test]
    fn causa_da_falha_vem_da_transicao_para_rollingback() {
        let dep = json!({
            "id": "dep_1",
            "state": "Failed",
            "states_log": [
                { "from": "Pending", "to": "BuildingImage", "message": null },
                {
                    "from": "BuildingImage", "to": "RollingBack",
                    "message": "docker build error: Cannot locate specified Dockerfile"
                },
                { "from": "RollingBack", "to": "Failed", "message": null }
            ]
        });
        assert_eq!(
            failure_reason(&dep),
            json!("docker build error: Cannot locate specified Dockerfile")
        );
    }

    #[test]
    fn deploy_bem_sucedido_nao_inventa_causa() {
        let dep = json!({
            "id": "dep_1",
            "state": "Pruning",
            "states_log": [
                { "from": "Live", "to": "Pruning", "message": "superseded by newer deployment" }
            ]
        });
        assert_eq!(failure_reason(&dep), Value::Null);
    }

    #[test]
    fn status_de_servico_com_erro_traz_a_causa() {
        assert_eq!(status_label(Some(&json!("Running"))), json!("Running"));
        assert_eq!(
            status_label(Some(&json!({ "Error": "docker build error: sem Dockerfile" }))),
            json!("Error: docker build error: sem Dockerfile")
        );
    }

    #[test]
    fn source_compacta_sem_arrastar_o_compose() {
        let git = source_label(Some(&json!({
            "Git": { "url": "https://x/y", "branch": "main", "dockerfile_path": "Dockerfile",
                     "build_context": ".", "credentials": "segredo" }
        })));
        assert_eq!(git.get("kind").unwrap(), "Git");
        assert_eq!(git.get("branch").unwrap(), "main");
        // Credencial não vaza para uma listagem.
        assert!(git.get("credentials").is_none());

        let compose = source_label(Some(&json!({ "Compose": { "content": "x".repeat(40_000) } })));
        assert_eq!(compose, json!({ "kind": "Compose" }));
    }

    #[test]
    fn indice_de_servicos_achata_o_snapshot() {
        let snap = json!({
            "services": [{
                "project_name": "loja",
                "service": {
                    "id": "svc_1",
                    "status": "Running",
                    "spec": {
                        "name": "web", "project_id": "proj_1", "port": 3000,
                        "source": { "Registry": { "image": "nginx:latest" } }
                    }
                }
            }]
        });
        let idx = service_index(&snap);
        assert_eq!(idx.len(), 1);
        assert_eq!(idx[0].get("service_id").unwrap(), "svc_1");
        assert_eq!(idx[0].get("name").unwrap(), "web");
        assert_eq!(idx[0].get("project").unwrap(), "loja");
        assert_eq!(idx[0].get("status").unwrap(), "Running");
    }

    #[test]
    fn sumario_recente_junta_nome_de_servico_e_projeto() {
        let sum = json!({
            "service_name": "web",
            "project_name": "loja",
            "deployment": { "id": "dep_1", "service_id": "svc_1", "state": "Live",
                            "states_log": [], "image": "nginx" }
        });
        let out = compact_summary(&sum);
        assert_eq!(out.get("service").unwrap(), "web");
        assert_eq!(out.get("project").unwrap(), "loja");
        assert_eq!(out.get("ok").unwrap(), &json!(true));
    }
}

/// Teste ponta a ponta da ponte: um daemon rustploy de mentira de um lado, a
/// API de agente no meio, um cliente HTTP cru do outro.
///
/// É o que separa "compila" de "funciona": exercita o roteamento, o gate de
/// token, o gate de sessão e — o principal — o `POST /agent/deploys` com
/// `wait`, que é a rota que existe para responder "funcionou ou falhou, e por
/// quê" numa chamada só.
#[cfg(test)]
mod e2e_tests {
    use super::*;
    use hyper::body::Bytes as HBytes;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Quantas vezes o daemon de mentira já respondeu um `DeployHistory` — o
    /// deploy só "termina" na segunda consulta, para o teste passar mesmo pelo
    /// caminho de espera em vez de acertar terminal de primeira.
    static HISTORY_HITS: AtomicUsize = AtomicUsize::new(0);

    const DEP_ID: &str = "dep_01TESTE";
    const SVC_ID: &str = "svc_01TESTE";
    const CAUSA: &str = "docker build error: Cannot locate specified Dockerfile: Dockerfile";

    /// Responde o subconjunto do protocolo que estas rotas usam.
    fn fake_daemon_reply(cmd: &Value) -> Value {
        // Variante unitária chega como string nua; a com campos, como objeto de
        // uma chave. Igualzinho ao daemon de verdade.
        if let Some(nome) = cmd.as_str() {
            return match nome {
                "DaemonStatus" => json!({ "DaemonStatus": {
                    "version": "0.1.0", "uptime_secs": 42,
                    "services_running": 1, "services_total": 2
                }}),
                "Snapshot" => json!({ "Snapshot": serde_json::to_string(&json!({
                    "services": [{
                        "project_name": "loja",
                        "service": {
                            "id": SVC_ID,
                            "status": "Running",
                            "spec": {
                                "name": "web", "project_id": "proj_1", "port": 3000,
                                "source": { "Git": {
                                    "url": "https://github.com/x/y", "branch": "main",
                                    "dockerfile_path": "Dockerfile", "build_context": "."
                                }}
                            }
                        }
                    }]
                })).unwrap() }),
                outro => json!({ "Err": { "code": "Unsupported", "message": outro } }),
            };
        }

        let (nome, _args) = cmd
            .as_object()
            .and_then(|m| m.iter().next())
            .map(|(k, v)| (k.as_str(), v))
            .unwrap_or(("?", &Value::Null));

        match nome {
            "DeployStart" => json!({ "Deployment": {
                "id": DEP_ID, "service_id": SVC_ID, "image": "rp_web:1",
                "state": "Pending", "states_log": [],
                "started_at": "2026-08-27T13:01:41Z", "finished_at": null
            }}),
            "DeployHistory" => {
                let n = HISTORY_HITS.fetch_add(1, Ordering::SeqCst);
                let (estado, fim, log) = if n == 0 {
                    ("BuildingImage", Value::Null, json!([]))
                } else {
                    (
                        "Failed",
                        json!("2026-08-27T13:01:47Z"),
                        json!([
                            { "from": "BuildingImage", "to": "RollingBack",
                              "at": "2026-08-27T13:01:47Z", "message": CAUSA },
                            { "from": "RollingBack", "to": "Failed",
                              "at": "2026-08-27T13:01:47Z", "message": null }
                        ]),
                    )
                };
                json!({ "Deployments": [{
                    "id": DEP_ID, "service_id": SVC_ID, "image": "rp_web:1",
                    "state": estado, "states_log": log,
                    "started_at": "2026-08-27T13:01:41Z", "finished_at": fim
                }]})
            }
            "GetBuildLogs" => json!({ "BuildLogs": (0..5)
                .map(|i| json!({
                    "stream": "Stdout",
                    "line": format!("linha {i}"),
                    "timestamp": "2026-08-27T13:01:41Z"
                }))
                .collect::<Vec<_>>() }),
            outro => json!({ "Err": { "code": "Unsupported", "message": outro } }),
        }
    }

    /// Sobe o daemon de mentira e devolve a base_url dele.
    async fn spawn_fake_daemon() -> String {
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();

        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = l.accept().await else { continue };
                tokio::spawn(async move {
                    let svc = service_fn(|req: Request<Incoming>| async move {
                        let bytes = req.into_body().collect().await.unwrap().to_bytes();
                        let cmd: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                        let corpo = serde_json::to_vec(&fake_daemon_reply(&cmd)).unwrap();
                        Ok::<_, Infallible>(Response::new(Full::new(HBytes::from(corpo))))
                    });
                    let _ = hyper::server::conn::http1::Builder::new()
                        .serve_connection(TokioIo::new(stream), svc)
                        .await;
                });
            }
        });

        format!("http://{addr}")
    }

    /// Sobe a ponte apontando para `remote_base` (ou desconectada, se `None`) e
    /// devolve (base_url da ponte, token dela).
    async fn spawn_bridge(remote_base: Option<&str>) -> (String, String) {
        let session = SharedSession::default();
        if let Some(base) = remote_base {
            let ctx: std::collections::HashMap<String, String> = [
                ("connected", "true"),
                ("api_url", base),
                ("api_token", "token-do-daemon"),
            ]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
            session.sync_from_context(&ctx);
        }

        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = l.local_addr().unwrap();
        let token = "token-de-teste".to_string();

        let ctx = Arc::new(Ctx {
            session,
            remote: Remote::new().unwrap(),
            // O canal externo dos testes não tem motor do outro lado: as
            // mensagens caem num receptor que ninguém drena, o que é
            // exatamente o certo aqui — estes testes exercitam o HTTP e a
            // conversa com o daemon, não o efeito na janela.
            ui: glacier_ui::external::sender(),
            token: token.clone(),
            addr,
        });
        tokio::spawn(accept_loop(l, ctx));

        (format!("http://{addr}"), token)
    }

    /// Cliente HTTP cru: hyper direto sobre TCP, sem TLS — é tudo loopback.
    async fn call(
        base: &str,
        metodo: Method,
        caminho: &str,
        token: Option<&str>,
        corpo: Option<Value>,
    ) -> (StatusCode, Value) {
        let authority = base.trim_start_matches("http://").to_string();
        let stream = tokio::net::TcpStream::connect(&authority).await.unwrap();
        let (mut sender, conn) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
            .await
            .unwrap();
        tokio::spawn(async move {
            let _ = conn.await;
        });

        let mut req = Request::builder()
            .method(metodo)
            .uri(caminho)
            .header(hyper::header::HOST, authority);
        if let Some(t) = token {
            req = req.header(AUTHORIZATION, format!("Bearer {t}"));
        }
        let req = req
            .body(Full::new(HBytes::from(
                corpo.map(|c| serde_json::to_vec(&c).unwrap()).unwrap_or_default(),
            )))
            .unwrap();

        let resp = sender.send_request(req).await.unwrap();
        let status = resp.status();
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let v = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        (status, v)
    }

    #[tokio::test]
    async fn health_dispensa_token_e_diz_se_ha_sessao() {
        let (bridge, _t) = spawn_bridge(None).await;
        let (status, body) = call(&bridge, Method::GET, "/agent/health", None, None).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("ok").unwrap(), &json!(true));
        assert_eq!(body.get("connected").unwrap(), &json!(false));
    }

    #[tokio::test]
    async fn rota_com_dado_exige_token() {
        let (bridge, token) = spawn_bridge(None).await;

        let (sem, body) = call(&bridge, Method::GET, "/agent/deploys", None, None).await;
        assert_eq!(sem, StatusCode::UNAUTHORIZED);
        assert_eq!(body.pointer("/error/code").unwrap(), "unauthorized");

        let (errado, _) =
            call(&bridge, Method::GET, "/agent/deploys", Some("outro"), None).await;
        assert_eq!(errado, StatusCode::UNAUTHORIZED);

        // Com o token certo, o que barra agora é a falta de sessão — prova que
        // passou do gate.
        let (certo, body) =
            call(&bridge, Method::GET, "/agent/deploys", Some(&token), None).await;
        assert_eq!(certo, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(body.pointer("/error/code").unwrap(), "not_connected");
    }

    #[tokio::test]
    async fn rota_inexistente_aponta_para_o_schema() {
        let (bridge, token) = spawn_bridge(None).await;
        let (status, body) = call(&bridge, Method::GET, "/agent/nada", Some(&token), None).await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(body
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap()
            .contains("/agent/schema"));
    }

    /// O caso do plano, ponta a ponta: dispara o deploy, espera, e a resposta
    /// já traz `ok=false` com a causa que o Docker deu — sem o agente precisar
    /// saber que ela mora no `states_log`.
    #[tokio::test]
    async fn deploy_com_wait_devolve_o_desfecho_e_a_causa() {
        HISTORY_HITS.store(0, Ordering::SeqCst);
        let daemon = spawn_fake_daemon().await;
        let (bridge, token) = spawn_bridge(Some(&daemon)).await;

        let (status, body) = call(
            &bridge,
            Method::POST,
            "/agent/deploys",
            Some(&token),
            // Por NOME, não por id: o daemon de mentira resolve via Snapshot.
            Some(json!({ "service": "web", "wait": true, "timeout_s": 30, "log_tail": 3 })),
        )
        .await;

        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(body.get("deployment_id").unwrap(), DEP_ID);
        assert_eq!(body.get("state").unwrap(), "Failed");
        assert_eq!(body.get("ok").unwrap(), &json!(false));
        assert_eq!(body.get("error").unwrap(), CAUSA);
        assert_eq!(body.get("timed_out").unwrap(), &json!(false));

        // log_tail traz só o fim, e o cursor diz onde continuar.
        let log = body.get("log_tail").and_then(Value::as_array).unwrap();
        assert_eq!(log.len(), 3);
        assert_eq!(log[2], "linha 4");
        assert_eq!(body.get("log_cursor").unwrap(), &json!(5));

        // Passou pelo caminho de espera: o primeiro DeployHistory ainda estava
        // em BuildingImage.
        assert!(HISTORY_HITS.load(Ordering::SeqCst) >= 2);
    }

    #[tokio::test]
    async fn nome_de_servico_desconhecido_da_404_util() {
        let daemon = spawn_fake_daemon().await;
        let (bridge, token) = spawn_bridge(Some(&daemon)).await;

        let (status, body) = call(
            &bridge,
            Method::POST,
            "/agent/deploys",
            Some(&token),
            Some(json!({ "service": "nao-existe" })),
        )
        .await;

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.pointer("/error/code").unwrap(), "no_such_service");
    }

    #[tokio::test]
    async fn indice_de_servicos_sai_achatado() {
        let daemon = spawn_fake_daemon().await;
        let (bridge, token) = spawn_bridge(Some(&daemon)).await;

        let (status, body) =
            call(&bridge, Method::GET, "/agent/services", Some(&token), None).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("count").unwrap(), &json!(1));
        let s = &body.get("services").unwrap()[0];
        assert_eq!(s.get("service_id").unwrap(), SVC_ID);
        assert_eq!(s.get("name").unwrap(), "web");
        assert_eq!(s.get("project").unwrap(), "loja");
        assert_eq!(s.pointer("/source/kind").unwrap(), "Git");
    }

    #[tokio::test]
    async fn build_log_pagina_por_cursor() {
        let daemon = spawn_fake_daemon().await;
        let (bridge, token) = spawn_bridge(Some(&daemon)).await;

        let (status, body) = call(
            &bridge,
            Method::GET,
            &format!("/agent/deploys/{DEP_ID}/logs?after=3&limit=10"),
            Some(&token),
            None,
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.get("total").unwrap(), &json!(5));
        assert_eq!(body.get("next_after").unwrap(), &json!(5));
        assert_eq!(body.get("has_more").unwrap(), &json!(false));
        assert_eq!(
            body.get("lines").unwrap(),
            &json!(["linha 3", "linha 4"])
        );
    }

    /// O passthrough encaminha comando cru e devolve a Response verbatim.
    #[tokio::test]
    async fn passthrough_encaminha_command_cru() {
        let daemon = spawn_fake_daemon().await;
        let (bridge, token) = spawn_bridge(Some(&daemon)).await;

        let (status, body) = call(
            &bridge,
            Method::POST,
            "/agent/rpc",
            Some(&token),
            Some(json!("DaemonStatus")),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body.pointer("/DaemonStatus/version").unwrap(), "0.1.0");
    }

    /// `Response::Err` do daemon vira 400 com o código e a mensagem dele — não
    /// um 200 que o agente teria de inspecionar para descobrir que falhou.
    #[tokio::test]
    async fn erro_do_daemon_vira_400() {
        let daemon = spawn_fake_daemon().await;
        let (bridge, token) = spawn_bridge(Some(&daemon)).await;

        let (status, body) = call(
            &bridge,
            Method::POST,
            "/agent/rpc",
            Some(&token),
            Some(json!("ComandoQueNaoExiste")),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.pointer("/error/code").unwrap(), "daemon_command_error");
    }

    /// Daemon inalcançável é 502 (falhou o salto daqui para lá), não 500 —
    /// a distinção diz ao agente se adianta repetir.
    #[tokio::test]
    async fn daemon_inalcancavel_vira_502() {
        // Porta fechada de propósito.
        let (bridge, token) = spawn_bridge(Some("http://127.0.0.1:1")).await;

        let (status, body) = call(
            &bridge,
            Method::POST,
            "/agent/rpc",
            Some(&token),
            Some(json!("DaemonStatus")),
        )
        .await;

        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(body.pointer("/error/code").unwrap(), "daemon_unreachable");
    }
}
