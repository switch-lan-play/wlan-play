use anyhow::Result;
pub use tokio::{io::{self, AsyncRead, AsyncBufRead, AsyncBufReadExt, AsyncWrite}, time::{timeout, Duration}};

#[async_trait::async_trait]
pub trait Commander {
    /// Execute command
    async fn exec(&mut self, command: &[u8]) -> Result<Vec<u8>>;
    /// For infinite output
    async fn exec_stream(&mut self, command: &[u8]) -> Result<Box<dyn AsyncRead>>;
}

#[async_trait::async_trait]
pub trait Agent : Commander {
    async fn check(&mut self) -> Result<()>;
    async fn list_device(&mut self) -> Result<Vec<String>>;
}

pub type BoxAgent = Box<dyn Agent + Send + Unpin>;
