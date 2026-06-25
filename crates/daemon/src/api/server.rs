use super::{AppState, routes::dispatch};
use anyhow::Result;
use shared::{ClientFrame, Event};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::broadcast::error::RecvError;
use tracing::warn;

const MAX_FRAME: usize = 4 * 1024 * 1024;

pub async fn handle_connection(mut stream: UnixStream, state: AppState) -> Result<()> {
    let frame_bytes = read_frame(&mut stream).await?;
    let msg = postcard::from_bytes::<ClientFrame>(&frame_bytes)?;

    match msg {
        ClientFrame::Rpc(cmd) => {
            let resp = dispatch(state, cmd).await;
            let resp_bytes = postcard::to_allocvec(&resp)?;
            write_frame(&mut stream, &resp_bytes).await?;
        }
        ClientFrame::Subscribe { service_id } => {
            stream_events(stream, state, service_id).await;
        }
    }
    Ok(())
}

async fn read_frame(stream: &mut UnixStream) -> Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf) as usize;
    anyhow::ensure!(len > 0 && len <= MAX_FRAME, "invalid frame length: {len}");
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn write_frame(stream: &mut UnixStream, data: &[u8]) -> Result<()> {
    let len = (data.len() as u32).to_le_bytes();
    stream.write_all(&len).await?;
    stream.write_all(data).await?;
    Ok(())
}

async fn stream_events(mut stream: UnixStream, state: AppState, service_id: Option<String>) {
    // Subscreve ao bus ANTES do replay para não perder eventos emitidos
    // durante a janela entre buscar o histórico e começar a ouvir.
    let mut rx = state.bus.subscribe();

    // Replay: envia eventos persistidos desde o último restart.
    let history = crate::db::event_log::recent(&state.db, service_id.as_deref(), 200)
        .await
        .unwrap_or_default();

    for event in history {
        if !passes_filter(&event, service_id.as_deref()) {
            continue;
        }
        if send_event(&mut stream, &event).await.is_err() {
            return;
        }
    }

    // Stream live.
    loop {
        match rx.recv().await {
            Ok(event) => {
                if !passes_filter(&event, service_id.as_deref()) {
                    continue;
                }
                if send_event(&mut stream, &event).await.is_err() {
                    break;
                }
            }
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => break,
        }
    }
}

/// Retorna `true` se o evento deve ser enviado para esta subscrição.
/// Sem filtro de serviço → todos os eventos passam.
/// Com filtro → só eventos relevantes para aquele serviço (incluindo globais).
fn passes_filter(event: &Event, service_id: Option<&str>) -> bool {
    match service_id {
        None => true,
        Some(sid) => event.matches(sid),
    }
}

async fn send_event(stream: &mut UnixStream, event: &Event) -> Result<()> {
    let payload = postcard::to_allocvec(event)
        .map_err(|e| { warn!(error = %e, "failed to serialize event"); e })?;
    let len = (payload.len() as u32).to_le_bytes();
    stream.write_all(&len).await?;
    stream.write_all(&payload).await?;
    Ok(())
}
