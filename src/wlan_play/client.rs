use anyhow::Result;
use crate::config::{Config, Mode};
use crate::agent::{self, Device, DeviceType, BoxAgentDevice};
use crate::utils::ieee80211::{Frame, FrameType, Mac};
use deku::prelude::*;
use tokio::{stream::StreamExt, time::{timeout, Duration}};
use std::collections::HashMap;

pub struct WlanPlay {
    dev: BoxAgentDevice,
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Station {
    pub channel: u32,
    pub mac: Mac,
}

impl WlanPlay {
    pub async fn new(config: &Config) -> Result<Self> {
        let d = Device {
            device_type: DeviceType::Dev,
            name: config.device.clone(),
        };
        let mut agent = agent::from_config(&config.agent).await?;
        let dev = agent.get_device(&d).await?;
        Ok(WlanPlay {
            dev,
        })
    }
    pub async fn find_switch(&mut self) -> Result<HashMap<Mac, Station>> {
        let list = [1u32, 6, 11];
        let mut set = HashMap::new();
        for i in list.iter() {
            log::trace!("Scanning channel {}", i);
            self.dev.set_channel(*i).await?;
            match timeout(
                Duration::from_millis(500),
                self.find_switch_packet(&mut set)).await
            {
                Ok(f) => f?,
                Err(_) => (),
            };
        }
        Ok(set)
    }
    async fn find_switch_packet(&mut self, set: &mut HashMap<Mac, Station>) -> Result<()> {
        while let Some(p) = self.dev.try_next().await? {
            let ((rest, _), frame) = Frame::from_bytes((&p.data, 0))?;
            let (frame_type, sub_type) = (&frame.frame_control.frame_type, &frame.frame_control.sub_type);
            match (frame_type, sub_type) {
                (FrameType::Management, 13) => {
                    // Nintendo action frame
                    if rest[0] == 0x7f && rest[1..4] == [0, 0x22, 0xaa] {
                        // log::trace!("action {} {:x?}, {:x?}", p.channel, frame, &rest[0..4]);
                        // return Ok(Some((p.channel, frame.addr2.unwrap())));
                        let addr = frame.addr2.unwrap();
                        set.insert(addr.clone(), Station {
                            channel: p.channel,
                            mac: addr
                        });
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    async fn set_station(&mut self, station: Station) -> Result<()> {
        self.dev.set_channel(station.channel).await?;
        self.dev.set_filter(Some(Box::new(move |p| {
            let (_, frame) = match Frame::from_bytes((&p.data, 0)) {
                Ok(r) => r,
                Err(_) => return true,
            };
            if station.mac == frame.addr1 {
                return false
            }
            if let Some(true) = frame.addr2.map(|k| station.mac == k) {
                return false
            }
            if let Some(true) = frame.addr3.map(|k| station.mac == k) {
                return false
            }
            true
        }))).await?;
        Ok(())
    }
}

pub async fn main(config: Config) -> Result<()> {
    let mut wlan_play = WlanPlay::new(&config).await?;

    match config.mode {
        Mode::Host => {
            let ns = wlan_play.find_switch().await?;
            if ns.len() == 0 {
                log::info!("NS not found");
                return Ok(())
            }
            log::info!("Found NS: {:#?}", ns);
            let sta = ns.values().next().unwrap();
            wlan_play.set_station(sta.clone()).await?;
            
            while let Some(p) = wlan_play.dev.try_next().await? {
                log::trace!("p {:02x?}", &p.data[0..8]);
            }
        },
        Mode::Station => {
            todo!()
        }
    };

    Ok(())
}