use serde_derive::Deserialize;
use crate::agent::Platform;
use crate::connection::ConnectionConfig;

#[derive(Deserialize, Debug)]
pub struct Agent {
    #[serde(flatten)]
    pub connection: ConnectionConfig,
    pub after_connected: Option<String>,
    pub platform: Platform,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub agent: Agent
}
