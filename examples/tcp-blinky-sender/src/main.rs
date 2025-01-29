#![no_main]
#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(used_with_arg)]

use ariel_os::{
    debug::log::*,
    net,
    reexports::embassy_net::{self, tcp::TcpSocket, IpAddress},
    time::{Duration, Timer},
};
use embedded_io_async::Write;

#[ariel_os::config(network)]
const NETWORK_CONFIG: ariel_os::reexports::embassy_net::Config = {
    use ariel_os::reexports::embassy_net::{self, Ipv4Address};

    embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(Ipv4Address::new(10, 42, 0, 61), 24),
        dns_servers: heapless::Vec::new(),
        gateway: Some(Ipv4Address::new(10, 42, 0, 1)),
    })
};

#[ariel_os::task(autostart)]
async fn tcp_blinky_sender() {
    let stack = net::network_stack().await.unwrap();

    let mut rx_buffer = [0; 4096];
    let mut tx_buffer = [0; 4096];

    let mut toggle = true;

    loop {
        stack.wait_config_up().await;

        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(10)));

        // info!("Listening on TCP:1234...");
        // if let Err(e) = socket.accept(1234).await {
        //     info!("accept error: {:?}", e);
        //     continue;
        // }

        info!("Connecting to 10.42.0.62:1234...");
        if let Err(e) = socket
            .connect(embassy_net::IpEndpoint::new(
                IpAddress::v4(10, 42, 0, 62),
                1234,
            ))
            .await
        {
            info!("connect error: {:?}", e);
            continue;
        }

        info!("connected to {:?}", socket.remote_endpoint());

        loop {
            let msg = match toggle {
                false => b"0",
                true => b"1",
            };

            match socket.write_all(msg).await {
                Ok(()) => {
                    info!("sent: {}", msg);
                }
                Err(e) => {
                    info!("write error: {:?}", e);
                    break;
                }
            };

            toggle = !toggle;
            Timer::after(Duration::from_millis(1000)).await;
        }
    }
}
