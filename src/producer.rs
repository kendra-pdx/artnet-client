use core::{net::SocketAddr, ops::DerefMut};

use alloc::{collections::btree_map::BTreeMap, format, string::String};
use bytes::BytesMut;
use derive_more::{Display, Error};
use derive_new::new;
use edge_net::nal::{UdpReceive, UdpSend};
use embassy_futures::select::{
    Either::{First, Second},
    select,
};
use tiny_artnet::{Art, Dmx, Poll};
use tracing::{info, instrument, warn};

use crate::*;

#[derive(Debug, Error, Display)]
pub struct ProducerError {
    message: String,
}

#[derive(new, Debug)]
struct AddressInfo {
    #[new(value = "1")]
    seq: u8,
    addr: SocketAddr,
}

#[derive(new)]
pub struct ArtnetProducer<UDP> {
    socket: UDP,
    rx: Receiver<ArtnetEvent>,
    #[new(default)]
    addresses: BTreeMap<NetAddress, AddressInfo>,
}

impl From<tiny_artnet::Error<'_>> for ProducerError {
    fn from(value: tiny_artnet::Error<'_>) -> Self {
        ProducerError {
            message: format!("{value:?}"),
        }
    }
}

impl<UDP: UdpReceive + UdpSend> ArtnetProducer<UDP>
where
    UDP::Error: 'static,
{
    #[instrument(skip_all, err)]
    pub async fn run(mut self) -> DynResult {
        loop {
            let mut socket_buffer = BytesMut::new();
            socket_buffer.resize(1024, 0);
            let socket_recv = self.socket.receive(socket_buffer.deref_mut());
            let rx_recv = self.rx.recv();
            match select(socket_recv, rx_recv).await {
                First(Ok((n, from))) => self.handle_socket_recv(n, &socket_buffer, from).await?,
                Second(Ok(ArtnetEvent::Data { address, data })) => {
                    self.handle_send_data(address, data).await?
                }
                First(Err(e)) => warn!(?e, "udp error"),
                Second(Err(e)) => warn!(?e, "recv error"),
            }
        }
    }

    #[instrument(skip_all, err)]
    async fn handle_socket_recv(&mut self, n: usize, buffer: &[u8], from: SocketAddr) -> DynResult {
        let command = tiny_artnet::from_slice(&buffer[0..n]).map_err(ProducerError::from)?;

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
            self.socket.send(address_info.addr, &buffer[0..len]).await?;
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
            self.socket.send(ARTNET_BROADCAST, &buffer[0..len]).await?;
        }
        OK
    }
}
