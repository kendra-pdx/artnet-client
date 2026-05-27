use bytes::Bytes;

mod edge_udp;
pub use edge_udp::*;

#[cfg(feature = "ieee802154")]
mod esp_ieee802154;

#[allow(async_fn_in_trait)]
pub trait AsyncIo {
    type Addr;
    type Error;
    async fn recv(&mut self) -> Result<(Bytes, Self::Addr), Self::Error>;
    async fn send(&mut self, to: Self::Addr, data: &[u8]) -> Result<(), Self::Error>;
}
