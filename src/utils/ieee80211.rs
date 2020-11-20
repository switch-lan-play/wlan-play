use deku::ctx::{Endian, BitSize};
use deku::prelude::*;
use std::fmt;

#[derive(DekuRead, DekuWrite, Eq, PartialEq, Hash)]
pub struct Mac([u8; 6]);

impl fmt::Debug for Mac {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let m = &self.0;
        write!(f, "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", m[0], m[1], m[2], m[3], m[4], m[5])
    }
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
#[deku(type = "u8", bits = "2", ctx = "_bitsize: BitSize")]
pub enum FrameType {
    #[deku(id = "0")]
    Management,
    #[deku(id = "1")]
    Control,
    #[deku(id = "2")]
    Data,
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
pub struct Flags {
    #[deku(bits = 1)]
    pub order: u8,
    #[deku(bits = 1)]
    pub protected_frame: u8,
    #[deku(bits = 1)]
    pub more_data: u8,
    #[deku(bits = 1)]
    pub power_management: u8,
    #[deku(bits = 1)]
    pub retry: u8,
    #[deku(bits = 1)]
    pub more_fragments: u8,
    #[deku(bits = 1)]
    pub from_ds: u8,
    #[deku(bits = 1)]
    pub to_ds: u8,
}

#[derive(Debug, DekuRead, DekuWrite, PartialEq)]
pub struct FrameControl {
    #[deku(bits = 4)]
    pub sub_type: u8,
    #[deku(bits = 2)]
    pub frame_type: FrameType,
    #[deku(bits = 2)]
    pub protocol_version: u8,

    pub flags: Flags,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
pub struct Frame {
    pub frame_control: FrameControl,
    pub duration_id: u16,
    pub addr1: Mac,
    #[deku(cond = "frame_control.frame_type != FrameType::Control")]
    pub addr2: Option<Mac>,
    #[deku(cond = "frame_control.frame_type != FrameType::Control")]
    pub addr3: Option<Mac>,
    #[deku(cond = "frame_control.frame_type != FrameType::Control")]
    pub sequence_control: Option<u16>,
    // pub addr4: Option<Mac>,
    // pub qos_control: Option<u16>,
    // pub ht_control: Option<u32>,
}

mod tests {
    use super::*;
    use deku::prelude::*;

    #[test]
    fn test_control_frame() {
        let data = vec![0x88u8, 0x41];
        let (_, control_frame) = FrameControl::from_bytes((data.as_ref(), 0)).unwrap();
        println!("{:#?}", control_frame);
        assert_eq!(control_frame, FrameControl {
            protocol_version: 0,
            frame_type: FrameType::Data,
            sub_type: 8,

            flags: Flags {
                to_ds: 1,
                from_ds: 0,
                more_fragments: 0,
                retry: 0,
                power_management: 0,
                more_data: 0,
                protected_frame: 1,
                order: 0,
            }
        })
    }

    #[test]
    fn test_frame() {
        let data = vec![0xc4u8, 0x00, 0xca, 0x00, 0x98, 0x41, 0x5c, 0xdc, 0x22, 0xec];
        let (_, frame) = Frame::from_bytes((data.as_ref(), 0)).unwrap();
        println!("{:#x?}", frame);

        let data = vec![
            0x08u8, 0x42, 0x00, 0x00, 0x33, 0x33, 0x00, 0x00, 0x01, 0x8c, 0x2c, 0xf8, 0x9b, 0xdd, 0x06, 0xa0,
            0x2c, 0xf8, 0x9b, 0x15, 0xa3, 0xd0, 0x20, 0x1e, 0x0a, 0x05, 0x00, 0x60, 0x00, 0x00, 0x00, 0x00,
        ];
        let (_, frame) = Frame::from_bytes((data.as_ref(), 0)).unwrap();
        println!("{:#x?}", frame);

        let data = vec![
            0x88, 0x41, 0x3a, 0x00, 0x2c, 0xf8, 0x9b, 0xdd, 0x06, 0xa0, 0x00, 0x20, 0xa6, 0xfc, 0xb0, 0x36,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x20, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x20, 0x00, 0x00,
            0x00, 0x00
        ];
        let (_, frame) = Frame::from_bytes((data.as_ref(), 0)).unwrap();
        println!("{:#x?}", frame);
    }
}
