use super::{
    Agent, AgentDevice, AsyncStream, Device, DeviceType, Executor, Filter, Packet, Stream,
};
use crate::connection::Connection;
use crate::utils::timeout::{TimeoutExt, DEFAULT_TIMEOUT};
use airnetwork::{AirNetwork, TxPacket};
use anyhow::{anyhow, Context as _, Result};
use futures::{pin_mut, ready};
use regex::Regex;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    time::{sleep, timeout, Duration},
};

mod airnetwork;

pub struct LinuxAgentDevice<S> {
    c: AirNetwork<S>,
    name: String,
    filter: Option<Filter>,
}

impl<S> LinuxAgentDevice<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn new(name: String, s: S) -> LinuxAgentDevice<S> {
        LinuxAgentDevice {
            c: AirNetwork::new(s),
            name,
            filter: None,
        }
    }
}

impl<S> Stream for LinuxAgentDevice<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    type Item = Result<Packet>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let p = {
                let fut = self.c.read();
                pin_mut!(fut);
                ready!(fut.poll(cx))?
            };
            let pkt = Packet {
                channel: p.rx_info.channel,
                data: p.data,
            };
            if let Some(true) = self.filter.as_ref().map(|f| f(&pkt)) {
                // drop
                continue;
            }
            return Poll::Ready(Some(Ok(pkt)));
        }
    }
}

#[async_trait::async_trait]
impl<S> AgentDevice for LinuxAgentDevice<S>
where
    S: AsyncRead + AsyncWrite + Unpin + Send,
{
    async fn set_channel(&mut self, channel: u32) -> Result<()> {
        self.c.set_channel(channel).await?;
        Ok(())
    }

    async fn get_channel(&mut self) -> Result<Option<u32>> {
        Ok(match self.c.get_channel().await? {
            -1 => None,
            c => Some(c as u32),
        })
    }

    async fn send(&mut self, packet: Packet) -> Result<()> {
        let pkt_len = packet.data.len();
        let written = self
            .c
            .write(TxPacket {
                data: packet.data,
                ..Default::default()
            })
            .await?;
        if written != pkt_len {
            log::warn!(
                "send it not successed, sent: {}, packet: {}",
                written,
                pkt_len
            );
        }
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    async fn set_filter(&mut self, filter: Option<super::Filter>) -> Result<Option<super::Filter>> {
        Ok(std::mem::replace(&mut self.filter, filter))
    }
}

pub struct LinuxAgent<F> {
    conn: LinuxExecutor,
    factory: F,
}

impl<F> LinuxAgent<F> {
    pub async fn new<Fut>(factory: F) -> Result<LinuxAgent<F>>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: Future<Output = Result<Connection>> + Send + 'static,
    {
        Ok(LinuxAgent {
            conn: LinuxExecutor::from_factory(&factory).await?,
            factory,
        })
    }

    async fn command_match(&mut self, command: &str, re: &str) -> Result<String> {
        let output = self.conn.exec(command).await?;
        match Regex::new(re)?.captures(&output) {
            Some(output) => Ok(output
                .get(1)
                .expect("Make sure there is a group in your RE")
                .as_str()
                .to_owned()),
            None => {
                log::debug!("cmd: {:?}\noutput: {:?}", command, output);
                Err(anyhow!(
                    "The output of command: {} is not matched with regex: {}",
                    command,
                    re
                ))
            }
        }
    }
}

#[async_trait::async_trait]
impl<F, Fut> Agent for LinuxAgent<F>
where
    F: Fn() -> Fut + Send + Sync,
    Fut: Future<Output = Result<Connection>> + Send + 'static,
{
    async fn check(&mut self) -> Result<()> {
        log::trace!("check");
        // let iw_version = self.command_match("iw --version", r"^(iw version .*)\n$").await?;
        let airserv_version = self
            .command_match("airserv-ng", r"(Airserv-ng\s+.*?)-")
            .await?;
        let nc_version = self
            .command_match(
                "nc -h 2>&1",
                r"((OpenBSD netcat.*)|(GNU netcat .*)|(BusyBox.*))\n",
            )
            .await?;
        log::debug!("version check passed:\n{}\n{}", airserv_version, nc_version);

        let mut stream = LinuxExecutor::from_factory(&self.factory)
            .await?
            .exec_stream("echo disconnect".as_bytes())
            .await?;

        let mut buf = String::new();
        timeout(DEFAULT_TIMEOUT, stream.read_to_string(&mut buf))
            .await
            .context(anyhow!(
                "Seems that your terminal doesn't exit when inner process exit. {}",
                "please check your after_connected script."
            ))??;

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

    fn platform(&self) -> super::Platform {
        super::Platform::Linux
    }

    async fn get_device(&mut self, device: &Device) -> Result<super::BoxAgentDevice> {
        assert_eq!(device.device_type, DeviceType::Dev);
        let device_name = device.name.clone();
        // kill previous server
        self.conn.exec("killall airserv-ng").await?;

        let serv = LinuxExecutor::from_factory(&self.factory).await?;

        tokio::spawn(async move {
            let cmd = format!("airserv-ng -p 16666 -d {} -v 1 2>&1", device_name);
            let serv_stream = serv.exec_stream(cmd.as_bytes()).await?;
            let mut s = BufReader::new(serv_stream);
            loop {
                let mut line = String::new();
                s.read_line(&mut line).await?;
                if line.is_empty() {
                    break;
                }
                log::trace!("airserv-ng: {}", line.trim_end());
            }
            log::error!("airserv-ng exited");
            Ok::<(), anyhow::Error>(())
        });

        sleep(Duration::from_millis(500)).await;

        let conn = LinuxExecutor::from_factory(&self.factory).await?;
        let stream = conn.exec_stream("nc 127.0.0.1 16666".as_bytes()).await?;

        Ok(Box::new(LinuxAgentDevice::new(device.name.clone(), stream)))
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
        Fut: Future<Output = Result<Connection>>,
    {
        Ok(Self::new(factory().await?))
    }
    async fn read_line(&mut self) -> Result<String> {
        let mut s = self.0.read_line_timeout(DEFAULT_TIMEOUT).await?;
        s.pop();
        Ok(s)
    }
    async fn assert_line(&mut self, expect: &str) -> Result<()> {
        if expect != self.read_line().await? {
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
        let result = self
            .0
            .read_until_timeout(DEFAULT_TIMEOUT, &b"---end---\n"[..])
            .await?;
        let retcode = String::from_utf8(
            self.0
                .read_until_timeout(DEFAULT_TIMEOUT, &b"\n"[..])
                .await?,
        )?;
        let retcode = retcode.trim().parse::<u8>()?;
        if retcode != 0 {
            // TODO: return retcode in some way
            log::warn!("{} retcode {:?}", String::from_utf8_lossy(command), retcode);
        }

        Ok(result)
    }

    async fn exec_stream(
        mut self,
        command: &[u8],
    ) -> Result<Box<dyn AsyncStream + Unpin + Send + 'static>> {
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
