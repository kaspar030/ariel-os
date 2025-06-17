use crate::{
    header::{Header, MsgType},
    message_variable_part as mvp,
    packet::Error::{PacketNotRecognized, ParsingFailed},
};
use ariel_os::debug::log::*;
use bilge::arbitrary_int::{u24, u40, u48};

#[derive(PartialEq)]
pub enum Packet<'a> {
    // Advertise(Header, Advertise),
    // SearchGw(Header, SearchGw),
    // GwInfo(Header, GwInfo, [u8; 4096]),
    Connect {
        header: Header,
        connect: mvp::Connect,
        client_id: &'a [u8],
    },
    ConnAck {
        header: Header,
        conn_ack: mvp::ConnAck,
    },
    Publish {
        header: Header,
        publish: mvp::Publish,
        data: &'a [u8],
    },
    Subscribe {
        header: Header,
        subscribe: mvp::Subscribe,
        topic: &'a [u8], // long name, short name, id
    },
    SubAck {
        header: Header,
        sub_ack: mvp::SubAck,
    },
    SearchGw {
        // delay sending randomly
        header: Header,
        search_gw: mvp::SearchGw,
    },
}

macro_rules! construct_buffer {
    ($buf:tt, $header: tt, $mvp: tt, $mvp_len:expr) => {
        let mvp_size = $header.size() + $mvp_len;

        $buf[..$header.size()].copy_from_slice(&$header.to_be_bytes()[..$header.size()]);
        $buf[$header.size()..mvp_size].copy_from_slice(&$mvp.to_be_bytes()[..$mvp_len]);
    };
    ($buf:tt, $header: tt, $mvp: tt, $mvp_len:expr, $payload: tt) => {
        construct_buffer!($buf, $header, $mvp, $mvp_len);

        let mvp_size = $header.size() + $mvp_len;

        $buf[mvp_size..$header.length()].copy_from_slice($payload);
    };
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
                let mvp_size = header.size() + mvp::ConnAck::SIZE;

                let mvp: [u8; mvp::ConnAck::SIZE] =
                    bytes[header.size()..mvp_size].try_into().unwrap();

                let conn_ack = match mvp::ConnAck::try_from(u8::from_be_bytes(mvp)) {
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
                let mvp_size = header.size() + mvp::Publish::SIZE;

                let mvp: [u8; mvp::Publish::SIZE] = match bytes[header.size()..mvp_size].try_into()
                {
                    Ok(it) => it,
                    Err(_) => return Err(ParsingFailed),
                };

                let publish = match mvp::Publish::try_from(u40::from_be_bytes(mvp)) {
                    Ok(it) => it,
                    Err(_) => return Err(ParsingFailed),
                };

                Ok(Packet::Publish {
                    header,
                    publish,
                    data: &bytes[mvp_size..],
                })
            }
            // MsgType::PubAck => {}
            // MsgType::PubComp => {}
            // MsgType::PubRec => {}
            // MsgType::PubRel => {}
            MsgType::Subscribe => {
                let mvp_size = header.size() + mvp::Subscribe::SIZE;

                let mvp: [u8; mvp::Subscribe::SIZE] =
                    bytes[header.size()..mvp_size].try_into().unwrap();

                let subscribe = mvp::Subscribe::try_from(u24::from_be_bytes(mvp)).unwrap();

                Ok(Packet::Subscribe {
                    header,
                    subscribe,
                    topic: &bytes[mvp::Subscribe::SIZE..],
                })
            }
            MsgType::SubAck => {
                let mvp_size = header.size() + mvp::SubAck::SIZE;

                let mvp: [u8; mvp::SubAck::SIZE] =
                    bytes[header.size()..mvp_size].try_into().unwrap();

                let sub_ack = mvp::SubAck::try_from(u48::from_be_bytes(mvp)).unwrap();

                Ok(Packet::SubAck { header, sub_ack })
            }
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
            _ => {
                info!("Packet not recognized");
                Err(PacketNotRecognized)
            }
        }
    }
    //
    pub fn write_to_buf(&self, buf: &mut [u8]) {
        match self {
            // MsgType::Advertise => {}
            Packet::SearchGw { header, search_gw } => {
                construct_buffer!(buf, header, search_gw, mvp::SearchGw::SIZE);
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
                construct_buffer!(buf, header, connect, mvp::Connect::SIZE, client_id);
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
                construct_buffer!(buf, header, publish, mvp::Publish::SIZE, data);
            }
            // MsgType::PubAck => {}
            // MsgType::PubComp => {}
            // MsgType::PubRec => {}
            // MsgType::PubRel => {}
            Packet::Subscribe {
                header,
                subscribe,
                topic,
            } => {
                construct_buffer!(buf, header, subscribe, mvp::Subscribe::SIZE, topic);
            }
            // MsgType::SubAck => {} // no send
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
            _ => {
                info!("Packet not recognized");
            }
        }
    }

    pub fn get_msg_type(&self) -> MsgType {
        match self {
            Packet::Connect { header, .. } => header.msg_type(),
            Packet::ConnAck { header, .. } => header.msg_type(),
            Packet::Publish { header, .. } => header.msg_type(),
            Packet::Subscribe { header, .. } => header.msg_type(),
            Packet::SearchGw { header, .. } => header.msg_type(),
            Packet::SubAck { header, .. } => header.msg_type(),
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
