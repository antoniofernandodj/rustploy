use anyhow::Result;
use bytes::Bytes;
use http_body_util::{BodyExt, Empty, Full};
use hyper::Request;
use postcard;
use shared::{Command, Event, Response};
use std::path::PathBuf;
use tokio::net::UnixStream;

pub struct DaemonClient {
    pub socket_path: PathBuf,
}

impl DaemonClient {
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self { socket_path: socket_path.into() }
    }

    pub async fn ping(&self) -> bool {
        matches!(self.send(Command::Ping).await, Ok(Response::Pong { .. }))
    }

    pub async fn send(&self, cmd: Command) -> Result<Response> {
        let stream = UnixStream::connect(&self.socket_path).await?;
        let io = hyper_util::rt::TokioIo::new(stream);
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
        tokio::spawn(async move { let _ = conn.await; });

        let body_bytes = postcard::to_allocvec(&cmd)?;
        let req = Request::builder()
            .method("POST")
            .uri("http://localhost/rpc")
            .header("content-type", "application/octet-stream")
            .body(Full::new(Bytes::from(body_bytes)))?;

        let resp = sender.send_request(req).await?;
        let body = resp.collect().await?.to_bytes();
        Ok(postcard::from_bytes(&body)?)
    }

    /// Connects to the daemon event stream and calls `on_event` for each decoded event.
    /// Uses the hyper HTTP/1 client so chunked transfer encoding is handled transparently.
    pub async fn stream<F>(&self, service_id: Option<&str>, mut on_event: F) -> Result<()>
    where
        F: FnMut(Event) + Send,
    {
        let path = match service_id {
            Some(id) => format!("/stream?service={id}"),
            None => "/stream".to_string(),
        };

        let stream = UnixStream::connect(&self.socket_path).await?;
        let io = hyper_util::rt::TokioIo::new(stream);
        let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
        tokio::spawn(async move { let _ = conn.await; });

        let req = Request::builder()
            .method("GET")
            .uri(format!("http://localhost{path}"))
            .body(Empty::<Bytes>::new())?;

        let resp = sender.send_request(req).await?;
        let mut body = resp.into_body();

        // Buffer for partial [u32 LE len][postcard bytes] frames that span chunk boundaries.
        let mut buf: Vec<u8> = Vec::new();

        loop {
            let Some(frame) = body.frame().await else { break };
            let frame = frame?;
            let Some(data) = frame.data_ref() else { continue };
            buf.extend_from_slice(data);

            while buf.len() >= 4 {
                let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
                if len == 0 || len > 4 * 1024 * 1024 {
                    return Ok(());
                }
                if buf.len() < 4 + len {
                    break;
                }
                if let Ok(event) = postcard::from_bytes::<Event>(&buf[4..4 + len]) {
                    on_event(event);
                }
                buf.drain(..4 + len);
            }
        }

        Ok(())
    }
}
