use deku::prelude::*;

#[derive(DekuRead, DekuWrite, Eq, PartialEq, Hash)]
pub struct Frame {
    #[deku(bits = 3)]
    version: u8,
    #[deku(bits = 5)]
    frame_type: u8,
    len: u16,
    #[deku(ctx = "*frame_type, *len")]
    body: FrameBody,
}

#[derive(DekuRead, DekuWrite, Eq, PartialEq, Hash)]
#[deku(ctx = "id: u8, len: u16", id = "id")]
pub enum FrameBody {
    #[deku(id = "0")]
    Data {
        #[deku(count = "len")]
        data: Vec<u8>,
    },
}
