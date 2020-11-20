use serde_derive::Deserialize;
use crate::agent::AgentConfig;


#[derive(Deserialize, Debug)]
pub enum Mode {
    Host,
    Station,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    /// agent config
    pub agent: AgentConfig,
    /// device must be monitor type
    pub device: String,
    /// host or station mode
    pub mode: Mode,
}
