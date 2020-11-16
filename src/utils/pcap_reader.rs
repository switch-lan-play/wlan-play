use std::{io::{Write, Cursor}, pin::Pin, task::{Context, Poll}};
use tokio::{io::{AsyncRead, AsyncReadExt, ReadBuf}, stream::Stream};
use pcap_parser::{LegacyPcapBlock, LegacyPcapReader, Linktype, PcapBlockOwned, PcapError, traits::PcapReaderIterator};
use futures::ready;
use anyhow::{anyhow, Result};
use ringbuf::{RingBuffer, Consumer, Producer};
use byteorder::{LittleEndian, ReadBytesExt};

pub struct Packet {
    pub data: Vec<u8>,
}

pub struct PcapReader<R> {
    reader: R,
    prod: Producer<u8>,
    capture: LegacyPcapReader<Consumer<u8>>,
    datalink: Option<Linktype>,
}

impl<R> PcapReader<R>
where
    R: AsyncRead + Unpin,
{
    pub async fn new(mut reader: R) -> Result<Self> {
        let buffer = RingBuffer::<u8>::new(2048);
        let (mut prod, cons) = buffer.split();
        let mut buf = [0u8; 32];
        reader.read_exact(&mut buf).await?;
        prod.write_all(&buf)?;
        prod.flush()?;
        // log::trace!("fuck {:?}", String::from_utf8(buf.to_vec()));
        Ok(PcapReader {
            reader,
            prod,
            capture: LegacyPcapReader::new(65536, cons)?,
            datalink: None,
        })
    }
}

fn convert_packet(datalink: Linktype, block: LegacyPcapBlock) -> Result<Packet> {
    let mut data = block.data.to_vec();
    Ok(match datalink.0 {
        // LINKTYPE_IEEE802_11
        105 => {
            Packet {
                data
            }
        }
        // LINKTYPE_IEEE802_11_RADIOTAP
        127 => {
            let mut cursor = Cursor::new(&mut data);
            if ReadBytesExt::read_u16::<LittleEndian>(&mut cursor)? != 0 {
                Err(anyhow!("radiotap header is invalid"))?
            }
            let radiotap_len = ReadBytesExt::read_u16::<LittleEndian>(&mut cursor)?;
            Packet {
                data: data.split_off(radiotap_len as usize),
            }
        }
        _ => Err(anyhow!("Unsupported data link {}", datalink))?
    })
}

impl<R> Stream for PcapReader<R>
where
    R: AsyncRead + Unpin,
{
    type Item = Result<Packet>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        loop {
            loop {
                let datalink = self.datalink;
                match self.capture.next() {
                    Ok((offset, block)) => {
                        let ret = match block {
                            PcapBlockOwned::LegacyHeader(_hdr) => {
                                log::info!("header {:?}", _hdr);
                                self.datalink = Some(_hdr.network);
                                None
                            },
                            PcapBlockOwned::Legacy(packet) => {
                                Some(convert_packet(datalink.ok_or(anyhow!("No header!"))?, packet))
                            },
                            PcapBlockOwned::NG(_) => unreachable!(),
                        };
                        self.capture.consume(offset);
                        if let Some(r) = ret {
                            return Poll::Ready(Some(Ok(r?)));
                        }
                    }
                    Err(PcapError::Incomplete) => {
                        break;
                    }
                    Err(e) => {
                        Err(e)?
                    }
                }
            }

            let mut buf = [0u8; 1024];
            let mut buf = ReadBuf::new(&mut buf);
    
            ready!(Pin::new(&mut self.reader).poll_read(cx, &mut buf))?;
            self.prod.write_all(&buf.filled())?;
            self.capture.refill()?;
        }
    }
}
