use anyhow::{anyhow, Result};
use super::{Agent, Executor, Connection, Stream, Packet, Device, DeviceType};
use crate::utils::{pcap_reader::PcapReader, timeout::{TimeoutExt, DEFAULT_TIMEOUT}};
use tokio::prelude::*;
use regex::Regex;

pub struct LinuxAgent(Connection);

impl LinuxAgent {
    pub async fn new(conn: Connection) -> Result<LinuxAgent> {
        Ok(LinuxAgent(conn))
    }
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
        match Regex::new(re)?.captures(&output) {
            Some(output) => {
                Ok(output.get(1).expect("Make sure there is a group in your RE").as_str().to_owned())
            }
            None => {
                log::debug!("cmd: {:?}\noutput: {:?}", command, output);
                Err(anyhow!("The output of command: {} is not matched with regex: {}", command, re))
            }
        }
    }
}

#[async_trait::async_trait]
impl Executor for LinuxAgent {
    async fn exec_bytes(&mut self, command: &[u8]) -> Result<Vec<u8>> {
        self.0.write_all("echo '---start---'\n".as_bytes()).await?;
        self.0.write_all(command).await?;
        self.0.write_all("\necho '---end---'\n".as_bytes()).await?;
        self.0.write_all("\necho $?\n".as_bytes()).await?;
        self.0.flush().await?;

        self.assert_line("---start---").await?;
        let result = self.0.read_until_timeout(DEFAULT_TIMEOUT, &b"---end---\n"[..]).await?;
        let retcode = String::from_utf8(self.0.read_until_timeout(DEFAULT_TIMEOUT, &b"\n"[..]).await?)?;
        let retcode = retcode.trim().parse::<u8>()?;
        log::debug!("retcode {:?}", retcode);
        // TODO: return retcode in some way

        Ok(result)
    }

    async fn exec_stream<'a>(&'a mut self, command: &[u8]) -> Result<Box<dyn AsyncRead + Unpin + Send + 'a>> {
        self.0.write_all("echo '---start---'\n".as_bytes()).await?;
        self.0.flush().await?;
        self.assert_line("---start---").await?;
        self.0.write_all(command).await?;
        self.0.write_all("\necho '---end---'\n".as_bytes()).await?;
        self.0.flush().await?;

        // TODO: stop at end
        Ok(Box::new(&mut self.0))
    }
}

#[async_trait::async_trait]
impl Agent for LinuxAgent {
    async fn check(&mut self) -> Result<()> {
        log::trace!("check");
        let tcpdump_version = self.command_match(
            "tcpdump --version 2>&1",
            r"^(tcpdump version .*\nlibpcap version .*\nOpenSSL .*\n)$"
        ).await?;
        let iw_version = self.command_match("iw --version", r"^(iw version .*\n)$").await?;
        let airserv_version = self.command_match("airserv-ng", r"(Airserv-ng\s+.*?)-").await?;
        log::debug!("check passed:\n{}{}{}\n", tcpdump_version, iw_version, airserv_version);
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

    async fn capture_packets<'a>(&'a mut self, device: &Device) -> Result<Box<dyn Stream<Item=Result<Packet>> + Unpin + Send + 'a>> {
        log::trace!("capture_packets");
        match device.device_type {
            DeviceType::Dev => {
                let cmd = format!("tcpdump --immediate-mode -l -w - -i {}", device.name);
                log::debug!("cmd {}", cmd);
                let stream = self.exec_stream(cmd.as_bytes()).await?;
                let reader = PcapReader::new(stream).await?;
                return Ok(Box::new(reader));
            }
            _ => todo!("Device type not supported {:?}", device.device_type)
        }
    }

    fn platform(&self) -> super::Platform {
        super::Platform::Linux
    }
}

