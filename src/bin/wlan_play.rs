use anyhow::Result;
use toml::from_slice;
use tokio::fs::read;
use wlan_play::config::Config;
use env_logger::Env;
use wlan_play::client::main as client_main;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("wlan_play=info")).init();

    let config: Config = from_slice(&read("config.toml").await?)?;

    client_main(config).await
}
