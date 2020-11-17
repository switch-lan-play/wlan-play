use serde_derive::Deserialize;
use crate::agent::Platform;
use crate::connection::ConnectionConfig;

#[derive(Deserialize, Debug)]
pub struct Agent {
    #[serde(flatten)]
    pub connection: ConnectionConfig,
    /// rhai script will be run after connected
    pub after_connected: Option<String>,
    pub platform: Platform,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    /// agent config
    pub agent: Agent,
    /// device must be monitor type
    pub device: String,
}
