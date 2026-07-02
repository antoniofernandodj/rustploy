use crate::api::{AppState, routes::dispatch};
use shared::{RwpConfig, RwpError, RwpFrame, RwpReply, RWP_PROTOCOL_VERSION};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn};

enum RwpStream {
    Plain(TcpStream),
    Tls(tokio_rustls::server::TlsStream<TcpStream>),
}

impl tokio::io::AsyncRead for RwpStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => std::pin::Pin::new(s).poll_read(cx, buf),
            Self::Tls(s) => std::pin::Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl tokio::io::AsyncWrite for RwpStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        match self.get_mut() {
            Self::Plain(s) => std::pin::Pin::new(s).poll_write(cx, buf),
            Self::Tls(s) => std::pin::Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => std::pin::Pin::new(s).poll_flush(cx),
            Self::Tls(s) => std::pin::Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        match self.get_mut() {
            Self::Plain(s) => std::pin::Pin::new(s).poll_shutdown(cx),
            Self::Tls(s) => std::pin::Pin::new(s).poll_shutdown(cx),
        }
    }
}

fn get_rwp_tls_config(cfg: &RwpConfig) -> anyhow::Result<rustls::ServerConfig> {
    let (cert_pem, key_pem) = if let (Some(cert_path), Some(key_path)) = (&cfg.tls_cert_path, &cfg.tls_key_path) {
        info!(cert_path = %cert_path, key_path = %key_path, "RWP: Carregando certificado TLS configurado");
        let cert_pem = std::fs::read_to_string(cert_path)?;
        let key_pem = std::fs::read_to_string(key_path)?;
        (cert_pem, key_pem)
    } else {
        info!("RWP: Gerando certificado TLS auto-assinado temporário");
        use rcgen::{CertificateParams, DistinguishedName, KeyPair};
        let key_pair = KeyPair::generate()?;
        let mut params = CertificateParams::new(vec!["localhost".to_string(), "127.0.0.1".to_string()])?;
        params.distinguished_name = DistinguishedName::new();
        let cert = params.self_signed(&key_pair)?;
        let cert_pem = cert.pem();
        let key_pem = key_pair.serialize_pem();
        (cert_pem, key_pem)
    };

    use rustls::pki_types::CertificateDer;
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_pem.as_bytes())
        .collect::<Result<_, _>>()?;

    if certs.is_empty() {
        anyhow::bail!("Nenhum certificado encontrado no PEM");
    }

    let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())?
        .ok_or_else(|| anyhow::anyhow!("Nenhuma chave privada encontrada no PEM"))?;

    let server_config = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()?
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    Ok(server_config)
}

/// Starts the RWP TCP listener. Returns once binding fails or never (the accept
/// loop runs forever). Intended to be `tokio::spawn`ed from `main`.
pub async fn run(state: AppState, cfg: RwpConfig) {
    // Safety guard: refuse to expose a non-loopback bind without a token.
    if cfg.is_public_bind() && cfg.token.as_deref().unwrap_or("").is_empty() {
        warn!(
            bind = %cfg.bind_address,
            "RWP: bind não-loopback sem token configurado — listener NÃO iniciado. \
             Defina rwp.token (ou RUSTPLOY_RWP_TOKEN) para expor remotamente."
        );
        return;
    }

    let tls_acceptor = if cfg.tls_enabled {
        match get_rwp_tls_config(&cfg) {
            Ok(sc) => {
                info!("RWP: TLS habilitado");
                Some(TlsAcceptor::from(Arc::new(sc)))
            }
            Err(e) => {
                warn!(error = %e, "RWP: Falha ao inicializar TLS, listener desabilitado");
                return;
            }
        }
    } else {
        None
    };

    let addr = format!("{}:{}", cfg.bind_address, cfg.port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            warn!(error = %e, addr = %addr, "RWP: falha ao bind, listener desabilitado");
            return;
        }
    };

    let auth_required = !cfg.token.as_deref().unwrap_or("").is_empty();
    info!(
        addr = %addr,
        auth = auth_required,
        tls = cfg.tls_enabled,
        max_connections = cfg.max_connections,
        "RWP: ouvindo"
    );

    let cfg = Arc::new(cfg);
    let limiter = Arc::new(Semaphore::new(cfg.max_connections));
    let tls_acceptor = tls_acceptor.map(Arc::new);

    loop {
        let (stream, peer) = match listener.accept().await {
            Ok(v) => v,
            Err(e) => {
                warn!(error = %e, "RWP: accept falhou");
                continue;
            }
        };

        // Reject (by immediate drop) when the connection pool is full.
        let permit = match Arc::clone(&limiter).try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                warn!(peer = %peer, "RWP: limite de conexões atingido, recusando");
                drop(stream);
                continue;
            }
        };

        let state = state.clone();
        let cfg = cfg.clone();
        let tls_acceptor = tls_acceptor.clone();
        tokio::spawn(async move {
            let _permit = permit; // held for the lifetime of the connection
            if let Err(e) = handle_connection(stream, state, &cfg, tls_acceptor.as_deref()).await {
                warn!(peer = %peer, error = %e, "RWP: conexão encerrada com erro");
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
    state: AppState,
    cfg: &RwpConfig,
    tls_acceptor: Option<&TlsAcceptor>,
) -> anyhow::Result<()> {
    let _ = stream.set_nodelay(true);
    let read_to = Duration::from_secs(cfg.read_timeout_secs.max(1));
    let idle_to = Duration::from_secs(cfg.idle_timeout_secs.max(1));
    let auth_required = !cfg.token.as_deref().unwrap_or("").is_empty();

    let mut stream = if let Some(acceptor) = tls_acceptor {
        match timeout(Duration::from_secs(10), acceptor.accept(stream)).await {
            Ok(Ok(tls_stream)) => RwpStream::Tls(tls_stream),
            Ok(Err(e)) => {
                anyhow::bail!("RWP: TLS handshake falhou: {e}");
            }
            Err(_) => {
                anyhow::bail!("RWP: TLS handshake timeout");
            }
        }
    } else {
        RwpStream::Plain(stream)
    };

    // --- Handshake: expect Hello ---
    let first = read_frame_typed::<RwpFrame>(&mut stream, cfg.max_frame_size, read_to).await?;
    match first {
        RwpFrame::Hello { protocol_version, .. } => {
            if protocol_version != RWP_PROTOCOL_VERSION {
                let reply = RwpReply::Error(RwpError::new(
                    "ProtocolVersionMismatch",
                    format!(
                        "daemon fala RWP v{RWP_PROTOCOL_VERSION}, client enviou v{protocol_version}"
                    ),
                ));
                write_reply(&mut stream, &reply, cfg.max_frame_size).await?;
                return Ok(());
            }
            let ack = RwpReply::HelloAck {
                protocol_version: RWP_PROTOCOL_VERSION,
                daemon_version: env!("CARGO_PKG_VERSION").to_string(),
                auth_required,
            };
            write_reply(&mut stream, &ack, cfg.max_frame_size).await?;
        }
        _ => {
            let reply = RwpReply::Error(RwpError::new(
                "ExpectedHello",
                "primeira mensagem deve ser Hello",
            ));
            write_reply(&mut stream, &reply, cfg.max_frame_size).await?;
            return Ok(());
        }
    }

    // --- Authentication (if required) ---
    if auth_required {
        let frame = read_frame_typed::<RwpFrame>(&mut stream, cfg.max_frame_size, read_to).await?;
        match frame {
            RwpFrame::Authenticate { token } => {
                let expected = cfg.token.as_deref().unwrap_or("");
                tracing::debug!(
                    received_len = token.len(),
                    expected_len = expected.len(),
                    received = %token,
                    expected = %expected,
                    "RWP: comparando tokens"
                );
                if !constant_time_eq(token.as_bytes(), expected.as_bytes()) {
                    warn!(
                        received_len = token.len(),
                        expected_len = expected.len(),
                        "RWP: falha de autenticação"
                    );
                    let reply = RwpReply::Error(RwpError::new("Unauthorized", "token inválido"));
                    write_reply(&mut stream, &reply, cfg.max_frame_size).await?;
                    return Ok(()); // close on auth failure
                }
                write_reply(&mut stream, &RwpReply::AuthOk, cfg.max_frame_size).await?;
                // NB: a reconciliação DB↔Docker NÃO é feita aqui. Cada ação do
                // GUI abre uma conexão de comando curta, e reconciliar em toda
                // autenticação disparava um `reconcile` completo (dezenas de
                // chamadas Docker) por clique. Agora ela roda só quando o cliente
                // abre o stream de eventos (`Subscribe`, 1 por sessão) — ver o
                // handler mais abaixo — além do loop periódico de 30s em `main`.
            }
            _ => {
                let reply = RwpReply::Error(RwpError::new(
                    "ExpectedAuthenticate",
                    "autenticação obrigatória",
                ));
                write_reply(&mut stream, &reply, cfg.max_frame_size).await?;
                return Ok(());
            }
        }
    }

    // --- Command / subscribe loop ---
    loop {
        let frame =
            match read_frame_typed::<RwpFrame>(&mut stream, cfg.max_frame_size, idle_to).await {
                Ok(f) => f,
                // Idle timeout or clean disconnect ends the connection.
                Err(_) => return Ok(()),
            };

        match frame {
            RwpFrame::Rpc(cmd) => {
                let resp = dispatch(state.clone(), cmd).await;
                write_reply(&mut stream, &RwpReply::Response(resp), cfg.max_frame_size).await?;
            }
            RwpFrame::Ping => {
                let reply = RwpReply::Pong {
                    uptime_secs: state.started_at.elapsed().as_secs(),
                };
                write_reply(&mut stream, &reply, cfg.max_frame_size).await?;
            }
            RwpFrame::Subscribe { service_id } => {
                // Reconcilia status DB ↔ Docker uma vez, quando o cliente abre o
                // stream de eventos (1 por sessão do GUI), para que ele veja o
                // estado real imediatamente — sem pagar um `reconcile` completo a
                // cada conexão de comando curta (uma por ação).
                {
                    let db = state.db.clone();
                    let docker = state.docker.clone();
                    let ingress = state.ingress.clone();
                    let tls = state.tls.clone();
                    tokio::spawn(async move {
                        crate::deploy::recovery::reconcile(&db, &docker, &ingress, &tls).await;
                    });
                }
                // Becomes a one-way event stream until the peer drops.
                stream_events(stream, state, service_id, cfg.max_frame_size).await;
                return Ok(());
            }
            RwpFrame::Hello { .. } | RwpFrame::Authenticate { .. } => {
                let reply = RwpReply::Error(RwpError::new(
                    "UnexpectedFrame",
                    "handshake já concluído",
                ));
                write_reply(&mut stream, &reply, cfg.max_frame_size).await?;
            }
        }
    }
}

async fn stream_events(
    mut stream: RwpStream,
    state: AppState,
    service_id: Option<String>,
    max_frame: usize,
) {
    let mut rx = state.bus.subscribe();
    loop {
        match rx.recv().await {
            Ok(event) => {
                // Same filtering semantics as the UDS stream: when a service is
                // given, skip events that do NOT belong to it.
                if let Some(ref sid) = service_id {
                    if !event.matches(sid) {
                        continue;
                    }
                }
                if write_reply(&mut stream, &RwpReply::Event(event), max_frame)
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => break,
        }
    }
}

// --- Framing helpers ---------------------------------------------------------

async fn read_frame_typed<T: serde::de::DeserializeOwned>(
    stream: &mut RwpStream,
    max_frame: usize,
    read_timeout: Duration,
) -> anyhow::Result<T> {
    let buf = timeout(read_timeout, read_frame(stream, max_frame)).await??;
    Ok(postcard::from_bytes::<T>(&buf)?)
}

async fn read_frame(stream: &mut RwpStream, max_frame: usize) -> anyhow::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf) as usize;
    anyhow::ensure!(
        len > 0 && len <= max_frame,
        "invalid frame length: {len} (max {max_frame})"
    );
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn write_reply(
    stream: &mut RwpStream,
    reply: &RwpReply,
    max_frame: usize,
) -> anyhow::Result<()> {
    let payload = postcard::to_allocvec(reply)?;
    anyhow::ensure!(
        payload.len() <= max_frame,
        "reply frame too large: {} bytes",
        payload.len()
    );
    let len = (payload.len() as u32).to_le_bytes();
    stream.write_all(&len).await?;
    stream.write_all(&payload).await?;
    Ok(())
}

/// Length-independent only when lengths match; short-circuits on mismatched
/// length but is otherwise constant-time over the compared bytes. Adequate for
/// a static admin token.
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
