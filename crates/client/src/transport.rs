use anyhow::Result;
use shared::{ClientFrame, Command, Event, Response};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

const MAX_FRAME: usize = 4 * 1024 * 1024;

pub struct DaemonClient {
    pub socket_path: PathBuf,
}

impl DaemonClient {
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    pub fn ping(&self) -> bool {
        matches!(self.send(Command::Ping), Ok(Response::Pong { .. }))
    }

    pub fn send(&self, cmd: Command) -> Result<Response> {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        write_frame(&mut stream, &postcard::to_allocvec(&ClientFrame::Rpc(cmd))?)?;
        let buf = read_frame(&mut stream)?;
        Ok(postcard::from_bytes(&buf)?)
    }

    pub fn stream<F>(&self, service_id: Option<&str>, mut on_event: F) -> Result<()>
    where
        F: FnMut(Event),
    {
        let mut stream = UnixStream::connect(&self.socket_path)?;
        let subscribe = ClientFrame::Subscribe {
            service_id: service_id.map(String::from),
        };
        write_frame(&mut stream, &postcard::to_allocvec(&subscribe)?)?;

        loop {
            match read_frame(&mut stream) {
                Ok(buf) => {
                    if let Ok(event) = postcard::from_bytes::<Event>(&buf) {
                        on_event(event);
                    }
                }
                Err(_) => break,
            }
        }
        Ok(())
    }
}

fn write_frame(stream: &mut UnixStream, data: &[u8]) -> Result<()> {
    let len = (data.len() as u32).to_le_bytes();
    stream.write_all(&len)?;
    stream.write_all(data)?;
    Ok(())
}

fn read_frame(stream: &mut UnixStream) -> Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    anyhow::ensure!(len > 0 && len <= MAX_FRAME, "invalid frame length: {len}");
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    Ok(buf)
}
