use anyhow::Result;
use super::{Agent, Commander, BoxAgent, Connection};
use crate::utils::{TimeoutExt, DEFAULT_TIMEOUT};
use tokio::prelude::*;
pub struct LinuxAgent(Connection);

#[async_trait::async_trait]
impl Commander for LinuxAgent {
    async fn exec(&mut self, command: &[u8]) -> Result<Vec<u8>> {
        self.0.write_all("echo '---start---'\n".as_bytes()).await?;
        self.0.write_all(command).await?;
        self.0.write_all("\necho '---end---'\n".as_bytes()).await?;

        self.0.read_line_timeout(DEFAULT_TIMEOUT).await?;
        todo!()
    }

    async fn exec_stream(&mut self, command: &[u8]) -> Result<Box<dyn AsyncRead>> {
        todo!()
    }
}

#[async_trait::async_trait]
impl Agent for LinuxAgent {
    async fn check(&mut self) -> Result<()> {
        Ok(())
    }

    async fn list_device(&mut self) -> Result<Vec<String>> {
        todo!()
    }
}

pub async fn from_connection(conn: Connection) -> Result<BoxAgent> {
    Ok(Box::new(LinuxAgent(conn)))
}
