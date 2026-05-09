use core::{net::SocketAddr, ops::DerefMut};

use alloc::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    string::{String, ToString},
};
use artnet_protocol::{ArtCommand, Output, Poll, PollReply};
use bytes::Bytes;
use derive_new::new;
use edge_net::nal::{UdpReceive, UdpSend};
use thiserror::Error;
use tracing::{debug, instrument, warn};

use crate::*;

#[derive(Debug, Error)]
#[error("{self:?}")]
pub struct ArtnetReceiverError {
    message: String,
}

#[derive(new)]
pub struct ArtnetReceiver<UDP> {
    socket: UDP,
    tx: Sender<ArtnetEvent>,
    address: AddressRange,
    #[new(default)]
    seq: BTreeMap<Address, u8>,
}

impl<UDP: UdpSend + UdpReceive> ArtnetReceiver<UDP>
where
    UDP::Error: 'static,
{
    #[instrument(skip(self))]
    pub async fn run(mut self) -> DynResult {
        const BUFFER_SIZE: usize = 1024;
        let mut buffer = Box::new([0_u8; BUFFER_SIZE]);
        loop {
            let (n, reply_to) = self.socket.receive(buffer.deref_mut()).await?;
            assert!(n < BUFFER_SIZE, "artnet command exceeded buffer size");
            let command = ArtCommand::from_buffer(&buffer[..n])?;
            match command {
                ArtCommand::Output(output) => self.handle_output(output).await?,
                ArtCommand::Poll(poll) => self.handle_poll(reply_to, poll).await?,
                command => {
                    warn!(?command, "unimplemented command");
                }
            }
        }
    }

    #[instrument(skip_all)]
    async fn handle_poll(&mut self, reply_to: SocketAddr, _poll: Poll) -> DynResult {
        debug!("handling poll command");

        let poll_reply = PollReply {
            num_ports: self.address.length.to_le_bytes(),
            port_address: self.address.base.into(),
            ..Default::default()
        };
        let command = ArtCommand::PollReply(Box::new(poll_reply));
        let buffer = command.write_to_buffer()?;
        self.socket.send(reply_to, &buffer).await?;
        OK
    }

    #[instrument(skip_all)]
    async fn handle_output(&mut self, output: Output) -> DynResult {
        debug!("handling output command");

        let address = Address::from(output.port_address);
        let seq = if let Some(prev_seq) = self.seq.get(&address) {
            let curr_seq = output.sequence;
            if curr_seq > *prev_seq || prev_seq - curr_seq > 127 {
                // either the sequence has advanced, or wrapped around
                Some(curr_seq)
            } else {
                // out of order
                None
            }
        } else {
            // never seen the sequence before
            Some(output.sequence)
        };

        if let Some(seq) = seq {
            // send the data
            let data = Bytes::from(output.data.as_ref().clone());
            let event = ArtnetEvent::Data { address, data };
            self.tx.send(event).await.map_err(|e| {
                Box::new(ArtnetReceiverError {
                    message: e.to_string(),
                })
            })?;

            // update the sequence
            self.seq.insert(address, seq);
        }

        OK
    }
}
