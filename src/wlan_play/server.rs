use anyhow::Result;
use crate::config::ServerOpt;
use tokio::net::UdpSocket;
use std::collections::HashSet;
use std::net::SocketAddr;
use super::protocol::{FrameBody, Frame};
use deku::prelude::*;

pub async fn main(opt: ServerOpt) -> Result<()> {
    let socket = UdpSocket::bind(("0.0.0.0", opt.port)).await?;
    log::info!("Listening on 0.0.0.0:{}", opt.port);
    let mut addrs = HashSet::<SocketAddr>::new();
    let mut buf = [0; 2048];
    loop {
        let (len, addr) = socket.recv_from(&mut buf).await?;
        addrs.insert(addr);
        let buf = &buf[..len];
        let (_, frame) = match Frame::from_bytes((buf, 0)) {
            Ok(f) => f,
            Err(e) => {
                log::error!("{:?} {:02x?}", e, buf);
                continue;
            },
        };
        let broadcast = match frame.body {
            FrameBody::Keepalive => (false),
            FrameBody::Data{..} => (true),
        };
        if broadcast {
            for a in &addrs {
                if a != &addr {
                    socket.send_to(buf, a).await?;
                }
            }
        }
    }
    // Ok(())
}
