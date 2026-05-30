// #![no_std]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

use core::fmt::Debug;

use async_channel::{Receiver, Sender};
use cfg_if::cfg_if;

mod address;
pub mod io;

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

use bytes::Bytes;

#[cfg(any(feature = "producer", feature = "receiver"))]
pub(crate) type DynError = alloc::boxed::Box<dyn core::error::Error>;
#[cfg(any(feature = "producer", feature = "receiver"))]
pub(crate) type DynResult<T = ()> = Result<T, DynError>;
#[cfg(any(feature = "producer", feature = "receiver"))]
pub(crate) const OK: DynResult = Ok(());

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize)
)]
pub enum ArtnetEvent {
    Data { address: Address, data: Bytes },
}

const EVENT_BUFFER: usize = 1;
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
