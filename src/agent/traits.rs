use anyhow::Result;
use super::Platform;
pub use tokio::{
    io::{self, AsyncRead, AsyncBufRead, AsyncBufReadExt, AsyncWrite},
    time::{timeout, Duration},
    stream::Stream
};
pub use crate::utils::pcap_reader::Packet;

#[derive(Debug)]
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
    async fn exec_stream<'a>(&'a mut self, command: &[u8]) -> Result<Box<dyn AsyncRead + Unpin + Send + 'a>>;
}

#[async_trait::async_trait]
pub trait Agent {
    async fn check(&mut self) -> Result<()>;
    async fn list_device(&mut self) -> Result<Vec<Device>>;
    async fn capture_packets<'a>(&'a mut self, device: &Device) -> Result<Box<dyn Stream<Item=Result<Packet>> + Unpin + Send + 'a>>;
    // async fn start_packets(&mut self);
    fn platform(&self) -> Platform;
}

pub type BoxAgent = Box<dyn Agent + Send + Unpin>;
