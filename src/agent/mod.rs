use crate::Result;
pub use traits::*;
use serde_derive::Deserialize;
pub use linux::LinuxAgent;
use crate::connection::{ConnectionConfig, connect};

mod traits;
mod linux;

#[derive(Deserialize, Debug)]
pub enum Platform {
    Linux,
}

#[derive(Deserialize, Debug)]
pub struct AgentConfig {
    #[serde(flatten)]
    pub connection: ConnectionConfig,
    pub platform: Platform,
}

pub async fn from_config(config: &AgentConfig) -> Result<BoxAgent> 
{
    let conn_cfg = config.connection.clone();
    let factory = move || {
        connect(conn_cfg.clone())
    };

    let mut agent: BoxAgent = match config.platform {
        Platform::Linux => {
            Box::new(LinuxAgent::new(factory).await?)
        }
    };
    agent.check().await?;

    Ok(agent)
}
