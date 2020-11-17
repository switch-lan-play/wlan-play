use anyhow::{Result, anyhow};
pub use url::Url;
pub use script::run_script;
use serde_derive::Deserialize;
pub use tokio::io::{AsyncRead, AsyncBufRead, AsyncWrite, BufStream};
pub use ssh::SshConnection;
pub use command::CommandConnection;

mod command;
mod ssh;
mod script;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum ConnectionConfig {
    Url { url: Url },
    Command { command: Vec<String> },
}

pub async fn connect(connection: &ConnectionConfig) -> Result<Connection> {
    let stream: BoxAsyncStream = match connection {
        ConnectionConfig::Url { url } => {
            match url.scheme() {
                "ssh" => {
                    Box::new(SshConnection::new(&url).await?)
                }
                _ => {
                    Err(anyhow!("{} not support!", url.scheme()))?
                }
            }
        }
        ConnectionConfig::Command { command } => {
            let (_, args) = command.split_at(1);
            Box::new(CommandConnection::new((&command[0], args)).await?)
        }
    };

    Ok(Connection::new(stream))
}

#[async_trait::async_trait]
pub trait AsyncStream: AsyncRead + AsyncWrite {
}

pub type BoxAsyncStream = Box<dyn AsyncStream + Send + Sync + Unpin>;

pub type Connection = BufStream<BoxAsyncStream>;
