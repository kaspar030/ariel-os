#![no_std]
pub mod flags;
mod header;
mod message_variable_part;
mod packet;
pub mod udp_nal;

use ariel_os::debug::log::*;
use core::net::SocketAddr;
use embassy_executor::Spawner;
use embassy_sync::{
    blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel, mutex::Mutex,
    once_lock::OnceLock, pubsub::PubSubChannel,
};
use embedded_nal_async::UnconnectedUdp;
use header::{Header, HeaderLong, HeaderShort, MsgType};
use heapless::Vec;
use packet::Packet;

pub const MAX_PAYLOAD_SIZE: usize = 1024; // -> OnceLock, Initialize later!
pub const MAX_PUBLISHERS: usize = 16;

type MessageChannel = PubSubChannel<CriticalSectionRawMutex, Message, 1, 1, MAX_PUBLISHERS>;

static SOCKET: OnceLock<MqttsnSocket> = OnceLock::new();
static MESSAGE_CHANNEL: MessageChannel = PubSubChannel::new();
static ACTION_CHANNEL: Channel<CriticalSectionRawMutex, Action, 1> = Channel::new();
static HANDLERS: Mutex<CriticalSectionRawMutex, [MessageChannel; MAX_PUBLISHERS]> = Mutex::new([]);
static SUBSCRIPTIONS: Mutex<CriticalSectionRawMutex, [(u16, u16); MAX_PUBLISHERS]> = Mutex::new([]);

type Payload = [u8; MAX_PAYLOAD_SIZE];

#[derive(Clone)]
pub enum Message {
    Publish { topic: u16, payload: Payload },
}

#[derive(Clone)]
pub enum Action {
    Subscribe,
    Publish { topic: u16, payload: Payload },
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

pub struct MqttsnSocket<'a> {
    socket: udp_nal::UnconnectedUdp<'a>,
    local: SocketAddr,
    remote: SocketAddr,
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

async fn send(buf: &[u8], length: u16) -> Result<(), Error> {
    let socket = SOCKET.get().await;

    match socket
        .socket
        .send(socket.local, socket.remote, &buf[..length as usize])
        .await
    {
        Ok(_) => Ok(()),
        Err(_) => Err(Error::TransmissionFailed),
    };
}

async fn receive(buf: &mut [u8]) -> Result<Packet, Error> {
    let socket = SOCKET.get().await;

    match socket.socket.receive_into(buf).await {
        Ok((n, _, _)) => match Packet::try_from(&buf[..n]) {
            Ok(packet) => Ok(packet),
            Err(_) => Err(Error::ConversionFailed),
        },
        Err(_) => Err(Error::TransmissionFailed),
    }
}

async fn ping() -> Result<(), Error> {
    const MSG_LEN: u16 = 16;
    let packet = Packet::PingResp {
        header: Header::new(MsgType::PingResp, MSG_LEN),
    };
    let mut send_buf = [0u8; MSG_LEN];
    packet.write_to_buf(send_buf);
    send(send_buf, MSG_LEN).await?;
    Ok(())
}

pub async fn init(
    max_buffer_size: u16,
    client_id: &[u8],
    socket: MqttsnSocket<'_>,
) -> Result<(), Error> {
    // Initialize Buffer Size, Global Lists, Socket
    // MQTT-SN Connect
    // Start Client Task
}

#[embassy_executor::task]
async fn client() {
    loop {
        let mut buf = [0u8; MAX_PAYLOAD_SIZE + 32];

        match receive(&mut buf).await {
            Ok(packet) => match packet {
                Packet::Publish {
                    header: _,
                    publish,
                    data,
                } => {
                    let topic_id = publish.get_topic_id();

                    SUBSCRIPTIONS.lock().await.map(|subs| async {
                        if subs.0 == topic_id {
                            for i in 0..16u16 {
                                // check if bit is set in handler bitmap
                                if subs.1 & (1 << i) != 0 {
                                    let handlers = HANDLERS.lock().await;
                                    let handler: MessageChannel = handlers[i as usize];
                                    handler.publisher().unwrap().publish_immediate(
                                        Message::Publish {
                                            topic: topic_id,
                                            payload: data,
                                        },
                                    );
                                }
                            }
                        }
                    });
                    continue;
                }
                Packet::PingReq { .. } => match ping().await {
                    Ok(_) => continue,
                    Err(_) => (), // handle Error
                },
                _ => {
                    info!("Conversion error");
                    continue;
                }
            },
            Err(_) => {
                info!("Packet not recognized");
                continue;
            }
        }
    }
}
