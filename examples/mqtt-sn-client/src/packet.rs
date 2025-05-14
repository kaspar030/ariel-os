use crate::header::{Header, MsgType};
use crate::message_variable_part::*;
use crate::packet::Error::{PacketNotRecognized, ParsingFailed};
use ariel_os::debug::log::info;
use bilge::arbitrary_int::{u24, u40};
use bilge::BitsError;
use core::str::from_utf8;

#[derive(PartialEq)]
pub enum Packet<'a> {
    // Advertise(Header, Advertise),
    // SearchGw(Header, SearchGw),
    // GwInfo(Header, GwInfo, [u8; 4096]),
    Connect {
        header: Header,
        connect: Connect,
        client_id: &'a str,
    },
    ConnAck {
        header: Header,
        conn_ack: ConnAck,
    },
    Publish {
        header: Header,
        publish: Publish,
        data: &'a str,
    },
    Subscribe {
        header: Header,
        subscribe: Subscribe,
        topic: &'a str, // long name, short name, id
    },
    SearchGw {
        // delay sending randomly
        header: Header,
        search_gw: SearchGw,
    },
}

pub enum Error {
    PacketNotRecognized,
    ParsingFailed,
}

impl Packet<'_> {
    pub fn try_from(bytes: &[u8]) -> Result<Packet, Error> {
        info!("bytes: {:?}", bytes);
        let header = Header::try_from(bytes).unwrap();

        info!("header: {:?}", header);

        let msg_type = header.msg_type();

        info!("msg_type: {:?}", msg_type);

        match msg_type {
            // MsgType::Advertise => {}
            // MsgType::SearchGw => {}
            // MsgType::GwInfo => {}
            // MsgType::Connect => {} // no recv
            MsgType::ConnAck => {
                let mvp_size = header.size() + ConnAck::SIZE;

                let mvp: [u8; ConnAck::SIZE] = bytes[header.size()..mvp_size].try_into().unwrap();

                let conn_ack = match ConnAck::try_from(u8::from_be_bytes(mvp)) {
                    Ok(it) => it,
                    Err(_) => return Err(ParsingFailed),
                };

                Ok(Packet::ConnAck { header, conn_ack })
            }
            // MsgType::WillTopicReq => {}
            // MsgType::WillTopic => {}
            // MsgType::WillMsgReq => {}
            // MsgType::WillMsg => {}
            // MsgType::Register => {}
            // MsgType::RegAck => {}
            MsgType::Publish => {
                let mvp_size = header.size() + 4;

                let data: &str = match from_utf8(&bytes[mvp_size..]) {
                    Ok(it) => it,
                    Err(_) => return Err(ParsingFailed),
                };

                let mvp: [u8; Publish::SIZE] = match bytes[header.size()..mvp_size].try_into() {
                    Ok(it) => it,
                    Err(_) => return Err(ParsingFailed),
                };

                let publish = match Publish::try_from(u40::from_be_bytes(mvp)) {
                    Ok(it) => it,
                    Err(_) => return Err(ParsingFailed),
                };

                Ok(Packet::Publish {
                    header,
                    publish,
                    data,
                })
            }
            // MsgType::PubAck => {}
            // MsgType::PubComp => {}
            // MsgType::PubRec => {}
            // MsgType::PubRel => {}
            MsgType::Subscribe => {
                let mvp_size = header.size() + Subscribe::SIZE;

                let topic: &str = from_utf8(&bytes[Subscribe::SIZE..]).unwrap();

                let mvp: [u8; Subscribe::SIZE] = bytes[header.size()..mvp_size].try_into().unwrap();

                let subscribe = Subscribe::try_from(u24::from_be_bytes(mvp)).unwrap();

                Ok(Packet::Subscribe {
                    header,
                    subscribe,
                    topic,
                })
            }
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
            _ => Err(PacketNotRecognized),
        }
    }
    //
    pub fn write_to_buf(&self, buf: &mut [u8]) {
        match self {
            // MsgType::Advertise => {}
            Packet::SearchGw { header, search_gw } => {
                buf[..header.size()].copy_from_slice(&header.to_be_bytes());
                buf[header.size()..].copy_from_slice(&search_gw.to_be_bytes());
            }
            // MsgType::GwInfo => {}
            Packet::Connect {
                header,
                connect,
                client_id,
            } => {
                if client_id.len() > 23 {
                    panic!("Client Id longer than maximum of 23 characters");
                }
                let mvp_size = header.size() + Connect::SIZE;

                // Self::construct_buf(buf, header, connect, Some(client_id));

                buf[..header.size()].copy_from_slice(&header.to_be_bytes()[..header.size()]);
                buf[header.size()..mvp_size].copy_from_slice(&connect.to_be_bytes());
                buf[mvp_size..header.length()].copy_from_slice(client_id.as_bytes());
            }
            // MsgType::ConnAck => {} // no send
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
                let mvp_size = header.size() + Publish::SIZE;

                // Self::construct_buf(buf, header, publish, Some(data));

                buf[..header.size()].copy_from_slice(&header.to_be_bytes()[..header.size()]);
                buf[header.size()..mvp_size].copy_from_slice(&publish.to_be_bytes());
                buf[mvp_size..header.length()].copy_from_slice(data.as_bytes());
            }
            // MsgType::PubAck => {}
            // MsgType::PubComp => {}
            // MsgType::PubRec => {}
            // MsgType::PubRel => {}
            // MsgType::Subscribe => {} // no send
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
            _ => {}
        }
    }

    // fn construct_buf(buf: &mut [u8], header: &Header, mvp: &impl MVP, payload: Option<&str>) {
    //     match mvp {
    //     }
    //
    //     let mvp_size = header.size() + MVP::SIZE;
    //
    //     buf[..header.size()].copy_from_slice(&header.to_be_bytes()[..header.size()]);
    //     buf[header.size()..mvp_size].copy_from_slice(&mvp.to_be_bytes());
    //
    //     if payload.is_some() {
    //         buf[mvp_size..header.length()].copy_from_slice(payload.unwrap().as_bytes());
    //     }
    // }

    pub fn get_msg_type(&self) -> MsgType {
        match self {
            Packet::Connect { header, .. } => header.msg_type(),
            Packet::ConnAck { header, .. } => header.msg_type(),
            Packet::Publish { header, .. } => header.msg_type(),
            Packet::Subscribe { header, .. } => header.msg_type(),
            Packet::SearchGw { header, .. } => header.msg_type(),
        }
    }
}

// #[cfg(test)]
// #[embedded_test::tests]
// mod tests {
//     use crate::{
//         flags::{Flags, QoS, TopicIdType},
//         header::{Header, MsgType},
//         message_variable_part,
//         packet::Packet,
//     };
//     use core::result;
//
//     #[test]
//     fn publish_read() {
//         let result = Packet::try_from(&[
//             0x12u8, 0x0Cu8, 0x62u8, 0x00u8, 0x01u8, 0x00u8, 0x00u8, 0x68u8, 0x65u8, 0x6Cu8, 0x6Cu8,
//             0x6Fu8, 0x20u8, 0x77u8, 0x6Fu8, 0x72u8, 0x6Cu8, 0x64u8,
//         ]);
//
//         match result {
//             Packet::Publish {
//                 header,
//                 publish,
//                 data,
//             } => {
//                 assert_eq!(header.msg_type(), MsgType::Publish);
//                 assert_eq!(data, "hello world");
//             }
//             _ => {}
//         }
//
//         //todo result.write_to_buf() "roundtrip test"
//     }
//
//     #[test]
//     fn publish_write() {
//         let data = "hello world";
//
//         let flags = Flags::new(
//             TopicIdType::IdPredefined,
//             false,
//             false,
//             false,
//             QoS::Zero,
//             false,
//         );
//
//         let length = (7 + data.len()) as u16;
//
//         let result = Packet::Publish {
//             header: Header::new(MsgType::Publish, length),
//             publish: message_variable_part::Publish::new(0, 1, flags),
//             data,
//         };
//
//         match result {
//             Packet::Publish {
//                 header,
//                 publish,
//                 data,
//             } => {
//                 assert_eq!(header.msg_type(), MsgType::Publish);
//                 assert_eq!(data, "hello world");
//             }
//             _ => {}
//         }
//     }
// }
