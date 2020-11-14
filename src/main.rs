use anyhow::Result;
use toml::from_slice;
use tokio::fs::read;
use config::Config;
use env_logger::Env;

mod config;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("wlan_play=info")).init();

    let config: Config = from_slice(&read("config.toml").await?)?;

    log::info!("{:#?}", config);

    Ok(())
}
