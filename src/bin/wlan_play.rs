use anyhow::Result;
use wlan_play::config::ClientOpt;
use env_logger::Env;
use wlan_play::client::main as client_main;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("wlan_play=info")).init();
    let opt = ClientOpt::from_args();

    client_main(opt).await
}
