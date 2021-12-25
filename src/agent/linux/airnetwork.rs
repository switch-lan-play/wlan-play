#![allow(dead_code)]

use deku::prelude::*;
use protocol::*;
pub use protocol::{RxInfo, RxPacket, TxInfo, TxPacket};
use std::{collections::VecDeque, io};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const HEADER_LEN: usize = 1 + 4;

fn other<E: std::error::Error + Send + Sync + 'static>(err: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

fn get_rc(cmd: NetCmd) -> io::Result<u32> {
    match cmd {
        NetCmd::Rc(rc) => Ok(rc),
        _ => Err(io::ErrorKind::InvalidData.into()),
    }
}

fn rc(cmd: NetCmd) -> io::Result<()> {
    if get_rc(cmd)? == 0 {
        Ok(())
    } else {
        Err(io::ErrorKind::InvalidData.into())
    }
}

/// A client to communicate with airserv-ng
///
/// Reference: https://github.com/aircrack-ng/aircrack-ng/blob/565870292e210010dea65ab4f289fc5ff392bd45/lib/osdep/network.c
pub struct AirNetwork<S> {
    s: S,
    queue: VecDeque<RxPacket>,
}

impl<S> AirNetwork<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(stream: S) -> AirNetwork<S> {
        AirNetwork {
            s: stream,
            queue: VecDeque::new(),
        }
    }
    async fn cmd(&mut self, cmd: NetCmd) -> io::Result<()> {
        let frame: NetCmdFrame = cmd.into();
        let bytes = frame.to_bytes().unwrap();

        self.s.write_all(&bytes).await?;
        self.s.flush().await
    }
    async fn get(&mut self) -> io::Result<NetCmd> {
        let mut buf = vec![0u8; HEADER_LEN];
        self.s.read_exact(&mut buf).await?;
        let (_, header) = NetCmdHeader::from_bytes((&buf, 0)).map_err(other)?;

        buf.resize(buf.len() + header.len as usize, 0);
        self.s.read_exact(&mut buf[HEADER_LEN..]).await?;

        let (_, frame) = NetCmdFrame::from_bytes((&buf, 0)).map_err(other)?;

        Ok(frame.body)
    }
    async fn get_no_packet(&mut self) -> io::Result<NetCmd> {
        loop {
            let p = match self.get().await? {
                NetCmd::Packet(p) => p,
                p => return Ok(p),
            };
            self.queue.push_back(p);
        }
    }
    pub async fn read(&mut self) -> io::Result<RxPacket> {
        if let Some(i) = self.queue.pop_front() {
            return Ok(i);
        }

        let resp = self.get().await?;
        match resp {
            NetCmd::Packet(p) => Ok(p),
            _ => Err(io::ErrorKind::InvalidData.into()),
        }
    }
    pub async fn write(&mut self, data: TxPacket) -> io::Result<usize> {
        self.cmd(NetCmd::Write(data)).await?;
        let rc = get_rc(self.get_no_packet().await?)?;
        Ok(rc as usize)
    }
    pub async fn set_channel(&mut self, channel: u32) -> io::Result<()> {
        self.cmd(NetCmd::SetChan(channel)).await?;
        rc(self.get_no_packet().await?)
    }
    pub async fn get_channel(&mut self) -> io::Result<i32> {
        self.cmd(NetCmd::GetChan).await?;
        Ok(get_rc(self.get_no_packet().await?)? as i32)
    }
    pub async fn set_rate(&mut self, rate: u32) -> io::Result<()> {
        self.cmd(NetCmd::SetRate(rate)).await?;
        rc(self.get_no_packet().await?)
    }
    pub async fn get_rate(&mut self) -> io::Result<u32> {
        self.cmd(NetCmd::GetRate).await?;
        Ok(get_rc(self.get_no_packet().await?)?)
    }
    pub async fn get_mac(&mut self) -> io::Result<[u8; 6]> {
        self.cmd(NetCmd::GetMac).await?;
        match self.get_no_packet().await? {
            NetCmd::Mac(mac) => Ok(mac),
            _ => Err(io::ErrorKind::InvalidData.into()),
        }
    }
    pub async fn get_monitor(&mut self) -> io::Result<u32> {
        self.cmd(NetCmd::GetRate).await?;
        Ok(get_rc(self.get_no_packet().await?)?)
    }
}

mod protocol {
    use deku::ctx::Endian;
    use deku::prelude::*;
    use std::mem::size_of;

    #[derive(Debug, DekuRead, DekuWrite, PartialEq)]
    #[deku(endian = "big")]
    #[deku(ctx = "_endian: Endian")]
    pub struct RxInfo {
        pub machine: u64,
        pub power: i32,
        pub noise: i32,
        pub channel: u32,
        pub freq: u32,
        pub rate: u32,
        pub antenna: u32,
    }

    #[derive(Debug, DekuRead, DekuWrite, PartialEq, Default)]
    #[deku(endian = "big")]
    #[deku(ctx = "_endian: Endian")]
    pub struct TxInfo {
        pub rate: u32,
    }

    #[derive(Debug, DekuRead, DekuWrite, PartialEq)]
    #[deku(endian = "big")]
    #[deku(ctx = "_endian: Endian, len: u32")]
    pub struct RxPacket {
        pub rx_info: RxInfo,
        #[deku(count = "len as usize - size_of::<RxInfo>()")]
        pub data: Vec<u8>,
    }

    #[derive(Debug, DekuRead, DekuWrite, PartialEq, Default)]
    #[deku(endian = "big")]
    #[deku(ctx = "_endian: Endian, len: u32")]
    pub struct TxPacket {
        pub tx_info: TxInfo,
        #[deku(count = "len as usize - size_of::<TxInfo>()")]
        pub data: Vec<u8>,
    }

    #[derive(Debug, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    pub struct NetCmdHeader {
        pub cmd: u8,
        pub len: u32,
    }

    #[derive(Debug, DekuRead, DekuWrite, PartialEq)]
    #[deku(endian = "big")]
    #[deku(ctx = "_endian: Endian, id: u8, len: u32", id = "id")]
    pub enum NetCmd {
        #[deku(id = "1")]
        Rc(u32),
        #[deku(id = "2")]
        GetChan,
        #[deku(id = "3")]
        SetChan(u32),
        #[deku(id = "4")]
        Write(#[deku(ctx = "len")] TxPacket),
        #[deku(id = "5")]
        Packet(#[deku(ctx = "len")] RxPacket),
        #[deku(id = "6")]
        GetMac,
        #[deku(id = "7")]
        Mac([u8; 6]),
        #[deku(id = "8")]
        GetMonitor,
        #[deku(id = "9")]
        GetRate,
        #[deku(id = "10")]
        SetRate(u32),
    }

    impl From<NetCmd> for NetCmdFrame {
        fn from(net_cmd: NetCmd) -> NetCmdFrame {
            let (cmd, len) = match &net_cmd {
                NetCmd::Rc(_) => (1, 4),
                NetCmd::GetChan => (2, 0),
                NetCmd::SetChan(_) => (3, 4),
                NetCmd::Write(TxPacket { data, .. }) => {
                    (4, (size_of::<TxInfo>() + data.len()) as u32)
                }
                NetCmd::Packet(RxPacket { data, .. }) => {
                    (5, (size_of::<RxInfo>() + data.len()) as u32)
                }
                NetCmd::GetMac => (6, 0),
                NetCmd::Mac(_) => (7, 6),
                NetCmd::GetMonitor => (8, 0),
                NetCmd::GetRate => (9, 0),
                NetCmd::SetRate(_) => (10, 4),
            };
            NetCmdFrame {
                cmd,
                len,
                body: net_cmd,
            }
        }
    }

    #[derive(Debug, DekuRead, DekuWrite, PartialEq)]
    #[deku(endian = "big")]
    pub struct NetCmdFrame {
        pub cmd: u8,
        pub len: u32,
        #[deku(ctx = "*cmd, *len")]
        pub body: NetCmd,
    }

    #[test]
    fn test_parse_net_cmd() {
        let (_, cmd) = NetCmdFrame::from_bytes((&[3u8, 0, 0, 0, 4, 0, 0, 0, 1], 0)).unwrap();
        assert_eq!(
            cmd,
            NetCmdFrame {
                cmd: 3,
                len: 4,
                body: NetCmd::SetChan(1),
            }
        );
    }

    #[test]
    fn test_generate_net_cmd() {
        let data = Into::<NetCmdFrame>::into(NetCmd::Rc(1)).to_bytes().unwrap();
        assert_eq!(data, &[1u8, 0, 0, 0, 4, 0, 0, 0, 1]);

        let data = Into::<NetCmdFrame>::into(NetCmd::GetChan)
            .to_bytes()
            .unwrap();
        assert_eq!(data, &[2u8, 0, 0, 0, 0]);

        let data = Into::<NetCmdFrame>::into(NetCmd::SetChan(1))
            .to_bytes()
            .unwrap();
        assert_eq!(data, &[3u8, 0, 0, 0, 4, 0, 0, 0, 1]);

        // let data = Into::<NetCmdFrame>::into(NetCmd::Write(vec![66])).to_bytes().unwrap();
        // assert_eq!(data, &[4u8, 0, 0, 0, 1, 66]);

        // let data = Into::<NetCmdFrame>::into(NetCmd::Packet(vec![66])).to_bytes().unwrap();
        // assert_eq!(data, &[5u8, 0, 0, 0, 1, 66]);

        let data = Into::<NetCmdFrame>::into(NetCmd::GetMac)
            .to_bytes()
            .unwrap();
        assert_eq!(data, &[6u8, 0, 0, 0, 0]);

        let data = Into::<NetCmdFrame>::into(NetCmd::Mac([1, 2, 3, 4, 5, 6]))
            .to_bytes()
            .unwrap();
        assert_eq!(data, &[7u8, 0, 0, 0, 6, 1, 2, 3, 4, 5, 6]);

        let data = Into::<NetCmdFrame>::into(NetCmd::GetMonitor)
            .to_bytes()
            .unwrap();
        assert_eq!(data, &[8u8, 0, 0, 0, 0]);

        let data = Into::<NetCmdFrame>::into(NetCmd::GetRate)
            .to_bytes()
            .unwrap();
        assert_eq!(data, &[9u8, 0, 0, 0, 0]);

        let data = Into::<NetCmdFrame>::into(NetCmd::SetRate(55))
            .to_bytes()
            .unwrap();
        assert_eq!(data, &[10u8, 0, 0, 0, 4, 0, 0, 0, 55]);
    }
}
