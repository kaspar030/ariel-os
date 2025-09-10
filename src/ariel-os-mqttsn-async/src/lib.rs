#![no_std]
pub mod flags;
mod header;
mod message_variable_part;
mod packet;
pub mod udp_nal;

use ariel_os::debug::log::*;
use ariel_os::net;
use ariel_os::time::Duration;
// TODO: when rebased, change to ariel_os::time::with_timeout
use ariel_os::reexports::embassy_time::WithTimeout;
use core::{default::Default, net::SocketAddr};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex,
    channel::{Channel, Receiver},
    mutex::Mutex,
    once_lock::OnceLock,
    pubsub::PubSubChannel,
};
use embedded_nal_async::UnconnectedUdp;
use header::{Header, HeaderLong, HeaderShort, MsgType};
use heapless::String;
use packet::Packet;

use message_variable_part as mvp;

pub const MAX_PAYLOAD_SIZE: usize = 1024;

pub const MAX_TOPIC_LENGTH: usize = 64;
// pub const MAX_PUBLISHERS: usize = 16;

// type MessageChannel = PubSubChannel<CriticalSectionRawMutex, Message, 1, 1, MAX_PUBLISHERS>;

// static SOCKET: OnceLock<MqttsnSocket> = OnceLock::new();
// static MESSAGE_CHANNEL: MessageChannel = PubSubChannel::new();
pub static ACTION_CHANNEL: Channel<CriticalSectionRawMutex, Action, 1> = Channel::new();
// static HANDLERS: Mutex<CriticalSectionRawMutex, [MessageChannel; MAX_PUBLISHERS]> = Mutex::new([]);
// static SUBSCRIPTIONS: Mutex<CriticalSectionRawMutex, [(u16, u16); MAX_PUBLISHERS]> = Mutex::new([]);

type Payload = [u8; MAX_PAYLOAD_SIZE];

#[derive(Clone)]
pub enum Message {
    Publish { topic: u16, payload: Payload },
}

#[derive(Clone)]
pub enum Action {
    Subscribe(Topic),
    Publish { topic: u16, payload: Payload },
}

#[derive(PartialEq, Default, Debug)]
enum State {
    #[default]
    Disconnected,
    Connected,
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

#[derive(Debug, defmt::Format, Clone)]
pub enum Topic {
    Id(u16),
    ShortName([u8; 2]),
    LongName(String<MAX_TOPIC_LENGTH>),
}

pub struct MqttsnConnection<'a> {
    state: State,
    local: SocketAddr,
    remote: SocketAddr,
    socket: udp_nal::UnconnectedUdp<'a>,
    action_rx: Receiver<'a, CriticalSectionRawMutex, Action, 1>,
}

impl<'a> MqttsnConnection<'a> {
    async fn handle_action(&mut self, action: Action) -> Result<(), Error> {
        match action {
            Action::Subscribe(_) => {
                info!("subscribe");
            }
            Action::Publish { topic, payload } => {
                info!("publish");
            }
        }
        Ok(())
    }

    async fn run(&mut self) {
        // TODO: loop, handle errors
        self.connect(60000, b"light", false, false, false)
            .await
            .unwrap();

        // TODO: "32" is arbitrary
        let mut rx_buf = [0; MAX_PAYLOAD_SIZE + 32];
        loop {
            let mut rx_fut = self.socket.receive_into(&mut rx_buf);
            match select(rx_fut, self.action_rx.receive()).await {
                Either::First(Ok(length)) => {
                    info!("got udp");
                }
                Either::Second(action) => {
                    self.handle_action(action).await.unwrap();
                }
                Either::First(Err(_)) => todo!(),
            }
        }
    }

    pub async fn connect(
        &mut self,
        duration_millis: u16,
        client_id: &[u8],
        dup: bool,
        will: bool,
        clean_session: bool,
    ) -> Result<(), Error> {
        info!("connect");
        use flags::{Flags, QoS, TopicIdType};

        let mut buf: [u8; MAX_PAYLOAD_SIZE + 32] = [0; MAX_PAYLOAD_SIZE + 32];

        if self.state != State::Disconnected {
            return Err(Error::InvalidState);
        }

        // build "connect" packet
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

        // send connect packet
        packet.write_to_buf(&mut buf);
        {
            info!("connect: send");
            let packet_slice = &buf[..msg_len as usize];
            self.send(packet_slice).await?;
        }

        info!("connect: recv");
        // wait for connack
        match self
            .receive(&mut buf)
            .with_timeout(Duration::from_millis(duration_millis as u64))
            .await
            .map_err(|_| Error::Timeout)?
            .map(|res| res.get_msg_type())?
        {
            MsgType::ConnAck => {
                self.state = State::Connected;
                Ok(())
            }
            _ => Err(Error::TransmissionFailed),
        }
    }

    async fn send(&mut self, buf: &[u8]) -> Result<(), Error> {
        match self.socket.send(self.local, self.remote, &buf).await {
            Ok(_) => Ok(()),
            Err(_) => Err(Error::TransmissionFailed),
        }
    }

    async fn receive<'b>(&mut self, buf: &'b mut [u8]) -> Result<Packet<'b>, Error> {
        match self.socket.receive_into(buf).await {
            Ok((n, _, _)) => match Packet::try_from(&buf[..n]) {
                Ok(packet) => Ok(packet),
                Err(_) => Err(Error::ConversionFailed),
            },
            Err(_) => Err(Error::TransmissionFailed),
        }
    }
}

#[embassy_executor::task]
pub async fn client() {
    let stack = net::network_stack().await.unwrap();
    let local = "0.0.0.0:1884".parse().unwrap();
    let remote = "192.168.1.129:1884".parse().unwrap();

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];

    let socket = UdpSocket::new(
        stack,
        &mut rx_meta,
        &mut rx_buffer,
        &mut tx_meta,
        &mut tx_buffer,
    );

    let mut socket = udp_nal::UnconnectedUdp::bind_multiple(socket, local)
        .await
        .unwrap();

    let action_rx = ACTION_CHANNEL.receiver();
    let mut connection = MqttsnConnection {
        socket,
        local,
        remote,
        action_rx,
        state: State::Disconnected,
    };

    connection.run().await;
}

impl Topic {
    pub fn from_short(short_name: [u8; 2]) -> Topic {
        Self::ShortName(short_name)
    }

    pub fn from_id(id: u16) -> Topic {
        Self::Id(id)
    }

    pub fn from_long(long_name: &str) -> Topic {
        // TODO: make fallible
        Self::LongName(String::try_from(long_name).unwrap())
    }

    pub fn len(&self) -> usize {
        match self {
            Self::Id(_id) => 2,
            Self::ShortName(_short_name) => 2,
            Self::LongName(long_name) => long_name.len(),
        }
    }
}

// async fn send(buf: &[u8], length: u16) -> Result<(), Error> {
//     let socket = SOCKET.get().await;

//     match socket
//         .socket
//         .send(socket.local, socket.remote, &buf[..length as usize])
//         .await
//     {
//         Ok(_) => Ok(()),
//         Err(_) => Err(Error::TransmissionFailed),
//     };
// }

// async fn ping() -> Result<(), Error> {
//     const MSG_LEN: u16 = 16;
//     let packet = Packet::PingResp {
//         header: Header::new(MsgType::PingResp, MSG_LEN),
//     };
//     let mut send_buf = [0u8; MSG_LEN];
//     packet.write_to_buf(send_buf);
//     send(send_buf, MSG_LEN).await?;
//     Ok(())
// }

// pub async fn init(
//     max_buffer_size: u16,
//     client_id: &[u8],
//     socket: MqttsnSocket<'_>,
// ) -> Result<(), Error> {
//     // Initialize Buffer Size, Global Lists, Socket
//     // MQTT-SN Connect
//     // Start Client Task
// }

// #[embassy_executor::task]
// async fn client() {
//     loop {
//         let mut buf = [0u8; MAX_PAYLOAD_SIZE + 32];

//         match receive(&mut buf).await {
//             Ok(packet) => match packet {
//                 Packet::Publish {
//                     header: _,
//                     publish,
//                     data,
//                 } => {
//                     let topic_id = publish.get_topic_id();

//                     SUBSCRIPTIONS.lock().await.map(|subs| async {
//                         if subs.0 == topic_id {
//                             for i in 0..16u16 {
//                                 // check if bit is set in handler bitmap
//                                 if subs.1 & (1 << i) != 0 {
//                                     let handlers = HANDLERS.lock().await;
//                                     let handler: MessageChannel = handlers[i as usize];
//                                     handler.publisher().unwrap().publish_immediate(
//                                         Message::Publish {
//                                             topic: topic_id,
//                                             payload: data,
//                                         },
//                                     );
//                                 }
//                             }
//                         }
//                     });
//                     continue;
//                 }
//                 Packet::PingReq { .. } => match ping().await {
//                     Ok(_) => continue,
//                     Err(_) => (), // handle Error
//                 },
//                 _ => {
//                     info!("Conversion error");
//                     continue;
//                 }
//             },
//             Err(_) => {
//                 info!("Packet not recognized");
//                 continue;
//             }
//         }
//     }
// }

fn calculate_message_length(payload_len: usize, mvp_byte_len: usize) -> u16 {
    use bilge::Bitsized;
    let mvp_len = payload_len + mvp_byte_len;

    if (mvp_len + HeaderShort::BITS / 8) > 256 {
        (mvp_len + HeaderLong::BITS / 8) as u16
    } else {
        (mvp_len + HeaderShort::BITS / 8) as u16
    }
}
