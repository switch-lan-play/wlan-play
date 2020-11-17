pub use anyhow::Result;
use toml::from_slice;
use tokio::fs::read;
use config::Config;
use env_logger::Env;
use connection::run_script;
use agent::{BoxAgent, Device, DeviceType};
use futures::stream::TryStreamExt;

mod agent;
mod config;
mod connection;
mod utils;
mod remote_device;

async fn get_agent(config: &Config) -> Result<BoxAgent> {
    let mut conn = connection::connect(&config.agent.connection).await?;

    if let Some(script) = &config.agent.after_connected {
        log::debug!("after_connected is set, run script...");
        conn = run_script(conn, script.clone()).await?;
    }
    log::debug!("Connection is ready");

    let agent = agent::from_connection(&config.agent.platform, conn).await?;

    Ok(agent)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("wlan_play=info")).init();

    let config: Config = from_slice(&read("config.toml").await?)?;

    let mut agent = get_agent(&config).await?;

    let devices = agent.list_device().await?;
    log::info!("Devices {:#?}", devices);
    let mut stream = agent.capture_packets(&Device {
        device_type: DeviceType::Dev,
        name: config.device,
    }).await?;

    while let Some(p) = stream.try_next().await? {
        log::trace!("Packet {:x?}", &p.data[..]);
    }

    log::info!("devices: {:?}", devices);

    Ok(())
}
