use pcap_parser::*;
use pcap_parser::traits::PcapReaderIterator;
use std::{fs::File, io::Write};
use structopt::StructOpt;
use std::path::PathBuf;
use anyhow::Result;
use crc::crc32::checksum_ieee;

#[derive(Debug, StructOpt)]
#[structopt(about = "A tool to convert UDP pcap to 802.11 pcap")]
pub struct Opt {
    /// Input file
    #[structopt(parse(from_os_str))]
    pub input: PathBuf,
    /// Output file
    #[structopt(parse(from_os_str))]
    pub output: PathBuf,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();
    let file = File::open(opt.input)?;
    let mut num_blocks = 0;
    let mut reader = LegacyPcapReader::new(65536, file)?;
    let mut writer = File::create(opt.output)?;
    loop {
        match reader.next() {
            Ok((offset, block)) => {
                match block {
                    PcapBlockOwned::LegacyHeader(mut header) => {
                        // 802.11
                        header.network = Linktype(105);
                        writer.write_all(&header.to_vec()?)?;
                        println!("{:?}", header);
                    }
                    PcapBlockOwned::Legacy(LegacyPcapBlock { data, ts_sec, ts_usec, caplen, origlen  }) => {
                        const OFFSET: usize = 0x31;
                        let data = &data[OFFSET..];
                        let fcs = checksum_ieee(data);
                        let data = &[data, &[fcs as u8, (fcs >> 8) as u8, (fcs >> 16) as u8, (fcs >> 24) as u8]].concat();
                        let mut block = LegacyPcapBlock {
                            data,
                            ts_sec,
                            ts_usec,
                            caplen: caplen - OFFSET as u32 + 4,
                            origlen: origlen - OFFSET as u32 + 4,
                        };
                        writer.write_all(&block.to_vec()?)?;
                    }
                    _ => {}
                };
                num_blocks += 1;
                reader.consume(offset);
            },
            Err(PcapError::Eof) => break,
            Err(PcapError::Incomplete) => {
                reader.refill().unwrap();
            },
            Err(e) => panic!("error while reading: {:?}", e),
        }
    };
    println!("num_blocks: {}", num_blocks);
    Ok(())
}