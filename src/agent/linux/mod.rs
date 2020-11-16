use anyhow::{anyhow, Result};
use super::{Agent, Executor, BoxAgent, Connection, Stream, Packet, Device, DeviceType};
use crate::utils::{TimeoutExt, DEFAULT_TIMEOUT};
use tokio::prelude::*;
use regex::Regex;

pub struct LinuxAgent(Connection);

impl LinuxAgent {
    async fn read_line(&mut self) -> Result<String> {
        let mut s = self.0.read_line_timeout(DEFAULT_TIMEOUT).await?;
        s.pop();
        Ok(s)
    }
    async fn assert_line(&mut self, expect: &str) -> Result<()> {
        if expect != &self.read_line().await? {
            return Err(anyhow!("Failed to assert_line"));
        }

        Ok(())
    }
    async fn command_match(&mut self, command: &str, re: &str) -> Result<String> {
        let output = self.exec(command).await?;
        if !Regex::new(re)?.is_match(&output) {
            log::debug!("cmd: {:?}\noutput: {:?}", command, output);
            Err(anyhow!("The output of command: {} is not matched with regex: {}", command, re))?;
        }
        Ok(output)
    }
}

#[async_trait::async_trait]
impl Executor for LinuxAgent {
    async fn exec_bytes(&mut self, command: &[u8]) -> Result<Vec<u8>> {
        self.0.write_all("echo '---start---'\n".as_bytes()).await?;
        self.0.write_all(command).await?;
        self.0.write_all("\necho '---end---'\n".as_bytes()).await?;
        self.0.flush().await?;

        self.assert_line("---start---").await?;
        let result = self.0.read_until_timeout(DEFAULT_TIMEOUT, &b"---end---\n"[..]).await?;

        Ok(result)
    }

    async fn exec_stream(&mut self, _command: &[u8]) -> Result<Box<dyn AsyncRead>> {
        todo!()
    }
}

#[async_trait::async_trait]
impl Agent for LinuxAgent {
    async fn check(&mut self) -> Result<()> {
        log::trace!("check");
        let tcpdump_version = self.command_match(
            "tcpdump --version 2>&1",
            r"^tcpdump version .*\nlibpcap version .*\nOpenSSL .*\n$"
        ).await?;
        let iw_version = self.command_match("iw --version", r"^iw version .*\n$").await?;
        log::info!("check passed:\n{}{}", tcpdump_version, iw_version.trim_end());
        Ok(())
    }

    async fn list_device(&mut self) -> Result<Vec<Device>> {
        log::trace!("list_device");
        let re = Regex::new(r"Wiphy\s+(?P<phy>\w+)")?;
        let s = self.exec("iw list").await?;
        let mut out: Vec<Device> = vec![];

        for c in re.captures_iter(&s) {
            let name = c.name("phy").unwrap().as_str().to_string();
            out.push(Device {
                device_type: DeviceType::Phy,
                name,
            });
        }

        let re = Regex::new(r"Interface\s+(?P<dev>\w+)")?;
        let s = self.exec("iw dev").await?;
        for c in re.captures_iter(&s) {
            let name = c.name("dev").unwrap().as_str().to_string();
            out.push(Device {
                device_type: DeviceType::Dev,
                name,
            });
        }

        Ok(out)
    }

    async fn capture_packets(&mut self, device: &Device) -> Result<Box<dyn Stream<Item=Packet>>> {
        log::trace!("capture_packets");
        match device.device_type {
            DeviceType::Dev => {
                let cmd = format!("tcpdump --immediate-mode -l -w - -i {}", device.name);
                log::info!("cmd {}", cmd);
                // let stream = self.exec_stream(cmd.as_bytes()).await?;
            }
            _ => todo!("Device type not supported {:?}", device.device_type)
        }
  
        todo!()
    }
}

pub async fn from_connection(conn: Connection) -> Result<BoxAgent> {
    Ok(Box::new(LinuxAgent(conn)))
}
