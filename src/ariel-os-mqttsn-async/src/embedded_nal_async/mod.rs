use core::net::SocketAddr;

use embassy_net::{udp, IpAddress, IpEndpoint};
use embedded_nal_async as nal;

mod util;
pub use util::Error;
use util::{is_unspec_ip, sockaddr_nal2smol, sockaddr_smol2nal};

pub struct ConnectedUdp<'a> {
    remote: IpEndpoint,
    socket: udp::UdpSocket<'a>,
}

impl nal::ConnectedUdp for ConnectedUdp<'_> {
    type Error = Error;

    async fn send(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.socket.send_to(data, self.remote).await?
    }

    async fn receive_into(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {}
}

pub struct UnconnectedUdp<'a> {
    socket: udp::UdpSocket<'a>,
}

pub struct UdpStack;

impl nal::UdpStack for UdpStack {
    type Error = Error;
    type Connected: ConnectedUdp<Error = Self::Error>;
    type UniquelyBound: UnconnectedUdp<Error = Self::Error>;
    type MultiplyBound: UnconnectedUdp<Error = Self::Error>;

    async fn connect_from(
        &self,
        local: SocketAddr,
        remote: SocketAddr,
    ) -> Result<(SocketAddr, Self::Connected), Self::Error> {
    }

    async fn bind_single(
        &self,
        local: SocketAddr,
    ) -> Result<(SocketAddr, Self::UniquelyBound), Self::Error> {
    }

    async fn bind_multiple(&self, local: SocketAddr) -> Result<Self::MultiplyBound, Self::Error> {}
}
