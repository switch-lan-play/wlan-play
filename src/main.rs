pub use anyhow::Result;
use toml::from_slice;
use tokio::fs::read;
use config::Config;
use env_logger::Env;
use connection::run_script;

mod agent;
mod config;
mod connection;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("wlan_play=info")).init();

    let config: Config = from_slice(&read("config.toml").await?)?;

    let mut conn = connection::connect(config.agent.connection).await?;

    if let Some(script) = &config.agent.after_connected {
        log::info!("after_connected is set, run script...");
        conn = run_script(conn, script.clone()).await?;
    }
    log::info!("Connection is ready");

    let mut agent = agent::from_connection(config.agent.platform, conn).await?;

    let devices = agent.list_device().await?;

    log::info!("devices: {:?}", devices);

    Ok(())
}
