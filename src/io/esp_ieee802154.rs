use core::sync::atomic::AtomicU8;

use bytes::Bytes;
use embassy_futures::yield_now;
use esp_radio::ieee802154::{Frame, Ieee802154};
use ieee802154::mac::{Address, FrameContent, FrameType, FrameVersion, Header};

use crate::io::AsyncIo;

impl AsyncIo for Ieee802154<'_> {
    type Addr = Option<Address>;
    type Error = esp_radio::ieee802154::Error;

    async fn recv(&mut self) -> Result<(bytes::Bytes, Self::Addr), Self::Error> {
        loop {
            match self.received() {
                Some(Ok(received)) if received.frame.content == FrameContent::Data => {
                    let from = received.frame.header.source;
                    let data = Bytes::from_iter(received.frame.payload.into_iter());
                    return Ok((data, from));
                }
                Some(Err(err)) => return Err(err),
                _ => yield_now().await,
            }
        }
    }

    async fn send(&mut self, to: Self::Addr, data: &[u8]) -> Result<(), Self::Error> {
        let frame = frame(data, to);
        self.transmit(&frame, true)?;
        Ok(())
    }
}

fn frame(payload: &[u8], destination: Option<Address>) -> Frame {
    static SEQ: AtomicU8 = AtomicU8::new(0);
    let seq = SEQ.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
    let header: Header = Header {
        frame_type: FrameType::Data,
        frame_pending: false,
        ack_request: false,
        pan_id_compress: false,
        seq_no_suppress: false,
        ie_present: false,
        version: FrameVersion::Ieee802154_2003,
        seq,
        destination,
        source: None,
        auxiliary_security_header: None,
    };

    Frame {
        header,
        content: FrameContent::Data,
        payload: payload.to_vec(),
        footer: [0x00, 0x00],
    }
}
