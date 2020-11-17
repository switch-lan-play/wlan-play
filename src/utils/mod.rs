pub mod timeout;
pub mod pcap_reader;
pub mod pcap_writer;

pub struct Packet {
    pub data: Vec<u8>,
}
