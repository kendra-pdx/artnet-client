use core::{net::SocketAddr, ops::DerefMut};

use alloc::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    format,
    string::{String, ToString},
};
use derive_more::From;
use derive_new::new;
use edge_net::nal::{UdpReceive, UdpSend};
use thiserror::Error;
use tiny_artnet::{Art, Dmx, Poll, PollReply};
use tracing::{debug, instrument, warn};

use crate::*;

#[derive(Debug, Error, From)]
#[error("{self:?}")]
pub struct ArtnetReceiverError {
    #[from(into)]
    message: String,
}

#[derive(new)]
pub struct ArtnetReceiver<UDP> {
    socket: UDP,
    tx: Sender<ArtnetEvent>,
    address: NetAddress,
    #[new(default)]
    seq: BTreeMap<Address, u8>,
}

impl<UDP: UdpSend + UdpReceive> ArtnetReceiver<UDP>
where
    UDP::Error: 'static,
{
    #[instrument(skip(self), err)]
    pub async fn run(mut self) -> DynResult {
        const BUFFER_SIZE: usize = 1024;
        let mut buffer = Box::new([0_u8; BUFFER_SIZE]);
        loop {
            let (n, reply_to) = self.socket.receive(buffer.deref_mut()).await?;
            assert!(n < BUFFER_SIZE, "artnet command exceeded buffer size");
            let command = tiny_artnet::from_slice(&buffer[..n])
                .map_err(|e| ArtnetReceiverError::from(format!("{e:?}")))?;
            match command {
                Art::Dmx(dmx) => self.handle_dmx(dmx).await?,
                Art::Poll(poll) => self.handle_poll(reply_to, poll).await?,
                command => {
                    warn!(?command, "unimplemented command");
                }
            }
        }
    }

    #[instrument(skip_all, err)]
    async fn handle_poll(&mut self, reply_to: SocketAddr, _poll: Poll) -> DynResult {
        debug!("handling poll command");

        let poll_reply = PollReply {
            net_switch: self.address.net,
            sub_switch: self.address.sub_net,
            ..Default::default()
        };
        let art = Art::PollReply(poll_reply);
        let mut buffer = Box::new([0_u8; 1024]);
        let len = art.serialize(buffer.deref_mut());
        self.socket.send(reply_to, &buffer[0..len]).await?;
        OK
    }

    #[instrument(skip_all, err)]
    async fn handle_dmx(&mut self, dmx: Dmx<'_>) -> DynResult {
        debug!("handling dmx");

        let address = Address::from(dmx.port_address);
        let seq = if let Some(prev_seq) = self.seq.get(&address) {
            let curr_seq = dmx.sequence;
            if curr_seq > *prev_seq || prev_seq - curr_seq > 127 {
                // either the sequence has advanced, or wrapped around
                Some(curr_seq)
            } else {
                // out of order
                None
            }
        } else {
            // never seen the sequence before
            Some(dmx.sequence)
        };

        if let Some(seq) = seq {
            // send the data
            let data = Bytes::copy_from_slice(dmx.data);
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
