use core::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use bytes::Bytes;
use edge_net::nal::{UdpReceive, UdpSend};

use crate::io::{AsyncIo, sealed::Sealed};

pub const ARTNET_PORT: u16 = 6454;
pub const ARTNET_BROADCAST: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::BROADCAST, ARTNET_PORT));

impl<UDP: UdpSend + UdpReceive> Sealed for UDP {}

impl<UDP: UdpSend + UdpReceive> AsyncIo for UDP {
    type Addr = core::net::SocketAddr;
    type Error = UDP::Error;

    fn broadcast_addr() -> Self::Addr {
        ARTNET_BROADCAST
    }

    async fn recv(&mut self) -> Result<(Bytes, Self::Addr), Self::Error> {
        let mut buffer = [0; 1024];
        let (n, addr) = self.receive(&mut buffer).await?;
        let data = Bytes::copy_from_slice(&buffer[0..n]);
        Ok((data, addr))
    }

    async fn send(&mut self, to: Self::Addr, data: &[u8]) -> Result<(), Self::Error> {
        <Self as UdpSend>::send(self, to, data).await?;
        Ok(())
    }
}
