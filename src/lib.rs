#![no_std]

extern crate alloc;

use core::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use async_channel::{Receiver, Sender};
use cfg_if::cfg_if;

mod address;

pub use address::*;

cfg_if! {
    if #[cfg(feature = "receiver") ] {
        mod receiver;
        pub use receiver::*;
    }
}

cfg_if! {
    if #[cfg(feature = "producer") ] {
        mod producer;
        pub use producer::*;
    }
}

use alloc::boxed::Box;
use bytes::Bytes;

pub(crate) type DynError = Box<dyn core::error::Error>;
pub(crate) type DynResult<T = ()> = Result<T, DynError>;
pub(crate) const OK: DynResult = Ok(());

pub enum ArtnetEvent {
    Data { address: Address, data: Bytes },
}

const EVENT_BUFFER: usize = 1;
pub const ARTNET_PORT: u16 = 6454;
pub const ARTNET_BROADCAST: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::BROADCAST, ARTNET_PORT));

impl ArtnetEvent {
    pub fn channel() -> (Sender<ArtnetEvent>, Receiver<ArtnetEvent>) {
        async_channel::bounded(EVENT_BUFFER)
    }
}
