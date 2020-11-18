pub use anyhow::Result;
use toml::from_slice;
use tokio::fs::read;
use config::Config;
use env_logger::Env;

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

    // let remote = remote_device::RemoteDevice::new(
    //     || async { agent::from_config(&config.agent).await },
    //     config.device.clone()
    // ).await?;

    // let mut stream = agent.capture_packets(&agent::Device {
    //     device_type: agent::DeviceType::Dev,
    //     name: config.device,
    // }).await?;
    // use futures::stream::TryStreamExt;
    // while let Some(p) = stream.try_next().await? {
    //     log::trace!("Packet {:x?}", &p.data[..]);
    // }

    use tokio::time;
    use futures::stream::StreamExt;
    let a = time::interval(time::Duration::from_secs(1))
        .map(|_| utils::Packet {
            data: vec![0, 1, 2, 3]
        });
    let d = agent::Device {
        device_type: agent::DeviceType::Dev,
        name: config.device,
    };
    agent.send_packets(&d, &a).await?;

    Ok(())
}
