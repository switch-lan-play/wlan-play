
use tokio::io::{AsyncRead, AsyncWrite};

/// A client to communicate with airserv-ng
///
/// Reference: https://github.com/aircrack-ng/aircrack-ng/blob/565870292e210010dea65ab4f289fc5ff392bd45/lib/osdep/network.c
pub struct AirNetwork<S> {
    s: S,
}

impl<S> AirNetwork<S>
where
    S: AsyncRead + AsyncWrite,
{
    fn new(stream: S) -> AirNetwork<S> {
        AirNetwork {
            s: stream,
        }
    }
    async fn cmd() {

    }
    pub async fn read() {

    }
    pub async fn write() {

    }
    pub async fn set_channel() {

    }
    pub async fn get_channel() {

    }
    pub async fn set_rate() {

    }
    pub async fn get_rate() {

    }
    pub async fn close() {

    }
    pub async fn get_mac() {

    }
    pub async fn get_monitor() {

    }
}

mod protocol {
    use deku::ctx::Endian;
    use deku::prelude::*;

    #[derive(Debug, DekuRead, DekuWrite)]
    #[deku(endian = "big")]
    pub struct NetCmdHeader {
        cmd: u8,
        len: u32,
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
        Write(#[deku(count = "len")] Vec<u8>),
        #[deku(id = "5")]
        Packet(#[deku(count = "len")] Vec<u8>,),
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

    impl Into<NetCmdFrame> for NetCmd {
        fn into(self) -> NetCmdFrame {
            let (cmd, len) = match &self {
                NetCmd::Rc(_) => (1, 4),
                NetCmd::GetChan => (2, 0),
                NetCmd::SetChan(_) => (3, 4),
                NetCmd::Write(p) => (4, p.len() as u32),
                NetCmd::Packet(p) => (5, p.len() as u32),
                NetCmd::GetMac => (6, 0),
                NetCmd::Mac(_) => (7, 6),
                NetCmd::GetMonitor => (8, 0),
                NetCmd::GetRate => (9, 0),
                NetCmd::SetRate(_) => (10, 4),
            };
            NetCmdFrame {
                cmd,
                len,
                body: self,
            }
        }
    }

    #[derive(Debug, DekuRead, DekuWrite, PartialEq)]
    #[deku(endian = "big")]
    pub struct NetCmdFrame {
        cmd: u8,
        len: u32,
        #[deku(ctx = "*cmd, *len")]
        body: NetCmd,
    }

    #[test]
    fn test_parse_net_cmd() {
        let (_, cmd) = NetCmdFrame::from_bytes((&[3u8, 0, 0, 0, 4, 0, 0, 0, 1], 0)).unwrap();
        assert_eq!(cmd, NetCmdFrame {
            cmd: 3,
            len: 4,
            body: NetCmd::SetChan(1),
        });

        let (_, cmd) = NetCmdFrame::from_bytes((&[5u8, 0, 0, 0, 4, 1, 2, 3, 4], 0)).unwrap();
        assert_eq!(cmd, NetCmdFrame {
            cmd: 5,
            len: 4,
            body: NetCmd::Packet(vec![1, 2, 3, 4]),
        });
    }
    
    #[test]
    fn test_generate_net_cmd() {
        let data = Into::<NetCmdFrame>::into(NetCmd::Rc(1)).to_bytes().unwrap();
        assert_eq!(data, &[1u8, 0, 0, 0, 4, 0, 0, 0, 1]);

        let data = Into::<NetCmdFrame>::into(NetCmd::GetChan).to_bytes().unwrap();
        assert_eq!(data, &[2u8, 0, 0, 0, 0]);

        let data = Into::<NetCmdFrame>::into(NetCmd::SetChan(1)).to_bytes().unwrap();
        assert_eq!(data, &[3u8, 0, 0, 0, 4, 0, 0, 0, 1]);

        let data = Into::<NetCmdFrame>::into(NetCmd::Write(vec![66])).to_bytes().unwrap();
        assert_eq!(data, &[4u8, 0, 0, 0, 1, 66]);

        let data = Into::<NetCmdFrame>::into(NetCmd::Packet(vec![66])).to_bytes().unwrap();
        assert_eq!(data, &[5u8, 0, 0, 0, 1, 66]);

        let data = Into::<NetCmdFrame>::into(NetCmd::GetMac).to_bytes().unwrap();
        assert_eq!(data, &[6u8, 0, 0, 0, 0]);

        let data = Into::<NetCmdFrame>::into(NetCmd::Mac([1, 2, 3, 4, 5, 6])).to_bytes().unwrap();
        assert_eq!(data, &[7u8, 0, 0, 0, 6, 1, 2, 3, 4, 5, 6]);

        let data = Into::<NetCmdFrame>::into(NetCmd::GetMonitor).to_bytes().unwrap();
        assert_eq!(data, &[8u8, 0, 0, 0, 0]);

        let data = Into::<NetCmdFrame>::into(NetCmd::GetRate).to_bytes().unwrap();
        assert_eq!(data, &[9u8, 0, 0, 0, 0]);

        let data = Into::<NetCmdFrame>::into(NetCmd::SetRate(55)).to_bytes().unwrap();
        assert_eq!(data, &[10u8, 0, 0, 0, 4, 0, 0, 0, 55]);

    }
}
