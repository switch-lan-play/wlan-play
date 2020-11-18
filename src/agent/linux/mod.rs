use anyhow::{anyhow, Context, Result};
use super::{Agent, Executor, Stream, Packet, Device, DeviceType, AsyncStream};
use crate::connection::Connection;
use crate::utils::{pcap_reader::PcapReader, timeout::{TimeoutExt, DEFAULT_TIMEOUT}};
use tokio::{io::BufReader, prelude::*, time::{timeout, sleep, Duration}};
use regex::Regex;
use std::future::Future;
use airnetwork::AirNetwork;

mod airnetwork;

pub struct LinuxAgent<F> {
    conn: LinuxExecutor,
    factory: F,
}

impl<F> LinuxAgent<F>
{
    pub async fn new<Fut>(factory: F) -> Result<LinuxAgent<F>>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output=Result<Connection>> + Send + 'static
    {
        Ok(LinuxAgent {
            conn: LinuxExecutor::from_factory(&factory).await?,
            factory,
        })
    }

    async fn command_match(&mut self, command: &str, re: &str) -> Result<String> {
        let output = self.conn.exec(command).await?;
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
impl<F, Fut> Agent for LinuxAgent<F>
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output=Result<Connection>> + Send + 'static
{
    async fn check(&mut self) -> Result<()> {
        log::trace!("check");
        let tcpdump_version = self.command_match(
            "tcpdump --version 2>&1",
            r"^(tcpdump version .*\nlibpcap version .*\nOpenSSL .*)\n$"
        ).await?;
        let iw_version = self.command_match("iw --version", r"^(iw version .*)\n$").await?;
        let airserv_version = self.command_match("airserv-ng", r"(Airserv-ng\s+.*?)-").await?;
        let nc_version = self.command_match("nc -h 2>&1", r"(OpenBSD netcat.*)\n").await?;
        log::debug!("version check passed:\n{}\n{}\n{}\n{}", tcpdump_version, iw_version, airserv_version, nc_version);

        let mut stream = LinuxExecutor::from_factory(&self.factory)
            .await?
            .exec_stream("echo disconnect".as_bytes())
            .await?;

        let mut buf = String::new();
        timeout(DEFAULT_TIMEOUT, stream.read_to_string(&mut buf))
            .await
            .context(anyhow!("Seems that your terminal doesn't exit when inner process exit. {}",
                "please check your after_connected script."))??;

        Ok(())
    }

    async fn list_device(&mut self) -> Result<Vec<Device>> {
        log::trace!("list_device");
        let re = Regex::new(r"Wiphy\s+(?P<phy>\w+)")?;
        let s = self.conn.exec("iw list").await?;
        let mut out: Vec<Device> = vec![];

        for c in re.captures_iter(&s) {
            let name = c.name("phy").unwrap().as_str().to_string();
            out.push(Device {
                device_type: DeviceType::Phy,
                name,
            });
        }

        let re = Regex::new(r"Interface\s+(?P<dev>\w+)")?;
        let s = self.conn.exec("iw dev").await?;
        for c in re.captures_iter(&s) {
            let name = c.name("dev").unwrap().as_str().to_string();
            out.push(Device {
                device_type: DeviceType::Dev,
                name,
            });
        }

        Ok(out)
    }

    async fn capture_packets(&mut self, device: &Device) -> Result<Box<dyn Stream<Item=Result<Packet>> + Unpin + Send + 'static>> {
        log::trace!("capture_packets");
        match device.device_type {
            DeviceType::Dev => {
                let conn = LinuxExecutor::from_factory(&self.factory).await?;
                let cmd = format!("tcpdump --immediate-mode -l -w - -i {}", device.name);
                log::debug!("cmd {}", cmd);
                let stream = conn.exec_stream(cmd.as_bytes()).await?;
                let reader = PcapReader::new(stream).await?;
                return Ok(Box::new(reader));
            }
            _ => todo!("Device type not supported {:?}", device.device_type)
        }
    }

    async fn send_packets<'a>(&mut self, device: &Device, packets: &'a (dyn Stream<Item=Packet> + Unpin + Send + Sync)) -> Result<()> {
        assert_eq!(device.device_type, DeviceType::Dev);
        let device_name = device.name.clone();
        // kill previous server
        self.conn.exec("killall airserv-ng").await?;

        let serv = LinuxExecutor::from_factory(&self.factory).await?;

        tokio::spawn(async move {
            let cmd = format!("airserv-ng -p 16666 -d {} -v 1 2>&1", device_name);
            let serv_stream = serv.exec_stream(
                cmd.as_bytes()
            ).await?;
            let mut s = BufReader::new(serv_stream);
            loop {
                let mut line = String::new();
                s.read_line(&mut line).await?;
                log::trace!("serv log {}", line.trim_end());
            }
            #[allow(unreachable_code)]
            Ok::<(), anyhow::Error>(())
        });

        sleep(Duration::from_millis(500)).await;

        let conn = LinuxExecutor::from_factory(&self.factory).await?;
        let stream = conn.exec_stream("nc 127.0.0.1 16666".as_bytes()).await?;
        let mut air = AirNetwork::new(stream);
        air.set_channel(11).await?;
        log::trace!("Current channel: {}", air.get_channel().await?);
        log::trace!("Mac: {:x?}", air.get_mac().await?);
        log::trace!("Monitor: {}", air.get_monitor().await?);
        // loop {
        //     let pkt = air.read().await?;
        //     log::trace!("{:#?} {:x?}", &pkt.rx_info, &pkt.data[..10]);
        // }
        todo!()
    }

    fn platform(&self) -> super::Platform {
        super::Platform::Linux
    }
}

pub struct LinuxExecutor(Connection);

impl LinuxExecutor {
    pub fn new(conn: Connection) -> LinuxExecutor {
        LinuxExecutor(conn)
    }
    pub async fn from_factory<F, Fut>(factory: &F) -> Result<LinuxExecutor>
    where
        F: Fn() -> Fut,
        Fut: Future<Output=Result<Connection>>
    {
        Ok(Self::new(factory().await?))
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
}

#[async_trait::async_trait]
impl Executor for LinuxExecutor {
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
        log::trace!("retcode {:?}", retcode);
        // TODO: return retcode in some way

        Ok(result)
    }

    async fn exec_stream(mut self, command: &[u8]) -> Result<Box<dyn AsyncStream + Unpin + Send + 'static>> {
        self.0.write_all("echo '---start---'\n".as_bytes()).await?;
        self.0.flush().await?;
        self.assert_line("---start---").await?;
        self.0.write_all("exec ".as_bytes()).await?;
        self.0.write_all(command).await?;
        self.0.write_all("\n".as_bytes()).await?;
        self.0.flush().await?;

        Ok(Box::new(self.0))
    }
}
