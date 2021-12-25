use crate::agent::AgentConfig;
use serde_derive::Deserialize;
use std::net::SocketAddr;
use std::path::PathBuf;
use structopt::StructOpt;

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
#[structopt(about = "wlan_play client")]
pub struct ClientOpt {
    /// Listening port
    #[structopt(short, long, default_value = "config.toml", parse(from_os_str))]
    pub cfg: PathBuf,

    /// Write packets to pcap file
    #[structopt(short, long, parse(from_os_str))]
    pub pcap: Option<PathBuf>,
}

#[derive(Debug, StructOpt)]
#[structopt(about = "A server for wlan_play")]
pub struct ServerOpt {
    /// Listening port
    #[structopt(short, long, default_value = "19198")]
    pub port: u16,
}
