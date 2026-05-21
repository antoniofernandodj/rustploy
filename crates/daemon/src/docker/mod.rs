pub mod containers;
pub mod images;
pub mod networks;

use anyhow::Result;
use bollard::Docker;

pub struct DockerClient {
    pub inner: Docker,
    pub socket_path: String,
}

impl DockerClient {
    pub fn connect(socket_path: &str) -> Result<Self> {
        let docker = Docker::connect_with_unix(socket_path, 120, bollard::API_DEFAULT_VERSION)?;
        Ok(Self {
            inner: docker,
            socket_path: socket_path.to_string(),
        })
    }

    pub async fn ping(&self) -> Result<()> {
        self.inner.ping().await?;
        Ok(())
    }
}
