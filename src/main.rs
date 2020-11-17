pub use anyhow::Result;
use toml::from_slice;
use tokio::fs::read;
use config::Config;
use env_logger::Env;
use futures::stream::TryStreamExt;
use remote_device::RemoteDevice;

mod agent;
mod config;
mod connection;
mod utils;
mod remote_device;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("wlan_play=info")).init();

    let config: Config = from_slice(&read("config.toml").await?)?;

    let mut agent = agent::from_config(&config.agent).await?;

    let devices = agent.list_device().await?;
    log::info!("Devices {:#?}", devices);

    let remote = RemoteDevice::new(
        || async { agent::from_config(&config.agent).await },
        config.device.clone()
    ).await?;

    // let mut stream = agent.capture_packets(&Device {
    //     device_type: DeviceType::Dev,
    //     name: config.device,
    // }).await?;

    // while let Some(p) = stream.try_next().await? {
    //     log::trace!("Packet {:x?}", &p.data[..]);
    // }

    // log::info!("devices: {:?}", devices);

    Ok(())
}
