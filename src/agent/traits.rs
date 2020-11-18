use anyhow::Result;
use super::Platform;
pub use tokio::{
    io::{self, AsyncRead, AsyncBufRead, AsyncBufReadExt, AsyncWrite},
    time::{timeout, Duration},
    stream::Stream
};
pub use crate::connection::AsyncStream;
pub use crate::utils::Packet;

#[derive(Debug, PartialEq)]
pub enum DeviceType {
    Dev,
    Phy,
}

#[derive(Debug)]
pub struct Device {
    pub device_type: DeviceType,
    pub name: String,
}

#[async_trait::async_trait]
pub trait Executor {
    /// Execute command
    async fn exec_bytes(&mut self, command: &[u8]) -> Result<Vec<u8>>;
    /// Execute command string
    async fn exec(&mut self, command: &str) -> Result<String>
    where
        Self: Sized
    {
        Ok(String::from_utf8(self.exec_bytes(command.as_bytes()).await?)?)
    }
    /// For infinite output
    async fn exec_stream(self, command: &[u8]) -> Result<Box<dyn AsyncStream + Unpin + Send + 'static>>;
}

#[async_trait::async_trait]
pub trait Agent {
    async fn check(&mut self) -> Result<()>;
    async fn list_device(&mut self) -> Result<Vec<Device>>;
    async fn capture_packets(&mut self, device: &Device) -> Result<Box<dyn Stream<Item=Result<Packet>> + Unpin + Send + 'static>>;
    async fn send_packets<'a>(&mut self, device: &Device, packets: &'a (dyn Stream<Item=Packet> + Unpin + Send + Sync)) -> Result<()>;
    fn platform(&self) -> Platform;
}

pub type BoxAgent = Box<dyn Agent + Send + Unpin>;
