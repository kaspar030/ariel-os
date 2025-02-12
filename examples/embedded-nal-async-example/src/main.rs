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
use embedded_nal_async::UnconnectedUdp;

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
        // let local = SocketAddr::V4(SocketAddrV4::new(Ipv4Address::new(10, 42, 0, 61), 1234));
        let mut unconnected = match udp_nal::UnconnectedUdp::bind_multiple(socket, local).await {
            Ok(unconnected_udp) => unconnected_udp,
            Err(_) => {
                info!("bind error");
                break;
            }
        };

        loop {
            let (n, local, remote) = match unconnected.receive_into(&mut buf).await {
                Ok((0, _, _)) => {
                    info!("read EOF");
                    break;
                }
                Ok((n, local, remote)) => (n, local, remote),
                Err(_) => {
                    info!("receive error");
                    break;
                }
            };

            info!("Received datagram from {:?}", remote.ip());
            info!("Contents: {}", &buf[..n]);

            match unconnected.send(local, remote, &buf[..n]).await {
                Ok(()) => {}
                Err(_) => {
                    info!("write error");
                    break;
                }
            }
        }
    }
}
