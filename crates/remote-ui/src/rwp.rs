//! Async RWP client transport: handshake, auth and framed request/response.

use shared::{Command, Response, RwpFrame, RwpReply, RWP_PROTOCOL_VERSION};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

const MAX_FRAME: usize = 4 * 1024 * 1024;

pub enum RwpStream {
    Plain(TcpStream),
    Tls(tokio_rustls::client::TlsStream<TcpStream>),
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

#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}

pub async fn write_frame<T: serde::Serialize>(s: &mut RwpStream, v: &T) -> anyhow::Result<()> {
    let payload = postcard::to_allocvec(v)?;
    anyhow::ensure!(payload.len() <= MAX_FRAME, "frame too large");
    s.write_all(&(payload.len() as u32).to_le_bytes()).await?;
    s.write_all(&payload).await?;
    Ok(())
}

pub async fn read_frame<T: serde::de::DeserializeOwned>(s: &mut RwpStream) -> anyhow::Result<T> {
    let mut len = [0u8; 4];
    s.read_exact(&mut len).await?;
    let n = u32::from_le_bytes(len) as usize;
    anyhow::ensure!(n > 0 && n <= MAX_FRAME, "invalid frame length: {n}");
    let mut buf = vec![0u8; n];
    s.read_exact(&mut buf).await?;
    Ok(postcard::from_bytes(&buf)?)
}

/// Connects, performs the `Hello` handshake and authenticates if required.
/// Returns a ready-to-use stream positioned right after `AuthOk`.
pub async fn connect(url: &str, token: Option<&str>) -> anyhow::Result<RwpStream> {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });

    let target = crate::connect_target(url)?;
    let raw_stream = TcpStream::connect(&target).await?;
    raw_stream.set_nodelay(true).ok();

    let use_tls = url.starts_with("rwps://");

    let mut s = if use_tls {
        let mut client_config = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoVerifier))
            .with_no_client_auth();

        // Allow ALPN or any other config if needed, keeping it simple.
        client_config.alpn_protocols = vec![b"rwp".to_vec()];

        let connector = TlsConnector::from(Arc::new(client_config));
        
        // Use server name as ServerName
        let domain_part = target.split(':').next().unwrap_or("localhost");
        let server_name = rustls::pki_types::ServerName::try_from(domain_part)
            .unwrap_or_else(|_| rustls::pki_types::ServerName::try_from("localhost").unwrap())
            .to_owned();

        let tls_stream = connector.connect(server_name, raw_stream).await?;
        RwpStream::Tls(tls_stream)
    } else {
        RwpStream::Plain(raw_stream)
    };

    write_frame(
        &mut s,
        &RwpFrame::Hello {
            protocol_version: RWP_PROTOCOL_VERSION,
            client_version: env!("CARGO_PKG_VERSION").to_string(),
        },
    )
    .await?;

    let auth_required = match read_frame::<RwpReply>(&mut s).await? {
        RwpReply::HelloAck {
            protocol_version,
            auth_required,
            ..
        } => {
            anyhow::ensure!(
                protocol_version == RWP_PROTOCOL_VERSION,
                "versão de protocolo incompatível (daemon v{protocol_version})"
            );
            auth_required
        }
        RwpReply::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("handshake inesperado"),
    };

    if auth_required {
        let tok = token.unwrap_or("");
        write_frame(
            &mut s,
            &RwpFrame::Authenticate {
                token: tok.to_string(),
            },
        )
        .await?;
        match read_frame::<RwpReply>(&mut s).await? {
            RwpReply::AuthOk => {}
            RwpReply::Error(e) => anyhow::bail!("autenticação falhou: {}", e.message),
            _ => anyhow::bail!("resposta de autenticação inesperada"),
        }
    }

    Ok(s)
}

/// Issues a single RPC on an already-authenticated command connection.
pub async fn rpc(s: &mut RwpStream, cmd: Command) -> anyhow::Result<Response> {
    write_frame(s, &RwpFrame::Rpc(cmd)).await?;
    match read_frame::<RwpReply>(s).await? {
        RwpReply::Response(r) => Ok(r),
        RwpReply::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("resposta inesperada"),
    }
}

/// Sends a keepalive `Ping` and waits for the matching `Pong`. Used to keep the
/// command connection from hitting the daemon's idle timeout while the user is
/// not issuing any RPCs.
#[allow(dead_code)]
pub async fn ping(s: &mut RwpStream) -> anyhow::Result<()> {
    write_frame(s, &RwpFrame::Ping).await?;
    match read_frame::<RwpReply>(s).await? {
        RwpReply::Pong { .. } => Ok(()),
        RwpReply::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("resposta inesperada ao ping"),
    }
}
