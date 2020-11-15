use anyhow::Result;
pub use tokio::{io::{self, AsyncRead, AsyncBufRead, AsyncBufReadExt, AsyncWrite}, time::{timeout, Duration}};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(1);

#[async_trait::async_trait]
pub trait TimeoutExt : AsyncBufRead + Unpin + Send {
    async fn read_line_timeout(&mut self, duration: Duration) -> Result<String> {
        let mut buf = String::new();
        timeout(duration, self.read_line(&mut buf)).await??;
        Ok(buf)
    }
}

impl<T: AsyncBufRead + Unpin + Send> TimeoutExt for T {}
