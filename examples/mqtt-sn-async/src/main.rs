#![no_main]
#![no_std]

mod pins;
use ariel_os::{
    debug::log::*,
    gpio::{Level, Output},
    net,
};
use ariel_os_mqttsn_async::{ACTION_CHANNEL, Action, Topic, flags::QoS, udp_nal};
use core::net::SocketAddr;
use embassy_net::udp::{PacketMetadata, UdpSocket};

#[ariel_os::task(autostart)]
async fn mqtt_sn_test() {
    ariel_os::asynch::spawner()
        .spawn(ariel_os_mqttsn_async::client())
        .unwrap();
    info!("mqtt_sn_test()");
    ACTION_CHANNEL
        .send(Action::Subscribe(Topic::from_long("my_topic")))
        .await;
}

// #[ariel_os::task(autostart, peripherals)]
// async fn mqtt_sn_light(peripherals: pins::LedPeripherals) {
//     let mut led = Output::new(peripherals.led, Level::Low);

//     let stack = net::network_stack().await.unwrap();
//     stack.wait_config_up().await;

//     let mut rx_meta = [PacketMetadata::EMPTY; 16];
//     let mut rx_buffer = [0; 4096];
//     let mut tx_meta = [PacketMetadata::EMPTY; 16];
//     let mut tx_buffer = [0; 4096];

//     loop {
//         let socket = UdpSocket::new(
//             stack,
//             &mut rx_meta,
//             &mut rx_buffer,
//             &mut tx_meta,
//             &mut tx_buffer,
//         );

//         info!("IP: {:?}", stack.config_v4().unwrap().address);
//         info!("Port: {:?}", socket.endpoint().port);

//         let local: SocketAddr = "0.0.0.0:1234".parse().unwrap();
//         let remote: SocketAddr = "10.42.0.1:1884".parse().unwrap();
//         let unconnected = match udp_nal::UnconnectedUdp::bind_multiple(socket, local).await {
//             Ok(unconnected_udp) => unconnected_udp,
//             Err(_) => {
//                 info!("bind error");
//                 break;
//             }
//         };
//         let mqttsn_socket = ariel_os_mqttsn::MqttSnSocket {
//             socket: unconnected,
//             local,
//             remote,
//         };

//         ariel_os_mqttsn::init(
//             64u16,
//             b"async-client",
//             mqttsn_socket,
//             client_sender,
//             client_receiver,
//         );

//         ariel_os_mqttsn::create_handler(|data| {});

//         info!("Listening on UDP:1234...");

//         loop {
//             info!("...attempt connection");

//             match mqtt_sn.connect(60000, b"light", false, false, false).await {
//                 Ok(_) => break,
//                 Err(_) => continue,
//             };
//         }

//         info!("...connected!");

//         info!("Subscribing...");
//         mqtt_sn
//             .subscribe(Topic::from_long(b"light"), false, QoS::Zero)
//             .await
//             .unwrap();

//         info!("...subscribed!");

//         info!("Awaiting LED signal...");

//         loop {
//             match subscriber.try_next_message() {
//                 Some(msg) => match msg {
//                     b"0" => {
//                         led.set_low();
//                         info!("LED turned off");
//                     }
//                     b"1" => {
//                         led.set_high();
//                         info!("LED turned on");
//                     }
//                     _ => {
//                         continue;
//                     }
//                 },
//                 None => Timer::after_millis(100).await,
//             }

//             // info!("expecting message...");
//             // let msg = match mqtt_sn.expect_message().await {
//             //     None => continue,
//             //     Some(msg) => msg,
//             // };
//             //
//             // info!("Received: {:?}", msg);
//             //
//             // match msg {
//             //     b"0" => {
//             //         led.set_low();
//             //         info!("LED turned off");
//             //     }
//             //     b"1" => {
//             //         led.set_high();
//             //         info!("LED turned on");
//             //     }
//             //     _ => {
//             //         continue;
//             //     }
//             // }
//         }
//     }
// }
