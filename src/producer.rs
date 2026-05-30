use core::ops::DerefMut;

use alloc::boxed::Box;

use alloc::{collections::btree_map::BTreeMap, format, string::String};
use derive_more::{Display, Error};
use derive_new::new;
use embassy_futures::select::{
    Either::{First, Second},
    select,
};
use tiny_artnet::{Art, Dmx, Poll};
use tracing::{info, instrument, warn};

use crate::{io::AsyncIo, *};

#[derive(Debug, Error, Display)]
pub struct ProducerError {
    message: String,
}

#[derive(new, Debug)]
struct AddressInfo<A> {
    #[new(value = "1")]
    seq: u8,
    addr: A,
}

#[derive(new)]
pub struct ArtnetProducer<IO: AsyncIo> {
    io: IO,
    rx: Receiver<ArtnetEvent>,
    #[new(default)]
    addresses: BTreeMap<NetAddress, AddressInfo<IO::Addr>>,
}

impl From<tiny_artnet::Error<'_>> for ProducerError {
    fn from(value: tiny_artnet::Error<'_>) -> Self {
        ProducerError {
            message: format!("{value:?}"),
        }
    }
}

impl<IO: AsyncIo> ArtnetProducer<IO>
where
    IO::Error: core::error::Error + 'static,
{
    #[instrument(skip_all, err)]
    pub async fn run(mut self) -> DynResult {
        loop {
            let io = self.io.recv();
            let rx_recv = self.rx.recv();
            match select(io, rx_recv).await {
                First(Ok((data, from))) => self.handle_socket_recv(&data, from).await?,
                Second(Ok(ArtnetEvent::Data { address, data })) => {
                    self.handle_send_data(address, data).await?
                }
                First(Err(e)) => warn!(?e, "udp error"),
                Second(Err(e)) => warn!(?e, "recv error"),
            }
        }
    }

    #[instrument(skip_all, err)]
    async fn handle_socket_recv(&mut self, buffer: &[u8], from: IO::Addr) -> DynResult {
        let command = tiny_artnet::from_slice(&buffer).map_err(ProducerError::from)?;

        match command {
            Art::PollReply(poll_reply) => {
                let address = NetAddress::from(poll_reply);
                if let Some(info) = self.addresses.get_mut(&address) {
                    info.addr = from;
                } else {
                    self.addresses.insert(address, AddressInfo::new(from));
                }
            }
            command => {
                warn!("unhandled art command: {command:?}");
            }
        }
        OK
    }

    #[instrument(skip_all, err)]
    async fn handle_send_data(&mut self, address: Address, data: Bytes) -> DynResult {
        if let Some(address_info) = self.addresses.get_mut(&address.net) {
            // increment the sequence
            address_info.seq = address_info.seq.wrapping_add(1);

            // send the data
            let art = Art::Dmx(Dmx {
                sequence: address_info.seq,
                physical: 0,
                port_address: address.into(),
                data: &data,
            });
            let mut buffer = Box::new([0_u8; 1024]);
            let len = art.serialize(buffer.deref_mut());
            self.io.send(address_info.addr, &buffer[0..len]).await?;
        } else {
            info!("polling for address");
            // we don't know where this universe is, so
            // broadcast a poll request
            let art = Art::Poll(Poll {
                // todo: target ports
                ..Default::default()
            });
            let mut buffer = Box::new([0_u8; 1024]);
            let len = art.serialize(buffer.deref_mut());
            self.io.send(IO::broadcast_addr(), &buffer[0..len]).await?;
        }
        OK
    }
}
