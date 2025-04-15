use crate::header::{Header, MsgType};
use crate::message_variable_part::*;
use ariel_os::debug::*;
use bilge::arbitrary_int::u40;
use bilge::Bitsized;
use core::str::from_utf8;

pub enum Packet<'a> {
    // Advertise(Header, Advertise),
    // SearchGw(Header, SearchGw),
    // GwInfo(Header, GwInfo, [u8; 4096]),
    Publish {
        header: Header,
        publish: Publish,
        data: &'a str,
    },
    Null,
}

impl Packet<'_> {
    pub fn try_from(bytes: &[u8]) -> Packet {
        println!("bytes: {:?}", bytes);
        let header = Header::try_from(bytes).unwrap();

        println!("header: {:?}", header);

        let msg_type = header.msg_type();

        println!("msg_type: {:?}", msg_type);

        match msg_type {
            // MsgType::Advertise => {}
            // MsgType::SearchGw => {}
            // MsgType::GwInfo => {}
            // MsgType::Connect => {}
            // MsgType::ConnAck => {}
            // MsgType::WillTopicReq => {}
            // MsgType::WillTopic => {}
            // MsgType::WillMsgReq => {}
            // MsgType::WillMsg => {}
            // MsgType::Register => {}
            // MsgType::RegAck => {}
            MsgType::Publish => {
                const PUBLISH_BYTE_LEN: usize = 5;

                let mvp_byte_len = header.len() + PUBLISH_BYTE_LEN;

                let data: &str = from_utf8(&bytes[mvp_byte_len..]).unwrap();

                let mvp: [u8; PUBLISH_BYTE_LEN] =
                    bytes[header.len()..mvp_byte_len].try_into().unwrap();

                let publish = Publish::try_from(u40::from_be_bytes(mvp)).unwrap();

                Packet::Publish {
                    header,
                    publish,
                    data,
                }
            }
            // MsgType::PubAck => {}
            // MsgType::PubComp => {}
            // MsgType::PubRec => {}
            // MsgType::PubRel => {}
            // MsgType::Subscribe => {}
            // MsgType::SubAck => {}
            // MsgType::Unsubscribe => {}
            // MsgType::UnsubAck => {}
            // MsgType::PingReq => {}
            // MsgType::PingResp => {}
            // MsgType::Disconnect => {}
            // MsgType::WillTopicUpd => {}
            // MsgType::WillTopicEsp => {}
            // MsgType::WillMsgUpd => {}
            // MsgType::WillMsgEsp => {}
            // MsgType::Encapsulated => {}
            _ => Packet::Null,
        }
    }
    //
    fn write_to_buf(&self, buf: &mut [u8]) {
        match self {
            // MsgType::Advertise => {}
            // MsgType::SearchGw => {}
            // MsgType::GwInfo => {}
            // MsgType::Connect => {}
            // MsgType::ConnAck => {}
            // MsgType::WillTopicReq => {}
            // MsgType::WillTopic => {}
            // MsgType::WillMsgReq => {}
            // MsgType::WillMsg => {}
            // MsgType::Register => {}
            // MsgType::RegAck => {}
            Packet::Publish {
                header,
                publish,
                data,
            } => {
                buf[..header.len()].copy_from_slice(&header.to_be_bytes());
                buf[header.len()..].copy_from_slice(&publish.to_be_bytes());
                buf[header.len() + header.length_mvp()..].copy_from_slice(data.as_bytes());
            }
            // MsgType::PubAck => {}
            // MsgType::PubComp => {}
            // MsgType::PubRec => {}
            // MsgType::PubRel => {}
            // MsgType::Subscribe => {}
            // MsgType::SubAck => {}
            // MsgType::Unsubscribe => {}
            // MsgType::UnsubAck => {}
            // MsgType::PingReq => {}
            // MsgType::PingResp => {}
            // MsgType::Disconnect => {}
            // MsgType::WillTopicUpd => {}
            // MsgType::WillTopicEsp => {}
            // MsgType::WillMsgUpd => {}
            // MsgType::WillMsgEsp => {}
            // MsgType::Encapsulated => {}
            Packet::Null => {}
        }
    }
}

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use crate::{
        flags::{Flags, QoS, TopicIdType},
        header::{Header, MsgType},
        message_variable_part,
        packet::Packet,
    };
    use core::result;

    #[test]
    fn publish_read() {
        let result = Packet::try_from(&[
            0x12u8, 0x0Cu8, 0x62u8, 0x00u8, 0x01u8, 0x00u8, 0x00u8, 0x68u8, 0x65u8, 0x6Cu8, 0x6Cu8,
            0x6Fu8, 0x20u8, 0x77u8, 0x6Fu8, 0x72u8, 0x6Cu8, 0x64u8,
        ]);

        match result {
            Packet::Publish {
                header,
                publish,
                data,
            } => {
                assert_eq!(header.msg_type(), MsgType::Publish);
                assert_eq!(data, "hello world");
            }
            _ => {}
        }
    }

    #[test]
    fn publish_write() {
        let data = "hello world";

        let flags = Flags::new(
            TopicIdType::IdPredefined,
            false,
            false,
            false,
            QoS::Zero,
            false,
        );

        let length = (7 + data.len()) as u16;

        let result = Packet::Publish {
            header: Header::new(MsgType::Publish, length),
            publish: message_variable_part::Publish::new(0, 1, flags),
            data,
        };

        match result {
            Packet::Publish {
                header,
                publish,
                data,
            } => {
                assert_eq!(header.msg_type(), MsgType::Publish);
                assert_eq!(data, "hello world");
            }
            _ => {}
        }
    }
}
