use anyhow::{Context, Result};
pub use tokio::{io::{self, AsyncRead, AsyncReadExt, AsyncBufRead, AsyncBufReadExt, AsyncWrite}, time::{timeout, Duration}};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

#[async_trait::async_trait]
pub trait TimeoutExt : AsyncBufRead + Unpin + Send {
    async fn read_line_timeout(&mut self, duration: Duration) -> Result<String> {
        let mut buf = String::new();
        timeout(duration, self.read_line(&mut buf))
            .await
            .context("read_line_timeout")??;
        Ok(buf)
    }
    async fn read_until_timeout(&mut self, duration: Duration, bytes: &[u8]) -> Result<Vec<u8>> {
        let byte = bytes[bytes.len() - 1];
        let mut buf: Vec<u8> = vec![];
        loop {
            timeout(duration, self.read_until(byte, &mut buf))
                .await
                .context("read_until_timeout read_until")??;
            if buf.len() >= bytes.len() && &buf[buf.len() - bytes.len()..buf.len()] == bytes {
                buf.truncate(buf.len() - bytes.len());
                break;
            }
        }

        Ok(buf)
    }
}

impl<T: AsyncBufRead + Unpin + Send> TimeoutExt for T {}
