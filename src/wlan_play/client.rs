use super::protocol;
use crate::agent::{self, BoxAgentDevice, Device, DeviceType, Packet};
use crate::config::{ClientOpt, Config, Mode};
use crate::utils::ieee80211::{self, Frame, FrameType, Mac};
use anyhow::{anyhow, Result};
use deku::prelude::*;
use futures::stream::TryStreamExt;
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    path::PathBuf,
};
use tokio::select;
use tokio::{
    net::UdpSocket,
    time::{timeout, Duration},
};

fn parse_ieee80211(data: &[u8]) -> Result<(ieee80211::Frame, &[u8])> {
    let ((body, _), frame) = match Frame::from_bytes((data, 0)) {
        Ok(r) => r,
        Err(e) => {
            log::trace!("IEEE80211 parse {:02x?}", data);
            return Err(e.into());
        }
    };
    Ok((frame, body))
}

pub struct WlanPlay {
    dev: BoxAgentDevice,
    _pcap: Option<PathBuf>,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Station {
    pub channel: u32,
    pub mac: Mac,
}

impl WlanPlay {
    pub async fn new(config: &Config, pcap: Option<PathBuf>) -> Result<Self> {
        let d = Device {
            device_type: DeviceType::Dev,
            name: config.device.clone(),
        };
        let mut agent = agent::from_config(&config.agent).await?;
        let dev = agent.get_device(&d).await?;
        Ok(WlanPlay { dev, _pcap: pcap })
    }
    pub async fn find_switch(&mut self) -> Result<HashMap<Mac, Station>> {
        let list = [1u32, 6, 11];
        let mut set = HashMap::new();
        for i in list.iter() {
            log::trace!("Scanning channel {}", i);
            self.dev.set_channel(*i).await?;
            let mut count = 0;
            match timeout(
                Duration::from_millis(300),
                self.find_switch_packet(&mut set, &mut count),
            )
            .await
            {
                Ok(f) => f?,
                Err(_) => {
                    log::trace!("Channel {} stop, packets: {}", i, count);
                }
            };
        }
        Ok(set)
    }
    async fn find_switch_packet(
        &mut self,
        set: &mut HashMap<Mac, Station>,
        count: &mut u32,
    ) -> Result<()> {
        while let Some(p) = self.dev.try_next().await? {
            *count += 1;
            let (frame, _) = parse_ieee80211(&p.data)?;
            // Nintendo action frame
            if get_action_ssid(&p.data).is_some() {
                let addr = frame.addr2.unwrap();
                set.insert(
                    addr.clone(),
                    Station {
                        channel: p.channel,
                        mac: addr,
                    },
                );
            }
        }
        Ok(())
    }
    async fn set_station(&mut self, station: Station) -> Result<()> {
        self.dev.set_channel(station.channel).await?;
        self.dev
            .set_filter(Some(Box::new(move |p| {
                let (frame, _) = match parse_ieee80211(&p.data) {
                    Ok(r) => r,
                    Err(_) => return true,
                };
                if packet_has_mac(&frame, &station.mac) {
                    return false;
                }
                true
            })))
            .await?;
        Ok(())
    }
}

fn packet_has_mac(frame: &ieee80211::Frame, mac: &Mac) -> bool {
    if mac == &frame.addr1 {
        return true;
    }
    if let Some(true) = frame.addr2.as_ref().map(|k| mac == k) {
        return true;
    }
    if let Some(true) = frame.addr3.as_ref().map(|k| mac == k) {
        return true;
    }
    false
}

struct Client {
    s: UdpSocket,
}

impl Client {
    async fn connect(addr: SocketAddr) -> Result<Client> {
        let s = UdpSocket::bind("0.0.0.0:0").await?;
        s.connect(addr).await?;
        Ok(Client { s })
    }
    async fn recv(&self) -> Result<protocol::FrameBody> {
        let mut buf = [0u8; 2048];
        let len = self.s.recv(&mut buf).await?;
        let buf = &buf[..len];
        let (_, frame) = protocol::Frame::from_bytes((buf, 0))?;
        Ok(frame.body)
    }
    async fn send(&self, frame: protocol::FrameBody) -> Result<()> {
        let frame: protocol::Frame = frame.into();
        let bytes = frame.to_bytes()?;
        self.s.send(&bytes).await?;
        Ok(())
    }
}

fn get_action_ssid(data: &[u8]) -> Option<String> {
    let (frame, rest) = parse_ieee80211(data).ok()?;
    let (frame_type, sub_type) = (
        &frame.frame_control.frame_type,
        &frame.frame_control.sub_type,
    );

    if let (FrameType::Management, 13) = (frame_type, sub_type) {
        // Nintendo action frame
        if rest[0] == 0x7f
            && rest[1..4] == [0x00, 0x22, 0xaa]
            && rest[4..12] == [0x04, 0x00, 0x01, 0x01, 0, 0, 0, 0]
        {
            return Some(hex::encode(&rest[28..28 + 0x10]));
        }
    }
    None
}

fn get_probe_ssid(data: &[u8]) -> Option<String> {
    let (frame, rest) = parse_ieee80211(data).ok()?;
    let (frame_type, sub_type) = (
        &frame.frame_control.frame_type,
        &frame.frame_control.sub_type,
    );

    // Probe request
    if let (FrameType::Management, 4) = (frame_type, sub_type) {
        // SSID
        if rest[0..2] == [0x00, 0x20] {
            return Some(String::from_utf8_lossy(&rest[2..2 + 0x20]).to_string());
        }
    }
    None
}

async fn host_main(client: Client, mut wlan_play: WlanPlay) -> Result<()> {
    use protocol::FrameBody;
    let ns = loop {
        let ns = wlan_play.find_switch().await?;
        if !ns.is_empty() {
            break ns;
        }
    };
    log::info!("Found NS: {:#?}", ns);
    let sta = ns.values().next().unwrap();
    wlan_play.set_station(sta.clone()).await?;

    loop {
        select! {
            cr = client.recv() => {
                match cr? {
                    FrameBody::Keepalive => {}
                    FrameBody::Data { channel, data } => {
                        wlan_play.dev.send(Packet {
                            channel,
                            data,
                        }).await?;
                    }
                };
            }
            dr = wlan_play.dev.try_next() => {
                let p = dr?.ok_or(anyhow!("Device stopped"))?;
                client.send(protocol::FrameBody::Data {
                    channel: p.channel,
                    data: p.data,
                }).await?;
            }
        }
    }
}

async fn station_main(client: Client, mut wlan_play: WlanPlay) -> Result<()> {
    use protocol::FrameBody;
    client.send(FrameBody::Keepalive).await?;

    let mut channel_has_set = false;
    let mut ssids = HashSet::<String>::new();
    let mut stations = HashSet::<Mac>::new();

    loop {
        select! {
            cr = client.recv() => {
                match cr? {
                    FrameBody::Keepalive => {}
                    FrameBody::Data { channel, data } => {
                        if !channel_has_set {
                            log::info!("Set channel to {}", channel);
                            wlan_play.dev.set_channel(channel).await?;
                            channel_has_set = true;
                        }
                        if let Some(ssid) = get_action_ssid(&data) {
                            ssids.insert(ssid);
                        }
                        wlan_play.dev.send(Packet {
                            channel,
                            data,
                        }).await?;
                    }
                };
            }
            dr = wlan_play.dev.try_next() => {
                let p = dr?.ok_or(anyhow!("Device stopped"))?;
                let (frame, _) = parse_ieee80211(&p.data)?;
                if let Some(true) = get_probe_ssid(&p.data).map(|ssid| ssids.contains(&ssid)) {
                    stations.insert(frame.addr2.as_ref().unwrap().clone());
                }
                if let Some(true) = frame.addr2.as_ref().map(|src| stations.contains(src)) {
                    client.send(protocol::FrameBody::Data {
                        channel: p.channel,
                        data: p.data,
                    }).await?;
                }
            }
        };
    }

    // log::error!("Failed to recv from server");
    // Ok(())
}

pub async fn main(opt: ClientOpt) -> Result<()> {
    use tokio::fs::read;
    use toml::from_slice;

    let config: Config = from_slice(&read(opt.cfg).await?)?;
    let wlan_play = WlanPlay::new(&config, opt.pcap).await?;
    let client = Client::connect(config.server).await?;

    match config.mode {
        Mode::Host => {
            host_main(client, wlan_play).await?;
        }
        Mode::Station => {
            station_main(client, wlan_play).await?;
        }
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_action_ssid() {
        let data = [
            0xD0u8, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x04, 0x03, 0xD6, 0x28,
            0xA3, 0xAC, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x40, 0x2E, 0x7F, 0x00, 0x22, 0xAA,
            0x04, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x10, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x29, 0xA6, 0x4B, 0x95,
            0x8B, 0x63, 0xD3, 0xE6, 0x7E, 0x83, 0x84, 0x88, 0x3F, 0x02, 0x4F, 0x76,
        ];
        assert_eq!(
            get_action_ssid(&data).unwrap(),
            "29a64b958b63d3e67e8384883f024f76"
        );
    }

    #[test]
    fn test_get_probe_ssid() {
        let data = [
            0x40u8, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x98, 0x41, 0x5C, 0xDC,
            0x22, 0xEC, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x10, 0x03, 0x00, 0x20, 0x32, 0x39,
            0x61, 0x36, 0x34, 0x62, 0x39, 0x35, 0x38, 0x62, 0x36, 0x33, 0x64, 0x33, 0x65, 0x36,
            0x37, 0x65, 0x38, 0x33, 0x38, 0x34, 0x38, 0x38, 0x33, 0x66, 0x30, 0x32, 0x34, 0x66,
            0x37, 0x36,
        ];
        assert_eq!(
            get_probe_ssid(&data).unwrap(),
            "29a64b958b63d3e67e8384883f024f76"
        );
    }
}
