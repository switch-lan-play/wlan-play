use anyhow::Result;
use wlan_play::config::ServerOpt;
use structopt::StructOpt;
use env_logger::Env;
use wlan_play::server::main as server_main;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("wlan_play=info")).init();
    let opt = ServerOpt::from_args();

    server_main(opt).await
}
