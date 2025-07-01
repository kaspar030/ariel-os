#![no_main]
#![no_std]

mod pins;
use ariel_os::{
    debug::log::*,
    gpio::{Level, Output},
    net,
};
use ariel_os_mqttsn::{flags::QoS, udp_nal, MqttSn, Topic};
use core::net::SocketAddr;
use embassy_net::udp::{PacketMetadata, UdpSocket};

#[ariel_os::task(autostart, peripherals)]
async fn mqtt_sn_light(peripherals: pins::LedPeripherals) {
    let mut led = Output::new(peripherals.led, Level::Low);

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

            match mqtt_sn.connect(60000, b"light", false, false, false).await {
                Ok(_) => break,
                Err(_) => continue,
            };
        }

        info!("...connected!");

        info!("Subscribing...");
        mqtt_sn
            .subscribe(Topic::from_long(b"light"), false, QoS::Zero)
            .await
            .unwrap();

        info!("...subscribed!");

        info!("Awaiting LED signal...");

        loop {
            info!("expecting message...");
            let msg = match mqtt_sn.expect_message().await {
                None => continue,
                Some(msg) => msg,
            };

            info!("Received: {:?}", msg);

            match msg {
                b"0" => {
                    led.set_low();
                    info!("LED turned off");
                }
                b"1" => {
                    led.set_high();
                    info!("LED turned on");
                }
                _ => {
                    continue;
                }
            }
        }
    }
}
