use serde_derive::Deserialize;
use crate::agent::AgentConfig;
use structopt::StructOpt;
use std::net::SocketAddr;

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
    /// server address:port
    pub server: SocketAddr,
}


#[derive(Debug, StructOpt)]
#[structopt(about = "A server for wlan_play")]
pub struct ServerOpt {
    /// Listening port
    #[structopt(short, long, default_value = "19198")]
    pub port: u16,
}
