mod shared;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::anyhow;
use artnet_client::{ArtnetEvent, ArtnetReceiver, NetAddress, Universe, io::edge_udp::ARTNET_PORT};
use async_channel::Receiver;
use edge_net::nal::UdpBind;
use embassy_futures::select::select;
use iocraft::prelude::*;
use smol::stream::StreamExt;
use smol_macros::main;

main! {
    async fn main() {
        shared::setup_tracing();
        app().await.unwrap()
    }
}

async fn app() -> anyhow::Result<()> {
    let stack = edge_nal_std::Stack::new();

    let (tx, rx) = ArtnetEvent::channel();

    let local = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), ARTNET_PORT);
    let socket = stack.bind(local).await?;

    let receiver = ArtnetReceiver::new(socket, tx, NetAddress::new(0, 0));

    let producer_task = receiver.run();
    let display_task = display(rx);

    match select(producer_task, display_task).await {
        embassy_futures::select::Either::First(r) => r.map_err(|e| anyhow!(e.to_string()))?,
        embassy_futures::select::Either::Second(r) => r?,
    }

    Ok(())
}

async fn display(rx: Receiver<ArtnetEvent>) -> anyhow::Result<()> {
    element!(App(rx)).render_loop().await?;
    Ok(())
}

#[derive(Default, Props)]
struct AppProps {
    rx: Option<Receiver<ArtnetEvent>>,
}

#[component]
fn App<'a>(props: &AppProps, mut hooks: Hooks) -> impl Into<AnyElement<'a>> {
    let mut image = hooks.use_state(|| vec![]);
    let rx = props.rx.clone().expect("rx is required");

    hooks.use_future({
        let mut rx = Box::pin(rx);
        async move {
            while let Some(e) = rx.next().await {
                match e {
                    ArtnetEvent::Data { address, data } => {
                        if address.universe == Universe::from(0x01) {
                            let new_image: Vec<[u8; 3]> =
                                data.chunks(3).map(|c| [c[0], c[1], c[2]]).collect();
                            image.set(new_image);
                        } else {
                            panic!("unexpected address for artnet data");
                        }
                    }
                }
            }
        }
    });

    element! {
        View() {
            #(image.read().iter().map(|color| {
                element! {
                    Pixel(color: *color)
                }
            }))
        }
    }
}

#[derive(Default, Props)]
struct PixelProps {
    color: [u8; 3],
}

#[component]
fn Pixel<'a>(props: &PixelProps) -> impl Into<AnyElement<'a>> {
    let [r, g, b] = props.color;
    element! {
        Text(color: Color::Rgb { r, g, b }, content: "█")
    }
}
