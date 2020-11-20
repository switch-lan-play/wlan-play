use anyhow::Result;
use crate::config::ServerOpt;
use tokio::net::UdpSocket;
use std::collections::HashSet;
use std::net::SocketAddr;

pub async fn main(opt: ServerOpt) -> Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", opt.port)).await?;
    log::info!("Listening on 0.0.0.0:{}", opt.port);
    let addrs = HashSet::<SocketAddr>::new();
    let mut buf = [0; 2048];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        let buf = &buf[..len];
        for a in &addrs {
            if a != &addr {
                socket.send_to(buf, a).await?;
            }
        }
    }
    // Ok(())
}
