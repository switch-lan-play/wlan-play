pub use anyhow::Result;
use toml::from_slice;
use tokio::fs::read;
use config::Config;
use env_logger::Env;

mod agent;
mod config;
mod connection;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("wlan_play=info")).init();

    let config: Config = from_slice(&read("config.toml").await?)?;

    // log::info!("{:#?}", config);
    let conn = connection::connect(&config.agent.url).await?;
    let agent = agent::from_connection(conn).await?;

    Ok(())
}
