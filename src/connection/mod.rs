use anyhow::{Result, anyhow};
pub use url::Url;
pub use traits::{Connection, BoxConnection};
pub use script::run_script;
use serde_derive::Deserialize;

mod command;
mod ssh;
mod traits;
mod script;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum ConnectionConfig {
    Url { url: Url },
    Command { command: Vec<String> },
}

pub async fn connect(connection: ConnectionConfig) -> Result<BoxConnection> {
    Ok(match connection {
        ConnectionConfig::Url { url } => {
            match url.scheme() {
                "ssh" => {
                    ssh::connect(&url).await
                }
                _ => {
                    Err(anyhow!("{} not support!", url.scheme()))?
                }
            }?
        }
        ConnectionConfig::Command { command } => {
            let (_, args) = command.split_at(1);
            command::connect((&command[0], args)).await?
        }
    })
}
