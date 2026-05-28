use bytes::Bytes;

#[cfg(feature = "io-udp")]
mod edge_udp;

#[cfg(feature = "io-wpan")]
mod esp_ieee802154;

pub(crate) mod sealed {
    pub trait Sealed {}
}

#[allow(async_fn_in_trait)]
pub trait AsyncIo: sealed::Sealed {
    type Addr: Copy;
    type Error;
    fn broadcast_addr() -> Self::Addr;
    async fn recv(&mut self) -> Result<(Bytes, Self::Addr), Self::Error>;
    async fn send(&mut self, to: Self::Addr, data: &[u8]) -> Result<(), Self::Error>;
}
