use anyhow::Result;
pub use tokio::{
    io::{self, AsyncRead, AsyncBufRead, AsyncBufReadExt, AsyncWrite},
    time::{timeout, Duration},
    stream::Stream
};

#[derive(Debug)]
pub enum DeviceType {
    Dev,
    Phy,
    Priv,
}

#[derive(Debug)]
pub struct Device {
    pub device_type: DeviceType,
    pub name: String,
}

pub struct Packet {
    pub data: Vec<u8>,
}

#[async_trait::async_trait]
pub trait Executor {
    /// Execute command
    async fn exec(&mut self, command: &[u8]) -> Result<Vec<u8>>;
    /// For infinite output
    async fn exec_stream(&mut self, command: &[u8]) -> Result<Box<dyn AsyncRead>>;
}

#[async_trait::async_trait]
pub trait Agent : Executor {
    async fn check(&mut self) -> Result<()>;
    async fn list_device(&mut self) -> Result<Vec<Device>>;
    async fn capture_packets(&mut self, device: &Device) -> Result<Box<dyn Stream<Item=Packet>>>;
}

pub type BoxAgent = Box<dyn Agent + Send + Unpin>;
