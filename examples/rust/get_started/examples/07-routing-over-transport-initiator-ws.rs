// This node routes a message, to a worker on a different node, over the websocket transport.

#[macro_use]
extern crate tracing;

use futures_util::StreamExt;
use ockam::{route, Context, Result, WebSocketError, WebSocketTransport, WS};
use tokio::io::AsyncReadExt;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let peer_addr = "127.0.0.1:4000";

    let ws = WebSocketTransport::create(&ctx).await?;

    if let Err(_) = tokio::time::timeout(tokio::time::Duration::from_secs(10), async {
        loop {
            if ws.connect(peer_addr).await.is_ok() {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    })
    .await
    {
        error!("Couldn't connect to {}", peer_addr);
        return ctx.stop().await;
    }

    let (stdin_tx, mut stdin_rx) = futures_channel::mpsc::unbounded();
    tokio::spawn(read_stdin(stdin_tx));

    let route = route![(WS, peer_addr), "echoer"];
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
