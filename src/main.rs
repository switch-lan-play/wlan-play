pub use anyhow::Result;
use toml::from_slice;
use tokio::fs::read;
use config::Config;
use env_logger::Env;
use connection::run_script;

mod agent;
mod config;
mod connection;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("wlan_play=info")).init();

    let config: Config = from_slice(&read("config.toml").await?)?;

    let mut conn = connection::connect(&config.agent.url).await?;

    if let Some(script) = config.agent.after_connected {
        log::info!("after_connected is set, run script...");
        conn = run_script(conn, script).await?;
    }
    log::info!("Connection is ready");

    let agent = agent::from_connection(conn).await?;

    Ok(())
}
