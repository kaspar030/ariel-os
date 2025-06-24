#![no_main]
#![no_std]

use ariel_os::{debug::log::*, net};
use ariel_os_mqttsn::{flags::QoS, udp_nal, MqttSn, Topic};
use core::net::SocketAddr;
use embassy_net::udp::{PacketMetadata, UdpSocket};

#[ariel_os::task(autostart)]
async fn mqtt_sn_client() {
    let stack = net::network_stack().await.unwrap();

    stack.wait_config_up().await;

    let mut rx_meta = [PacketMetadata::EMPTY; 16];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 16];
    let mut tx_buffer = [0; 4096];

    loop {
        let socket = UdpSocket::new(
            stack,
            &mut rx_meta,
            &mut rx_buffer,
            &mut tx_meta,
            &mut tx_buffer,
        );

        // if let Err(e) = socket.bind(1234) {
        //     info!("bind error: {:?}", e);
        //     continue;
        // }

        info!("IP: {:?}", stack.config_v4().unwrap().address);
        info!("Port: {:?}", socket.endpoint().port);

        let local: SocketAddr = "0.0.0.0:1234".parse().unwrap();
        let unconnected = match udp_nal::UnconnectedUdp::bind_multiple(socket, local).await {
            Ok(unconnected_udp) => unconnected_udp,
            Err(_) => {
                info!("bind error");
                break;
            }
        };

        let remote: SocketAddr = "10.42.0.1:1884".parse().unwrap();

        let mut mqtt_sn = MqttSn::new(unconnected, local, remote);

        info!("Listening on UDP:1234...");

        loop {
            info!("...attempt connection");

            match mqtt_sn.connect(60000, b"test", false, false, false).await {
                Ok(_) => break,
                Err(_) => continue,
            };
        }

        info!("...connected!");

        info!("subscribing...");

        mqtt_sn
            .subscribe(Topic::from_long(b"LOOONG"), false, QoS::Zero)
            .await
            .unwrap();

        loop {
            info!("expecting message...");
            match mqtt_sn.expect_message().await {
                None => continue,
                Some(msg) => {
                    if msg == b"end" {
                        info!("... message is 'end'");
                        break;
                    }
                }
            }
        }

        break;
    }
}
