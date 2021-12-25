use anyhow::Result;
use env_logger::Env;
use structopt::StructOpt;
use wlan_play::client::main as client_main;
use wlan_play::config::ClientOpt;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("wlan_play=trace")).init();
    let opt = ClientOpt::from_args();

    client_main(opt).await
}
