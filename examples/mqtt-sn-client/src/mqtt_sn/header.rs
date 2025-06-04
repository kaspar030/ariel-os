use ariel_os::debug::log::*;
use bilge::give_me_error;
use bilge::prelude::*;

#[bitsize(8)]
#[derive(TryFromBits, Debug, PartialEq, defmt::Format)]
pub enum MsgType {
    Advertise = 0x00,
    SearchGw = 0x01,
    GwInfo = 0x02,
    Connect = 0x04,
    ConnAck = 0x05,
    WillTopicReq = 0x06,
    WillTopic = 0x07,
    WillMsgReq = 0x08,
    WillMsg = 0x09,
    Register = 0x0A,
    RegAck = 0x0B,
    Publish = 0x0C,
    PubAck = 0x0D,
    PubComp = 0x0E,
    PubRec = 0x0F,
    PubRel = 0x10,
    Subscribe = 0x12,
    SubAck = 0x13,
    Unsubscribe = 0x14,
    UnsubAck = 0x15,
    PingReq = 0x16,
    PingResp = 0x17,
    Disconnect = 0x18,
    WillTopicUpd = 0x1A,
    WillTopicEsp = 0x1B,
    WillMsgUpd = 0x1C,
    WillMsgEsp = 0x1D,
    Encapsulated = 0xFE,
}

#[bitsize(32)]
#[derive(TryFromBits, DebugBits, PartialEq, defmt::Format)]
pub struct HeaderLong {
    msg_type: MsgType,
    length: u24,
}

#[bitsize(16)]
#[derive(TryFromBits, DebugBits, PartialEq, defmt::Format)]
pub struct HeaderShort {
    msg_type: MsgType,
    length: u8,
}

#[derive(Debug, PartialEq, defmt::Format)]
pub enum Header {
    Long(HeaderLong),
    Short(HeaderShort),
}

impl Header {
    const LONG_FLAG: u8 = 0x01;

    pub fn new(msg_type: MsgType, length: u16) -> Self {
        let length_bytes = length.to_be_bytes();
        if length_bytes[0] == 0u8 {
            // todo simplyfy by checking < 256?
            Self::Short(HeaderShort::new(msg_type, length_bytes[1]))
        } else {
            Self::Long(HeaderLong::new(
                msg_type,
                u24::from_be_bytes([Self::LONG_FLAG, length_bytes[0], length_bytes[1]]),
            ))
        }
    }

    pub fn msg_type(&self) -> MsgType {
        match self {
            Header::Long(long) => long.msg_type(),
            Header::Short(short) => short.msg_type(),
        }
    }

    /// Get the total message length
    pub fn length(&self) -> usize {
        match self {
            Header::Short(header_short) => header_short.length() as usize,
            Header::Long(header_long) => header_long.length().as_usize(),
        }
    }

    pub fn length_mvp(&self) -> usize {
        match self {
            Header::Short(header_short) => header_short.length() as usize - 2,
            Header::Long(header_long) => header_long.length().as_usize() - 4,
        }
    }

    pub fn is_long(&self) -> bool {
        matches!(self, Self::Long(_))
    }

    pub fn size(&self) -> usize {
        match self {
            Header::Short(_) => 2,
            Header::Long(_) => 4,
        }
    }

    pub fn to_be_bytes(&self) -> [u8; 4] {
        let mut result: [u8; 4] = [0u8; 4];
        match self {
            Header::Long(long) => {
                info!("long {:?}", long.value.to_be_bytes());
                result.copy_from_slice(&long.value.to_be_bytes()[..4])
            }
            Header::Short(short) => {
                info!("short {:?}", short.value.to_be_bytes());
                result[..2].copy_from_slice(&short.value.to_be_bytes()[..2])
            }
        }
        info!("result {:?}", result);
        result
    }
}

impl TryFrom<u16> for Header {
    type Error = bilge::BitsError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match HeaderShort::try_from(value) {
            Ok(header_short) => Ok(Self::Short(header_short)),
            Err(err) => Err(err),
        }
    }
}

impl TryFrom<u32> for Header {
    type Error = bilge::BitsError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value.to_be_bytes()[0] != 1 {
            return Err(give_me_error());
        }
        match HeaderLong::try_from(value) {
            Ok(header) => Ok(Self::Long(header)),
            Err(err) => Err(err),
        }
    }
}

impl TryFrom<&[u8]> for Header {
    type Error = bilge::BitsError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value[0] == Self::LONG_FLAG {
            // println!("long header");
            Header::try_from(u32::from_be_bytes(value[..4].try_into().unwrap()))
        } else {
            // println!("short header");
            Header::try_from(u16::from_be_bytes(value[..2].try_into().unwrap()))
        }
    }
}

// #[cfg(test)]
// #[embedded_test::tests]
// mod tests {
//     use crate::header::*;
//
//     #[test]
//     fn short() {
//         assert_eq!(
//             HeaderShort::new(MsgType::Publish, 25),
//             HeaderShort::try_from(0x19_0C).unwrap()
//         )
//     }
//
//     #[test]
//     fn long() {
//         assert_eq!(
//             HeaderLong::new(MsgType::Publish, u24::new(25)),
//             HeaderLong::try_from(0x19_0C).unwrap()
//         )
//     }
//
//     #[test]
//     fn header() {
//         assert_eq!(
//             Header::try_from(0x12_0Cu16).unwrap(),
//             Header::Short(HeaderShort::new(MsgType::Publish, 18))
//         );
//     }
// }
