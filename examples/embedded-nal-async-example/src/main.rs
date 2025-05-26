#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(used_with_arg)]

mod udp_nal;

use crate::udp_nal::Error;
use ariel_os::{debug::log::*, net};
use core::future::Future;
use core::net::{SocketAddr, SocketAddrV4};
use embassy_net::{
    udp::{PacketMetadata, UdpSocket},
    Ipv4Address,
};
use embedded_nal_async::{ConnectedUdp, UnconnectedUdp};

#[ariel_os::task(autostart)]
async fn embedded_nal_async_example() {
    let stack = net::network_stack().await.unwrap();

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];
    let mut buf = [0; 4096];

    loop {
        let socket = UdpSocket::new(
            stack,
            &mut rx_meta,
            &mut rx_buffer,
            &mut tx_meta,
            &mut tx_buffer,
        );

        info!("Listening on UDP:1234...");
        let local: SocketAddr = "10.42.0.61:1234".parse().unwrap();
        let remote: SocketAddr = "10.42.0.1:1884".parse().unwrap();
        // let local = SocketAddr::V4(SocketAddrV4::new(Ipv4Address::new(10, 42, 0, 61), 1234));
        // let mut unconnected = match udp_nal::UnconnectedUdp::bind_multiple(socket, local).await {
        //     Ok(unconnected_udp) => unconnected_udp,
        //     Err(_) => {
        //         info!("bind error");
        //         break;
        //     }
        // };

        let mut connected = match udp_nal::ConnectedUdp::connect_from(socket, local, remote).await {
            Ok(connected) => connected,
            Err(_) => {
                info!("bind error");
                break;
            }
        };

        loop {
            let n = match connected.receive_into(&mut buf).await {
                Ok(0) => {
                    info!("read EOF");
                    break;
                }
                Ok(n) => n,
                Err(_) => {
                    info!("receive error");
                    break;
                }
            };

            info!(
                "Received datagram from {:?}:{:?}",
                connected.remote().addr,
                connected.remote().port
            );
            info!("Contents: {}", &buf[..n]);

            match connected.send(&buf[..n]).await {
                Ok(()) => {}
                Err(_) => {
                    info!("write error");
                    break;
                }
            }
        }
    }
}
