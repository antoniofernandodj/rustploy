//! Cliente do helper privilegiado de firewall (`rustployd-fw`).
//!
//! O daemon roda sem privilégios (`NoNewPrivileges=yes`), então quem toca no
//! ufw é um binário root separado, ativado por socket do systemd em
//! `/run/rustploy/fw.sock` (dono root:rustploy, modo 0660). O protocolo é uma
//! linha JSON por conexão: `{"op":"allow"|"deny","port":N}` →
//! `{"ok":bool,"backend":"ufw"|"none","error":...}`.
//!
//! Política de erro: falha de firewall NUNCA aborta criação/atualização/deploy
//! de serviço — o pior caso é o comportamento antigo (porta bloqueada, admin
//! libera à mão), nunca pior que ele.

use serde::Deserialize;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tracing::{info, warn};

const DEFAULT_SOCKET: &str = "/run/rustploy/fw.sock";
/// `ufw` normalmente responde em milissegundos; o timeout largo cobre o
/// primeiro start do helper via socket activation.
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize)]
struct FwResponse {
    ok: bool,
    #[serde(default)]
    backend: String,
    #[serde(default)]
    error: String,
}

fn socket_path() -> String {
    std::env::var("RUSTPLOY_FW_SOCKET").unwrap_or_else(|_| DEFAULT_SOCKET.into())
}

/// Libera `port`/tcp no firewall do host. `Ok(backend)` informa quem aplicou
/// ("ufw", ou "none" quando não há firewall ativo — nada a liberar).
pub async fn ensure_allowed(port: u16) -> Result<String, String> {
    request("allow", port).await
}

/// Remove a liberação de `port`/tcp.
pub async fn ensure_denied(port: u16) -> Result<String, String> {
    request("deny", port).await
}

/// Variante fire-and-forget para handlers de RPC: não atrasa a resposta ao
/// cliente; o resultado vai para o log estruturado.
pub fn ensure_allowed_bg(port: u16) {
    tokio::spawn(async move {
        let _ = ensure_allowed(port).await;
    });
}

pub fn ensure_denied_bg(port: u16) {
    tokio::spawn(async move {
        let _ = ensure_denied(port).await;
    });
}

async fn request(op: &str, port: u16) -> Result<String, String> {
    let result = tokio::time::timeout(TIMEOUT, do_request(op, port)).await;
    let outcome = match result {
        Ok(r) => r,
        Err(_) => Err("timeout aguardando o helper".into()),
    };
    match &outcome {
        Ok(backend) => info!(op, port, backend, "firewall: regra aplicada"),
        Err(e) => warn!(
            op,
            port,
            error = %e,
            "firewall: falha ao aplicar regra (pode exigir liberação manual da porta)"
        ),
    }
    outcome
}

async fn do_request(op: &str, port: u16) -> Result<String, String> {
    let path = socket_path();
    let mut stream = UnixStream::connect(&path)
        .await
        .map_err(|e| format!("helper indisponível em {path}: {e}"))?;
    let req = format!("{{\"op\":\"{op}\",\"port\":{port}}}\n");
    stream
        .write_all(req.as_bytes())
        .await
        .map_err(|e| format!("falha ao escrever no helper: {e}"))?;

    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .await
        .map_err(|e| format!("falha ao ler resposta do helper: {e}"))?;
    let resp: FwResponse =
        serde_json::from_str(line.trim()).map_err(|e| format!("resposta inválida do helper: {e}"))?;
    if resp.ok {
        Ok(if resp.backend.is_empty() { "?".into() } else { resp.backend })
    } else {
        Err(if resp.error.is_empty() { "erro não informado".into() } else { resp.error })
    }
}
