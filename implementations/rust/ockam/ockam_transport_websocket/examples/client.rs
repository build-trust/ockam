// This node routes a message, to a worker on a different node, over the websocket transport.

#[macro_use]
extern crate tracing;

use futures_util::{SinkExt, StreamExt};
use tokio::io::AsyncReadExt;
use tokio::time::{sleep, timeout, Duration};

use ockam_core::{route, Result};
use ockam_node::Context;
use ockam_transport_core::TransportError;
use ockam_transport_websocket::{WebSocketTransport, WS};

#[ockam_macros::node]
async fn main(mut ctx: Context) -> Result<()> {
    let peer_addr = get_peer_addr();

    let _try_connect = {
        let ws = WebSocketTransport::create(&ctx).await?;
        let connect_fut = async {
            loop {
                if ws.connect(&peer_addr).await.is_ok() {
                    break;
                }
                sleep(Duration::from_secs(1)).await;
            }
        };
        if timeout(Duration::from_secs(10), connect_fut).await.is_err() {
            error!("Couldn't connect to {}", peer_addr);
            return ctx.stop().await;
        }
        debug!("Connected to {peer_addr}");
    };

    let (stdin_tx, mut stdin_rx) = futures_channel::mpsc::channel(1);
    tokio::spawn(read_stdin(stdin_tx));

    let route = route![(WS, peer_addr.as_str()), "echoer"];
    while let Some(data) = stdin_rx.next().await {
        if ctx.send(route.clone(), data).await.is_err() {
            error!("Failed to send data");
            break;
        }
        match ctx.receive::<String>().await {
            Err(_) => {
                warn!("Failed to receive data");
                continue;
            }
            Ok(msg) => {
                info!("Received echo: '{}'", msg);
                msg
            }
        };
    }

    ctx.stop().await
}

async fn read_stdin(mut tx: futures_channel::mpsc::Sender<Vec<u8>>) -> Result<()> {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut msg = vec![0; 256];
        match stdin.read(&mut msg).await {
            Err(_) | Ok(0) => {
                warn!("Empty stdin");
                tx.close_channel();
                break;
            }
            Ok(n) => msg.truncate(n),
        };

        if tx.is_closed() {
            error!("Stdin channel is closed");
            break;
        }

        let msg = Vec::from(String::from_utf8(msg).unwrap().trim());
        tx.send(msg).await.map_err(|_| TransportError::GenericIo)?;
    }
    Ok(())
}

fn get_peer_addr() -> String {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        .unwrap_or_else(|| "127.0.0.1:10222".to_string())
}
