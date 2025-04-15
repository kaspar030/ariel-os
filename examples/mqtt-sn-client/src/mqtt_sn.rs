use crate::{
    flags::{Flags, QoS, TopicIdType},
    header::Header,
    header::{HeaderShort, MsgType},
    message_variable_part as mvp,
    packet::Packet::*,
    udp_nal,
};
use ariel_os::debug::log::info;
use bilge::prelude::*;
use core::net::SocketAddr;
use embedded_nal_async::UnconnectedUdp;
use heapless::String;

/// MQTT-SN state
enum ClientState {
    Disconnected,
    Active,
    Asleep,
    Awake,
    Lost,
}

pub struct MqttSn<'a> {
    socket: udp_nal::UnconnectedUdp<'a>,
    broker: SocketAddr,
    client_state: ClientState,
    msg_buf: [u8; 4096],
}

impl<'a> MqttSn<'a> {
    pub fn new(socket: udp_nal::UnconnectedUdp<'a>, broker: SocketAddr) -> Self {
        Self {
            socket,
            broker,
            client_state: ClientState::Disconnected,
            msg_buf: [0; 4096],
        }
    }

    pub async fn publish_minus_one(&mut self, payload: &str) {
        let flags = Flags::new(
            TopicIdType::IdPredefined,
            false,
            false,
            false,
            QoS::Zero,
            false,
        );

        let length = (7 + payload.len()) as u8; // TOTAL number of octets in message

        // Header::new(MsgType::Publish, length.as_u16());
        //
        // Header::try_from(0x40u16).unwrap();

        // Publish(
        //     Header::new(MsgType::Publish, length as u16),
        //     mvp::Publish::new(0, 1, flags),
        //     payload,
        // );

        let header = u16::from(HeaderShort::new(MsgType::Publish, length)).to_le_bytes();
        let message_variable_part = u40::from(mvp::Publish::new(0, 1, flags)).to_le_bytes();

        for i in 0..2 {
            self.msg_buf[i] = header[i];
        }
        for i in 0..5 {
            self.msg_buf[2 + i] = message_variable_part[i];
        }
        for i in 0..payload.len() {
            self.msg_buf[7 + i] = payload.as_bytes()[i];
        }

        self.send(length).await;
    }

    async fn send(&mut self, length: u8) {
        let local: SocketAddr = "0.0.0.0:1234".parse().unwrap();
        match self
            .socket
            .send(local, self.broker, &self.msg_buf[..length as usize])
            .await
        {
            Ok(()) => {
                info!(
                    "Message: {}",
                    core::str::from_utf8(&self.msg_buf[..length as usize]).unwrap()
                );
                info!("Sent to: {}", self.broker);
            }
            Err(_) => {
                info!("write error");
            }
        }
    }
}
