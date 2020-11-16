use std::{io::Write, pin::Pin, task::{Context, Poll}};
use tokio::{io::{AsyncRead, AsyncReadExt, ReadBuf}, stream::Stream};
use pcap_parser::{LegacyPcapReader, Linktype, PcapBlockOwned, PcapError, traits::PcapReaderIterator};
use futures::ready;
use anyhow::Result;
use ringbuf::{RingBuffer, Consumer, Producer};

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
                match self.capture.next() {
                    Ok((offset, block)) => {
                        let ret = match block {
                            PcapBlockOwned::LegacyHeader(_hdr) => {
                                log::info!("header {:?}", _hdr);
                                self.datalink = Some(_hdr.network);
                                None
                            },
                            PcapBlockOwned::Legacy(packet) => {
                                Some(Packet {
                                    data: packet.data.to_vec(),
                                })
                            },
                            PcapBlockOwned::NG(_) => unreachable!(),
                        };
                        self.capture.consume(offset);
                        if let Some(r) = ret {
                            return Poll::Ready(Some(Ok(r)));
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
