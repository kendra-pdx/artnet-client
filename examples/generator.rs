mod shared;

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
    u8,
};

use anyhow::anyhow;
use artnet_client::*;
use async_channel::{Receiver, Sender};
use bevy_color::{Color, Hue, Srgba};
use bytes::Bytes;
use edge_net::nal::UdpBind;
use embassy_futures::select::select3;
use iocraft::prelude::*;
use rand::RngExt;
use smol::{Timer, stream::StreamExt};
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
    let (tui_tx, tui_rx) = TuiEvent::channel();

    let local = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);
    let socket = stack.bind(local).await?;

    let receiver = ArtnetProducer::new(socket, rx);

    let receiver_task = receiver.run();
    let generate_task = generate(tx, tui_tx);

    let tui_task = tui(tui_rx);

    match select3(receiver_task, generate_task, tui_task).await {
        embassy_futures::select::Either3::First(r) => r.map_err(|e| anyhow!(e.to_string()))?,
        embassy_futures::select::Either3::Second(r) => r?,
        embassy_futures::select::Either3::Third(r) => r?,
    }

    Ok(())
}

async fn generate(tx: Sender<ArtnetEvent>, tui_tx: Sender<TuiEvent>) -> anyhow::Result<()> {
    const FPS: u64 = 4;
    const FPS_DELAY: Duration = Duration::from_millis(1000 / FPS);

    let mut image: [Color; 16] = [Color::hsv(0.0, 0.9, 0.5); 16];
    let mut rng = rand::rng();
    loop {
        let now = Instant::now();
        for c in image.iter_mut() {
            let hue: f32 = rng.random::<f32>() * 360.0;
            c.set_hue(hue);
        }
        let rgb_data = image.iter().flat_map(|c| {
            const U8_MAX: f32 = u8::MAX as f32;

            let Srgba {
                red, green, blue, ..
            } = c.to_srgba();

            [red, green, blue].into_iter().map(|c| (c * U8_MAX) as u8)
        });
        let data = ArtnetEvent::Data {
            address: Address::from(0x0001),
            data: Bytes::from_iter(rgb_data),
        };

        tx.send(data).await?;
        tui_tx.send(TuiEvent::PacketSent).await?;

        Timer::after(FPS_DELAY - now.elapsed()).await;
    }
}

enum TuiEvent {
    PacketSent,
}

impl TuiEvent {
    fn channel() -> (Sender<TuiEvent>, Receiver<TuiEvent>) {
        async_channel::bounded(4)
    }
}

async fn tui(rx: Receiver<TuiEvent>) -> anyhow::Result<()> {
    element!(App(rx)).render_loop().await?;
    Ok(())
}

#[derive(Default, Props)]
struct AppProps {
    rx: Option<Receiver<TuiEvent>>,
}

#[component]
fn App<'a>(props: &AppProps, mut hooks: Hooks) -> impl Into<AnyElement<'a>> {
    let mut n_packets = hooks.use_state(|| 0);
    let rx = props.rx.clone().expect("rx is required");

    hooks.use_future({
        let mut rx = Box::pin(rx);
        async move {
            while let Some(_) = rx.next().await {
                n_packets.set(n_packets.get() + 1);
            }
        }
    });
    element! {
        View(
            flex_direction: FlexDirection::Row, gap: 1,
            border_color: iocraft::Color::DarkGrey, border_style: BorderStyle::Round,
            padding_left: 1, padding_right: 1) {
            Text(content: "Packets:")
            Text(content: format!("{n_packets:08}"), color: iocraft::Color::Green)
        }
    }
}
