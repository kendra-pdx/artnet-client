// #![no_std]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

use core::fmt::Debug;
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

impl Debug for ArtnetEvent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Data { address, data } => f
                .debug_struct("Data")
                .field("address", address)
                .field("data.len", &data.len())
                .finish(),
        }
    }
}
