use crate::message_variable_part::Publish;
use crate::mqtt_sn::Error::ConversionFailed;
use crate::{
    flags::{Flags, QoS, TopicIdType},
    header::{Header, HeaderLong, HeaderShort, MsgType},
    message_variable_part as mvp,
    mqtt_sn::Error::{InvalidState, Timeout, TransmissionFailed},
    packet::Packet,
    udp_nal,
};
use ariel_os::{
    debug::log::info,
    reexports::embassy_time::{Duration, WithTimeout},
};
use bilge::prelude::*;
use core::net::SocketAddr;
use embedded_nal_async::UnconnectedUdp;

#[derive(PartialEq)]
enum State {
    Disconnected,
    Active,
    Asleep,
    Awake,
    Lost,
}

pub struct MqttSn<'a> {
    state: State,
    socket: udp_nal::UnconnectedUdp<'a>,
    local: SocketAddr,
    remote: SocketAddr,
    send_buf: &'a mut [u8],
    recv_buf: &'a mut [u8],
}

impl<'a> MqttSn<'a> {
    pub fn new(
        socket: udp_nal::UnconnectedUdp<'a>,
        local: SocketAddr,
        remote: SocketAddr,
        send_buf: &'a mut [u8],
        recv_buf: &'a mut [u8],
    ) -> Self {
        MqttSn {
            state: State::Disconnected,
            socket,
            local,
            remote,
            send_buf,
            recv_buf,
        }
    }

    async fn send(
        &mut self,
        length: u16,
        local: SocketAddr,
        remote: SocketAddr,
    ) -> Result<(), Error> {
        match self
            .socket
            .send(local, remote, &self.send_buf[..length as usize])
            .await
        {
            Ok(()) => {
                info!("Message: {}", &self.send_buf[..length as usize]);
                info!("Sent to: {}", remote);
                Ok(())
            }
            Err(_) => {
                info!("Transmission write error");
                Err(TransmissionFailed)
            }
        }
    }

    async fn receive(&mut self) -> Result<Packet, Error> {
        match self.socket.receive_into(self.recv_buf).await {
            Ok((n, _, _)) => match Packet::try_from(&self.recv_buf[..n]) {
                Ok(packet) => {
                    info!("received: {:?}", packet.get_msg_type());
                    match &packet {
                        Packet::ConnAck { header, conn_ack } => {
                            info!("{:?}", conn_ack);
                        }
                        _ => {}
                    }
                    Ok(packet)
                }
                Err(_) => {
                    info!("Conversion error for: {}", &self.recv_buf[..n]);
                    Err(ConversionFailed)
                }
            },
            Err(_) => {
                info!("Transmission read error");
                Err(TransmissionFailed)
            }
        }
    }

    pub async fn publish_minus_one(
        &mut self,
        payload: &str,
        local: SocketAddr,
        remote: SocketAddr,
    ) -> Result<(), Error> {
        let flags = Flags::new(
            TopicIdType::IdPredefined,
            false,
            false,
            false,
            QoS::Zero,
            false,
        );

        let length = calculate_message_length(payload.len(), mvp::Publish::BITS / 8);

        let packet = Packet::Publish {
            header: Header::new(MsgType::Publish, length),
            publish: mvp::Publish::new(0, 1, flags),
            data: payload,
        };

        packet.write_to_buf(&mut self.send_buf);

        self.send(length, local, remote).await
    }

    pub async fn connect(
        &mut self,
        duration_millis: u16,
        client_id: &str,
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
            connect: mvp::Connect::new(duration_millis, 0x01, flags),
            client_id,
        };

        packet.write_to_buf(&mut self.send_buf);

        self.send(msg_len, self.local, self.remote).await?;

        match self
            .receive()
            .with_timeout(Duration::from_millis(duration_millis as u64))
            .await
        {
            Ok(res) => {
                if res?.get_msg_type() != MsgType::ConnAck {
                    return Err(TransmissionFailed);
                }
                self.state = State::Awake;
                Ok(())
            }
            Err(_) => Err(Timeout),
        }
    }

    pub async fn subscribe(
        &mut self,
        topic: &str,
        topic_id_type: TopicIdType,
        msg_id: u16,
        dup: bool,
        qos: QoS,
    ) -> Result<(), Error> {
        let flags = Flags::new(topic_id_type, true, false, false, qos, dup);
        let msg_len = calculate_message_length(topic.len(), mvp::Subscribe::SIZE);

        info!("msg_len: {}", &msg_len);
        info!("msg_id: {}", &msg_id);

        let packet = Packet::Subscribe {
            header: Header::new(MsgType::Subscribe, msg_len),
            subscribe: mvp::Subscribe::new(msg_id, flags),
            topic,
        };

        info!("write buf");
        packet.write_to_buf(&mut self.send_buf);
        info!("send start");
        self.send(msg_len, self.local, self.remote).await?;
        info!("send complete");

        match self
            .receive()
            .with_timeout(Duration::from_millis(60000))
            .await
        {
            Ok(res) => {
                match res? {
                    Packet::SubAck { .. } => {
                        info!("SubAck for topic: {}", topic);
                        self.state = State::Active;
                        Ok(())
                    }
                    _ => Err(TransmissionFailed),
                }
                // if res.get_msg_type() != MsgType::SubAck {
                //     return Err(TransmissionFailed);
                // }
                // self.state = State::Active;
                // Ok(())
            }
            Err(_) => Err(Timeout),
        }
    }

    pub async fn expect_message(&mut self) -> Option<&str> {
        match self.receive().await {
            Ok(res) => match res {
                Packet::Publish {
                    header,
                    publish,
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
}

#[derive(Debug)]
pub enum Error {
    InvalidState,
    Timeout,
    TransmissionFailed,
    ConversionFailed,
}

fn calculate_message_length(payload_len: usize, mvp_byte_len: usize) -> u16 {
    let mvp_len = payload_len + mvp_byte_len;

    if (mvp_len + HeaderShort::BITS / 8) > 256 {
        (mvp_len + HeaderLong::BITS / 8) as u16
    } else {
        (mvp_len + HeaderShort::BITS / 8) as u16
    }
}
