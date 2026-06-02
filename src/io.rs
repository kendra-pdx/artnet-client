use bytes::Bytes;

#[cfg(feature = "io-udp")]
pub mod edge_udp;

#[cfg(feature = "io-wpan")]
pub mod esp_ieee802154;

#[allow(async_fn_in_trait)]
pub trait AsyncIo {
    type Addr: Copy;
    type Error;
    fn broadcast_addr() -> Self::Addr;
    async fn recv(&mut self) -> Result<(Bytes, Self::Addr), Self::Error>;
    async fn send(&mut self, to: Self::Addr, data: &[u8]) -> Result<(), Self::Error>;
}
