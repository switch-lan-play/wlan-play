pub mod timeout;
pub mod pcap_reader;
pub mod pcap_writer;
pub mod ieee80211;

pub struct Packet {
    pub data: Vec<u8>,
}
