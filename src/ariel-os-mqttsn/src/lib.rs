#![no_std]
use flags::{Flags, QoS, TopicIdType};
use header::{Header, HeaderLong, HeaderShort, MsgType};
use message_variable_part as mvp;
use mvp::ReturnCode;
use packet::Packet;
use Error::*;

use ariel_os::{
    debug::log::*,
    reexports::embassy_time::{Duration, WithTimeout},
};
use bilge::prelude::*;
use core::{net::SocketAddr, str::from_utf8};
use embedded_nal_async::UnconnectedUdp;

pub mod flags;
mod header;
mod message_variable_part;
mod packet;
pub mod udp_nal;

const UDP_PAYLOAD_SIZE: usize = u16::MAX as usize - 8;

#[derive(PartialEq)]
enum State {
    Disconnected,
    Active,
    Asleep,
    #[expect(dead_code, reason = "state not yet implemented")]
    Awake,
    #[expect(dead_code, reason = "state not yet implemented")]
    Lost,
}

#[derive(Debug, defmt::Format)]
pub enum Topic<'a> {
    Id(u16),
    ShortName([u8; 2]),
    LongName(&'a [u8]),
}

impl<'a> Topic<'a> {
    pub fn from_short(short_name: [u8; 2]) -> Topic<'a> {
        Self::ShortName(short_name)
    }

    pub fn from_id(id: u16) -> Topic<'a> {
        Self::Id(id)
    }

    pub fn from_long(long_name: &'a [u8]) -> Topic<'a> {
        Self::LongName(long_name)
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Id(_id) => 2,
            Self::ShortName(_short_name) => 2,
            Self::LongName(long_name) => long_name.len(),
        }
    }
}

pub struct MqttSn<'a> {
    state: State,
    socket: udp_nal::UnconnectedUdp<'a>,
    local: SocketAddr,
    remote: SocketAddr,
    send_buf: [u8; UDP_PAYLOAD_SIZE],
    recv_buf: [u8; UDP_PAYLOAD_SIZE],
    msg_id: u16,
}

impl<'a> MqttSn<'a> {
    pub fn new(socket: udp_nal::UnconnectedUdp<'a>, local: SocketAddr, remote: SocketAddr) -> Self {
        MqttSn {
            state: State::Disconnected,
            socket,
            local,
            remote,
            send_buf: [0u8; UDP_PAYLOAD_SIZE],
            recv_buf: [0u8; UDP_PAYLOAD_SIZE],
            msg_id: 0,
        }
    }

    async fn send(&mut self, length: u16) -> Result<(), Error> {
        match self
            .socket
            .send(self.local, self.remote, &self.send_buf[..length as usize])
            .await
        {
            Ok(()) => {
                info!("Message: {:?}", &self.send_buf[..length as usize]);
                info!("Sent to: {}", self.remote);
                Ok(())
            }
            Err(_) => {
                info!("Transmission write error");
                Err(TransmissionFailed)
            }
        }
    }

    async fn receive(&mut self) -> Result<Packet, Error> {
        info!("Receiving...");
        match self.socket.receive_into(&mut self.recv_buf).await {
            Ok((n, _, _)) => match Packet::try_from(&self.recv_buf[..n]) {
                Ok(packet) => {
                    info!("received: {:?}", packet.get_msg_type());
                    match &packet {
                        Packet::ConnAck {
                            header: _,
                            conn_ack,
                        } => {
                            info!("{:?}", conn_ack);
                        }
                        _ => {}
                    }
                    Ok(packet)
                }
                Err(_) => {
                    info!("Conversion error for: {:?}", &self.recv_buf[..n]);
                    Err(ConversionFailed)
                }
            },
            Err(_) => {
                info!("Transmission read error");
                Err(TransmissionFailed)
            }
        }
    }

    pub async fn publish_minus_one(&mut self, payload: &[u8]) -> Result<(), Error> {
        let flags = Flags::new(
            TopicIdType::IdPredefined,
            false,
            false,
            false,
            QoS::MinusOne,
            false,
        );

        let length = calculate_message_length(payload.len(), mvp::Publish::SIZE);

        let packet = Packet::Publish {
            header: Header::new(MsgType::Publish, length),
            publish: mvp::Publish::new(0, 1, flags),
            data: payload,
        };

        packet.write_to_buf(&mut self.send_buf);

        self.send(length).await
    }

    pub async fn connect(
        &mut self,
        duration_millis: u16,
        client_id: &[u8],
        dup: bool,
        will: bool,
        clean_session: bool,
    ) -> Result<(), Error> {
        if self.state != State::Disconnected {
            return Err(InvalidState);
        }

        let flags = Flags::new(
            TopicIdType::IdNormal,
            clean_session,
            will,
            false,
            QoS::Zero,
            dup,
        );

        let msg_len = calculate_message_length(client_id.len(), mvp::Connect::SIZE);

        let packet = Packet::Connect {
            header: Header::new(MsgType::Connect, msg_len),
            connect: mvp::Connect::new(0x00, 0x01, flags),
            client_id,
        };

        packet.write_to_buf(&mut self.send_buf);

        self.send(msg_len).await?;

        match self
            .receive()
            .with_timeout(Duration::from_millis(duration_millis as u64))
            .await
            .map_err(|_| Timeout)?
            .map(|res| res.get_msg_type())?
        {
            MsgType::ConnAck => {
                self.state = State::Active;
                Ok(())
            }
            _ => Err(TransmissionFailed),
        }
    }

    pub async fn subscribe(
        &mut self,
        topic: Topic<'_>,
        dup: bool,
        qos: QoS,
    ) -> Result<Topic, Error> {
        if self.state != State::Active {
            return Err(InvalidState);
        }

        let (topic_id_type, topic_value) = match &topic {
            Topic::ShortName(name) => (TopicIdType::ShortName, &name[..]),
            Topic::Id(id) => (TopicIdType::IdPredefined, &id.to_be_bytes()[..]),
            Topic::LongName(name) => (TopicIdType::IdNormal, &name[..]),
        };

        let flags = Flags::new(topic_id_type, true, false, false, qos, dup);
        let msg_len = calculate_message_length(topic.len(), mvp::Subscribe::SIZE);

        trace!("msg_len: {}", &msg_len);

        let packet = Packet::Subscribe {
            header: Header::new(MsgType::Subscribe, msg_len),
            subscribe: mvp::Subscribe::new(self.msg_id, flags),
            topic: topic_value,
        };

        self.msg_id += 1;

        info!("write buf");
        packet.write_to_buf(&mut self.send_buf);
        info!("send start");
        self.send(msg_len).await?;
        info!("send complete");

        match self
            .receive()
            .with_timeout(Duration::from_millis(60000))
            .await
        {
            Ok(res) => match res? {
                Packet::SubAck { header: _, sub_ack } => {
                    let topic_id = sub_ack.get_topic_id();
                    info!(
                        "received Topic Id {:?} for Topic {}",
                        topic_id,
                        from_utf8(topic_value).unwrap()
                    );
                    Ok(Topic::from_id(topic_id))
                }
                _ => Err(TransmissionFailed),
            },
            Err(_) => Err(Timeout),
        }
    }

    pub async fn expect_message(&mut self) -> Option<&[u8]> {
        match self.receive().await {
            Ok(res) => match res {
                Packet::Publish {
                    header: _,
                    publish: _,
                    data,
                } => Some(data),
                _ => None,
            },
            Err(_) => None,
        }
    }

    pub fn disconnect(&self) -> Result<(), Error> {
        if self.state != State::Active {
            return Err(InvalidState);
        }
        Ok(())
    }

    pub fn sleep(self) -> Result<(), Error> {
        if self.state != State::Active {
            return Err(InvalidState);
        }
        Ok(())
    }

    pub fn wake(self) -> Result<(), Error> {
        if self.state != State::Asleep {
            return Err(InvalidState);
        }
        Ok(())
    }

    pub async fn register<'b>(&mut self, topic: &'b [u8]) -> Result<Topic<'b>, Error> {
        if self.state != State::Active {
            return Err(InvalidState);
        }

        let msg_len = calculate_message_length(topic.len(), mvp::Register::SIZE);

        let packet = Packet::Register {
            header: Header::new(MsgType::Register, msg_len),
            register: mvp::Register::new(0x0000, self.msg_id),
            topic,
        };

        self.msg_id += 1;

        packet.write_to_buf(&mut self.send_buf);
        self.send(msg_len).await?;

        match self
            .receive()
            .with_timeout(Duration::from_millis(60000))
            .await
        {
            Ok(res) => match res? {
                Packet::RegAck { header: _, reg_ack } => {
                    let return_code = reg_ack.get_return_code();
                    if return_code != ReturnCode::Accepted {
                        error!("{:?}", return_code);
                        return Err(Rejected);
                    }
                    let topic_id = reg_ack.get_topic_id();
                    info!(
                        "received Topic Id {:?} for Topic {}",
                        topic_id,
                        from_utf8(topic).unwrap()
                    );
                    return Ok(Topic::from_id(topic_id));
                }
                _ => Err(TransmissionFailed),
            },
            Err(_) => Err(Timeout),
        }
    }

    pub async fn publish(&mut self, topic: &Topic<'_>, payload: &[u8]) -> Result<(), Error> {
        if self.state != State::Active {
            return Err(InvalidState);
        }

        let (topic_id_type, topic_value) = match topic {
            Topic::ShortName(name) => (TopicIdType::ShortName, &u16::from_be_bytes(*name)),
            Topic::Id(id) => (TopicIdType::IdPredefined, id),
            Topic::LongName(_) => return Err(InvalidIdType),
        };

        let flags = Flags::new(topic_id_type, false, false, false, QoS::Zero, false);

        let length = calculate_message_length(payload.len(), mvp::Publish::SIZE);

        let packet = Packet::Publish {
            header: Header::new(MsgType::Publish, length),
            publish: mvp::Publish::new(0x0000u16, *topic_value, flags), // msg_id 0x0000 on QoS 0 & -1
            data: payload,
        };

        // self.msg_id += 1;

        packet.write_to_buf(&mut self.send_buf);

        self.send(length).await
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidState,
    Timeout,
    TransmissionFailed,
    ConversionFailed,
    Rejected,
    InvalidIdType,
}

fn calculate_message_length(payload_len: usize, mvp_byte_len: usize) -> u16 {
    let mvp_len = payload_len + mvp_byte_len;

    if (mvp_len + HeaderShort::BITS / 8) > 256 {
        (mvp_len + HeaderLong::BITS / 8) as u16
    } else {
        (mvp_len + HeaderShort::BITS / 8) as u16
    }
}
