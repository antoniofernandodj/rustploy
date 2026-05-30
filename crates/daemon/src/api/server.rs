use super::{routes::dispatch, AppState};
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
    let mut rx: tokio::sync::broadcast::Receiver<Event> = state.bus.subscribe();
    loop {
        match rx.recv().await {
            Ok(event) => {
                if let Some(ref sid) = service_id {
                    if event.matches(sid) { continue; }
                }
                match postcard::to_allocvec(&event) {
                    Ok(payload) => {
                        let len: [u8; 4] = (payload.len() as u32).to_le_bytes();
                        if
                            stream.write_all(&len).await.is_err() ||
                            stream.write_all(&payload).await.is_err() {
                                break;
                        }
                    }
                    Err(e) => warn!(error = %e, "failed to serialize event"),
                }
            }
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => break,
        }
    }
}
