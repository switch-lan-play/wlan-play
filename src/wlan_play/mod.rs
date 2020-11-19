use anyhow::Result;
use crate::config::Config;
use crate::agent::{self, Device, DeviceType, BoxAgentDevice};
use crate::utils::ieee80211::{Frame, FrameType};
use deku::prelude::*;
use tokio::{stream::StreamExt, time::{timeout, Duration}};
use std::collections::HashSet;

pub struct WlanPlay {
    dev: BoxAgentDevice,
}

type Sta = (u32, [u8; 6]);

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
    pub async fn find_switch(&mut self) -> Result<HashSet::<Sta>> {
        let list = [1u32, 6, 11, 1, 6, 11];
        let mut set = HashSet::<Sta>::new();
        for i in list.iter() {
            log::trace!("Scanning channel {}", i);
            self.dev.set_channel(*i).await?;
            match timeout(
                Duration::from_secs(3),
                self.find_switch_packet()).await
            {
                Ok(f) => {
                    if let Some(s) = f? {
                        set.insert(s);
                    }
                },
                Err(_) => {},
            };
        }
        Ok(set)
    }
    async fn find_switch_packet(&mut self) -> Result<Option<Sta>> {
        while let Some(p) = self.dev.try_next().await? {
            let ((rest, _), frame) = Frame::from_bytes((&p.data, 0))?;
            let (frame_type, sub_type) = (&frame.frame_control.frame_type, &frame.frame_control.sub_type);
            match (frame_type, sub_type) {
                (FrameType::Management, 13) => {
                    // Nintendo action frame
                    if rest[0] == 0x7f && rest[1..4] == [0, 0x22, 0xaa] {
                        // log::trace!("action {} {:x?}, {:x?}", p.channel, frame, &rest[0..4]);
                        return Ok(Some((p.channel, frame.addr2.unwrap())));
                    }
                }
                _ => {}
            }
        }
        Ok(None)
    }
}
