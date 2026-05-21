use anyhow::Result;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request;
use shared::{Command, Event, Response};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

pub struct DaemonClient {
    pub socket_path: PathBuf,
}

impl DaemonClient {
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self { socket_path: socket_path.into() }
    }

    pub async fn send(&self, cmd: Command) -> Result<Response> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        let io = hyper_util::rt::TokioIo::new(stream);

        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
        tokio::spawn(async move { let _ = conn.await; });

        let body_bytes = bincode::serialize(&cmd)?;
        let req = Request::builder()
            .method("POST")
            .uri("http://localhost/rpc")
            .header("content-type", "application/octet-stream")
            .body(Full::new(Bytes::from(body_bytes)))?;

        let resp = sender.send_request(req).await?;
        let body = resp.collect().await?.to_bytes();
        Ok(bincode::deserialize(&body)?)
    }

    /// Connects to the daemon event stream and calls `on_event` for each event.
    pub async fn stream<F>(&self, service_id: Option<&str>, mut on_event: F) -> Result<()>
    where
        F: FnMut(Event) + Send,
    {
        let path = match service_id {
            Some(id) => format!("/stream?service={id}"),
            None => "/stream".to_string(),
        };

        let mut stream = UnixStream::connect(&self.socket_path).await?;
        let req = format!(
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n"
        );
        stream.write_all(req.as_bytes()).await?;

        // Skip HTTP response headers
        let mut buf = Vec::new();
        let mut byte = [0u8; 1];
        loop {
            if stream.read_exact(&mut byte).await.is_err() {
                return Ok(());
            }
            buf.push(byte[0]);
            if buf.ends_with(b"\r\n\r\n") {
                break;
            }
            if buf.len() > 16384 {
                return Ok(());
            }
        }

        // Read framed events: [u32 LE len][bincode payload]
        loop {
            let mut len_buf = [0u8; 4];
            if stream.read_exact(&mut len_buf).await.is_err() {
                break;
            }
            let len = u32::from_le_bytes(len_buf) as usize;
            if len == 0 || len > 4 * 1024 * 1024 {
                break;
            }
            let mut payload = vec![0u8; len];
            if stream.read_exact(&mut payload).await.is_err() {
                break;
            }
            if let Ok(event) = bincode::deserialize::<Event>(&payload) {
                on_event(event);
            }
        }

        Ok(())
    }
}
