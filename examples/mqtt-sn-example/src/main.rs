#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(used_with_arg)]

mod udp_nal;

use ariel_os::{
    debug::log::*,
    net,
    time::{Duration, Timer},
};
use core::net::SocketAddr;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embedded_nal_async::UnconnectedUdp;

#[ariel_os::task(autostart)]
async fn mqtt_sn_example() {
    let stack = net::network_stack().await.unwrap();

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];
    let mut send_buf = [0; 4096];
    // let mut recv_buf = [0; 4096];

    // MQTT Variables
    // Flags: 1 octet
    let dup = 0;
    let qos = 0b11; // QoS level -1 (send w/o connection)
    let retain = 0;
    let will = 0;
    let clean_session = 0;
    let topic_id_type = 0b01; // pre-defined topic id

    // Message Variable part: n octets
    // let client_id;
    let data = b"hello world";
    // let duration;
    let flags: u8 = (dup << 7)
        | (qos << 5)
        | (retain << 4)
        | (will << 3)
        | (clean_session << 2)
        | topic_id_type;
    // let gw_add;
    // let gw_id;
    let msg_id: u8 = 0;
    // let protocol_id = 0x01; // fixed value, all others reserved
    // let radius;
    // let return_code;
    let topic_id: u8 = 1; // 0x0000 & 0xFFFF reserved
                          // let topic_name;
                          // let will_msg;
                          // let will_topic;

    // Message Header: 2 or 4 octets
    let length = (7 + data.len()) as u8; // TOTAL number of octets in message
    let msg_type: u8 = 0x0C; // MQTT-SN PUBLISH

    // CONNECT = length, msg_type, flags, protocol_id, duration, client_id
    // PUBLISH = length, msg_type, flags, topic_id, msg_id, data
    let message = [length, msg_type, flags, 0, topic_id, 0, msg_id];

    for i in 0..7 {
        send_buf[i] = message[i];
    }
    for i in 0..data.len() {
        send_buf[7 + i] = data[i];
    }

    loop {
        let socket = UdpSocket::new(
            stack,
            &mut rx_meta,
            &mut rx_buffer,
            &mut tx_meta,
            &mut tx_buffer,
        );

        info!("Listening on UDP:1234...");
        let local: SocketAddr = "0.0.0.0:1234".parse().unwrap();
        let mut unconnected = match udp_nal::UnconnectedUdp::bind_multiple(socket, local).await {
            Ok(unconnected_udp) => unconnected_udp,
            Err(_) => {
                info!("bind error");
                break;
            }
        };

        let remote: SocketAddr = "10.42.0.1:1884".parse().unwrap();

        loop {
            // let (n, local, remote) = match unconnected.receive_into(&mut recv_buf).await {
            //     Ok((0, _, _)) => {
            //         info!("read EOF");
            //         break;
            //     }
            //     Ok((n, local, remote)) => (n, local, remote),
            //     Err(_) => {
            //         info!("receive error");
            //         break;
            //     }
            // };

            info!("Connecting to {:?}", remote);

            info!("{}", length);
            match unconnected
                .send(local, remote, &send_buf[..length as usize])
                .await
            {
                Ok(()) => {
                    info!("Sent message");
                }
                Err(_) => {
                    info!("write error");
                    break;
                }
            }

            Timer::after(Duration::from_millis(1000)).await;
        }
    }
}
