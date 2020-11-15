pub use tokio::io::{AsyncRead, AsyncWrite};

#[async_trait::async_trait]
pub trait Connection: AsyncRead + AsyncWrite {
    // async fn send(&mut self, data: &[u8]) -> Result<()>;
    // async fn recv(&mut self) -> Option<Vec<u8>>;
}

pub type BoxConnection = Box<dyn Connection + Send + Unpin>;
