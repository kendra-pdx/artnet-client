use core::ops::DerefMut;

use bytes::Bytes;
use derive_more::{Deref, DerefMut, From, Into};
use edge_net::nal::{UdpReceive, UdpSend};

use crate::io::AsyncIo;

#[derive(From, Into, Deref, DerefMut)]
pub struct EdgeUdpEmbassySocket<'s>(edge_nal_embassy::UdpSocket<'s>);

impl AsyncIo for EdgeUdpEmbassySocket<'_> {
    type Addr = core::net::SocketAddr;
    type Error = edge_nal_embassy::UdpError;

    async fn recv(&mut self) -> Result<(Bytes, Self::Addr), Self::Error> {
        let mut buffer = [0; 1024];
        let (n, addr) = self.receive(&mut buffer).await?;
        let data = Bytes::copy_from_slice(&buffer[0..n]);
        Ok((data, addr))
    }

    async fn send(&mut self, to: Self::Addr, data: &[u8]) -> Result<(), Self::Error> {
        self.deref_mut().send(to, data).await?;
        Ok(())
    }
}
