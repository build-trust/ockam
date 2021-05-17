use futures_util::StreamExt;
use ockam_core::{Result, Route};
use ockam_node::Context;
use ockam_transport_websocket::{WebSocketTransport, TCP};
use tokio::io::AsyncReadExt;

fn main() -> Result<()> {
    let (ctx, mut executor) = ockam_node::start_node();
    executor.execute(async move {
        run_main(ctx).await.unwrap();
    })
}

async fn run_main(mut ctx: Context) -> Result<()> {
    let peer_addr = get_peer_addr();

    let ws = WebSocketTransport::create(&ctx).await?;
    ws.connect(&peer_addr).await?;

    let (stdin_tx, mut stdin_rx) = futures_channel::mpsc::unbounded();
    tokio::spawn(read_stdin(stdin_tx));

    while let Some(data) = stdin_rx.next().await {
        ctx.send(
            Route::new()
                .append_t(TCP, &peer_addr)
                .append("echo_service"),
            data,
        )
        .await?;

        let msg = ctx.receive::<String>().await?;
        println!("Received echo: '{}'", msg);
    }

    ctx.stop().await
}

fn get_peer_addr() -> String {
    std::env::args()
        .skip(1)
        .take(1)
        .next()
        .unwrap_or(format!("127.0.0.1:10222"))
}

async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Vec<u8>>) {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 10];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        let buf = String::from_utf8(buf).unwrap();
        let msg = Vec::from(buf.trim());
        tx.unbounded_send(msg).unwrap();
    }
}
