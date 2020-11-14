use anyhow::Result;

#[async_trait::async_trait]
pub trait Connection {
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    async fn recv(&mut self) -> Option<Vec<u8>>;
}
