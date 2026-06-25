//! Async RWP client transport: handshake, auth and framed request/response.

use shared::{Command, Response, RwpFrame, RwpReply, RWP_PROTOCOL_VERSION};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const MAX_FRAME: usize = 4 * 1024 * 1024;

pub async fn write_frame<T: serde::Serialize>(s: &mut TcpStream, v: &T) -> anyhow::Result<()> {
    let payload = postcard::to_allocvec(v)?;
    anyhow::ensure!(payload.len() <= MAX_FRAME, "frame too large");
    s.write_all(&(payload.len() as u32).to_le_bytes()).await?;
    s.write_all(&payload).await?;
    Ok(())
}

pub async fn read_frame<T: serde::de::DeserializeOwned>(s: &mut TcpStream) -> anyhow::Result<T> {
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
pub async fn connect(addr: &str, token: Option<&str>) -> anyhow::Result<TcpStream> {
    let mut s = TcpStream::connect(addr).await?;
    s.set_nodelay(true).ok();

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
pub async fn rpc(s: &mut TcpStream, cmd: Command) -> anyhow::Result<Response> {
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
pub async fn ping(s: &mut TcpStream) -> anyhow::Result<()> {
    write_frame(s, &RwpFrame::Ping).await?;
    match read_frame::<RwpReply>(s).await? {
        RwpReply::Pong { .. } => Ok(()),
        RwpReply::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("resposta inesperada ao ping"),
    }
}
