// This node routes a message, to a worker on a different node, over the websocket transport.

#[macro_use]
extern crate tracing;

use futures_util::StreamExt;
use ockam::{route, Context, Result};
use tokio::io::AsyncReadExt;
use tokio::time::{sleep, timeout, Duration};

use ockam_transport_websocket::{WebSocketError, WebSocketTransport, WS};

#[ockam::node]
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
    };

    let (stdin_tx, mut stdin_rx) = futures_channel::mpsc::unbounded();
    tokio::spawn(read_stdin(stdin_tx));

    let route = route![(WS, peer_addr.as_str()), "echoer"];
    while let Some(data) = stdin_rx.next().await {
        if ctx.send(route.clone(), data).await.is_err() {
            error!("Failed to send data");
            break;
        }
        match ctx.receive::<String>().await {
            Err(_) => {
                error!("Failed to receive data");
                break;
            }
            Ok(msg) => {
                info!("Received echo: '{}'", msg);
                msg
            }
        };
    }

    ctx.stop().await
}

async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Vec<u8>>) -> Result<()> {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 10];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => {
                tx.close_channel();
                break;
            }
            Ok(n) => n,
        };

        if tx.is_closed() {
            error!("Stdin channel is closed");
            break;
        }

        buf.truncate(n);
        let buf = String::from_utf8(buf).unwrap();
        let msg = Vec::from(buf.trim());
        tx.unbounded_send(msg).map_err(WebSocketError::from)?;
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
