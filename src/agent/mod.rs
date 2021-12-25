use crate::connection::{connect, ConnectionConfig};
use crate::Result;
pub use linux::LinuxAgent;
use serde_derive::Deserialize;
pub use traits::*;

mod linux;
mod traits;

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

pub async fn from_config(config: &AgentConfig) -> Result<BoxAgent> {
    let conn_cfg = config.connection.clone();
    let factory = move || connect(conn_cfg.clone());

    let mut agent: BoxAgent = match config.platform {
        Platform::Linux => Box::new(LinuxAgent::new(factory).await?),
    };
    agent.check().await?;

    Ok(agent)
}
