use core::ops::DerefMut;

use alloc::{
    boxed::Box,
    collections::btree_map::BTreeMap,
    string::{String, ToString},
};
use derive_more::From;
use derive_new::new;
use thiserror::Error;
use tiny_artnet::{Art, Dmx, Poll, PollReply};
use tracing::{debug, instrument, warn};

use crate::{io::AsyncIo, *};

#[derive(Debug, Error, From)]
#[error("{self:?}")]
pub struct ArtnetReceiverError {
    #[from(into)]
    message: String,
}

#[derive(new)]
pub struct ArtnetReceiver<IO> {
    io: IO,
    tx: Sender<ArtnetEvent>,
    address: NetAddress,
    #[new(default)]
    seq: BTreeMap<Address, u8>,
}

impl<IO: AsyncIo> ArtnetReceiver<IO>
where
    IO::Error: core::error::Error + 'static,
{
    #[instrument(skip(self), err)]
    pub async fn run(mut self) -> DynResult {
        loop {
            let (data, reply_to) = self.io.recv().await?;
            if let Ok(command) = tiny_artnet::from_slice(&data) {
                match command {
                    Art::Dmx(dmx) => self.handle_dmx(dmx).await?,
                    Art::Poll(poll) => self.handle_poll(reply_to, poll).await?,
                    command => {
                        warn!(?command, "unimplemented command");
                    }
                }
            } else {
                warn!("buffer did not contain artnet data");
            }
        }
    }

    #[instrument(skip_all, err)]
    async fn handle_poll(&mut self, reply_to: IO::Addr, _poll: Poll) -> DynResult {
        debug!("handling poll command");

        let poll_reply = PollReply {
            net_switch: self.address.net,
            sub_switch: self.address.sub_net,
            ..Default::default()
        };
        let art = Art::PollReply(poll_reply);
        let mut buffer = Box::new([0_u8; 1024]);
        let len = art.serialize(buffer.deref_mut());
        self.io.send(reply_to, &buffer[0..len]).await?;
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
