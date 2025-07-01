#![no_main]
#![no_std]

mod pins;
use ariel_os::{
    debug::log::*,
    gpio::{Input, Pull},
    net,
    time::{Duration, Timer},
};
use ariel_os_mqttsn::{udp_nal, MqttSn, Topic};
use core::net::SocketAddr;
use embassy_net::udp::{PacketMetadata, UdpSocket};

#[ariel_os::task(autostart, peripherals)]
async fn mqtt_sn_button(peripherals: pins::Peripherals) {
    let stack = net::network_stack().await.unwrap();

    let mut toggle = true;
    let pull = Pull::Up;
    let button = Input::builder(peripherals.btn1, pull)
        .build_with_interrupt()
        .unwrap();

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

            match mqtt_sn.connect(60000, b"button", false, false, true).await {
                Ok(_) => break,
                Err(_) => continue,
            };
        }

        info!("...connected!");

        info!("Registering topic...");

        let topic_id: Topic = match mqtt_sn.register(b"light").await {
            Ok(id) => id,
            Err(_) => break,
        };

        info!("Registered: {:?}", topic_id);

        info!("Awaiting user input...");

        loop {
            if button.is_low() {
                let msg = match toggle {
                    true => b"1",
                    false => b"0",
                };

                match mqtt_sn.publish(&topic_id, msg).await {
                    Ok(_) => info!("Published message: {:?} ", msg),
                    Err(_) => {
                        info!("Error occurred");
                        continue;
                    }
                }

                toggle = !toggle;

                Timer::after(Duration::from_millis(1000)).await;
            }
        }
    }
}
