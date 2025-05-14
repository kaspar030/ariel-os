#![no_main]
#![no_std]
#![feature(used_with_arg)]
#![feature(impl_trait_in_assoc_type)]
#![feature(let_chains)]

mod flags;
mod header;
mod message_variable_part;
mod mqtt_sn;
mod packet;
mod udp_nal;

use crate::flags::{QoS, TopicIdType};
use crate::mqtt_sn::MqttSn;
use ariel_os::{debug::log::*, net};
// use ariel_os_random as rng;
use core::net::SocketAddr;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use rand::Rng as _;

#[ariel_os::task(autostart)]
async fn mqtt_sn_client() {
    let stack = net::network_stack().await.unwrap();

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];

    let mut send_buf = [0u8; 4096];
    let mut recv_buf = [0u8; 4096];

    loop {
        let socket = UdpSocket::new(
            stack,
            &mut rx_meta,
            &mut rx_buffer,
            &mut tx_meta,
            &mut tx_buffer,
        );

        let local: SocketAddr = "0.0.0.0:1234".parse().unwrap();
        let unconnected = match udp_nal::UnconnectedUdp::bind_multiple(socket, local).await {
            Ok(unconnected_udp) => unconnected_udp,
            Err(_) => {
                info!("bind error");
                break;
            }
        };

        let remote: SocketAddr = "10.42.0.1:1884".parse().unwrap();

        let mut mqtt_sn = MqttSn::new(unconnected, local, remote, &mut send_buf, &mut recv_buf);

        info!("Listening on UDP:1234...");

        loop {
            info!("...attempt connection");
            match mqtt_sn.connect(3000, "test", false, false, false).await {
                Ok(_) => break,
                Err(_) => continue,
            };
        }

        info!("...connected!");
        //
        // let mut rng = ariel_os::random::fast_rng();
        //
        // mqtt_sn
        //     .subscribe(
        //         "topic1",
        //         TopicIdType::IdNormal,
        //         rng.gen_range(1..=u16::MAX),
        //         false,
        //         QoS::Zero,
        //     )
        //     .expect("TODO: panic message");

        break;
    }
}
