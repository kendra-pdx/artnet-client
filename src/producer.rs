use core::{net::SocketAddr, ops::DerefMut};

use alloc::collections::btree_map::BTreeMap;
use artnet_protocol::{ArtCommand, Output, PaddedData, Poll};
use bytes::BytesMut;
use derive_new::new;
// use edge_nal_embassy::UdpSocket;
use edge_net::nal::{UdpReceive, UdpSend};
use embassy_futures::select::{
    Either::{First, Second},
    select,
};
use tracing::warn;

use crate::*;

#[derive(new)]
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
    addresses: BTreeMap<Address, AddressInfo>,
}

impl<UDP: UdpReceive + UdpSend> ArtnetProducer<UDP>
where
    UDP::Error: 'static,
{
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

    async fn handle_socket_recv(&mut self, n: usize, buffer: &[u8], from: SocketAddr) -> DynResult {
        let command = ArtCommand::from_buffer(&buffer[..n])?;

        match command {
            ArtCommand::PollReply(poll_reply) => {
                let n_ports = u16::from_le_bytes(poll_reply.num_ports);
                let addresses = Address::from(poll_reply.port_address).as_range(n_ports);
                for address in addresses {
                    if let Some(info) = self.addresses.get_mut(&address) {
                        info.addr = from;
                    } else {
                        self.addresses.insert(address, AddressInfo::new(from));
                    }
                }
            }
            command => {
                warn!("unhandled art command: {command:?}");
            }
        }
        OK
    }

    async fn handle_send_data(&mut self, address: Address, data: Bytes) -> DynResult {
        if let Some(address_info) = self.addresses.get_mut(&address) {
            // increment the sequence
            address_info.seq = address_info.seq.wrapping_add(1);

            // send the data
            let command = ArtCommand::Output(Output {
                sequence: address_info.seq,
                data: PaddedData::from(data.to_vec()),
                ..Default::default()
            });

            let data = command.write_to_buffer()?;
            self.socket.send(address_info.addr, &data).await?;
        } else {
            // we don't know where this universe is, so
            // broadcast a poll request
            let command = ArtCommand::Poll(Poll {
                ..Default::default()
            });
            let data = command.write_to_buffer()?;
            self.socket.send(ARTNET_BROADCAST, &data).await?;
        }
        OK
    }
}
