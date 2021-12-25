use deku::prelude::*;
use std::mem::size_of_val;

#[derive(DekuRead, DekuWrite, Eq, PartialEq, Hash, Debug)]
pub struct Frame {
    #[deku(bits = 3)]
    pub version: u8,
    #[deku(bits = 5)]
    pub frame_type: u8,
    pub len: u16,
    #[deku(ctx = "*frame_type, *len")]
    pub body: FrameBody,
}

#[derive(DekuRead, DekuWrite, Eq, PartialEq, Hash, Debug)]
#[deku(ctx = "id: u8, len: u16", id = "id")]
pub enum FrameBody {
    #[deku(id = "0")]
    Keepalive,
    #[deku(id = "1")]
    Data {
        channel: u32,
        #[deku(count = "len as usize - size_of_val(channel)")]
        data: Vec<u8>,
    },
}

impl Into<Frame> for FrameBody {
    fn into(self) -> Frame {
        let (frame_type, len) = match &self {
            FrameBody::Keepalive => (0u8, 0),
            FrameBody::Data { data, channel } => (1, (data.len() + size_of_val(channel)) as u16),
        };
        Frame {
            version: 0,
            frame_type,
            len,
            body: self,
        }
    }
}
