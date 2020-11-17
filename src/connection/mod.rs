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

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum ConnectionMethod {
    Url { url: Url },
    Command { command: Vec<String> },
}

#[derive(Deserialize, Debug, Clone)]
pub struct ConnectionConfig {
    #[serde(flatten)]
    pub method: ConnectionMethod,
    
    /// rhai script will be run after connected
    pub after_connected: Option<String>,
}


pub async fn connect(config: ConnectionConfig) -> Result<Connection> {
    let stream: BoxAsyncStream = match config.method {
        ConnectionMethod::Url { url } => {
            match url.scheme() {
                "ssh" => {
                    Box::new(SshConnection::new(&url).await?)
                }
                _ => {
                    Err(anyhow!("{} not support!", url.scheme()))?
                }
            }
        }
        ConnectionMethod::Command { command } => {
            let (_, args) = command.split_at(1);
            Box::new(CommandConnection::new((&command[0], args)).await?)
        }
    };
    let mut conn = Connection::new(stream);

    if let Some(script) = config.after_connected {
        log::debug!("after_connected is set, run script...");
        conn = run_script(conn, script).await?;
    }
    log::debug!("Connection is ready");

    Ok(conn)
}

#[async_trait::async_trait]
pub trait AsyncStream: AsyncRead + AsyncWrite {
}

pub type BoxAsyncStream = Box<dyn AsyncStream + Send + Sync + Unpin>;

pub type Connection = BufStream<BoxAsyncStream>;
// type ConnectionFactory = Box<dyn (Fn() -> BoxFuture<'static, Result<Connection>>) + Send + 'static>;
