use ockam::{stream::Stream, Context, Result};
use ockam_transport_tcp::TcpTransport;
use std::time::Duration;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let hub_addr = "127.0.0.1:4000";

    let tcp = TcpTransport::create(&ctx).await?;
    let serv = tcp.connect(hub_addr).await?.service("stream_service");

    // Create 2 new stream workers
    let (tx, mut rx) = Stream::new(&ctx)?
        .with_interval(Duration::from_millis(500))
        .connect(serv, "test-stream".to_string())
        .await?;

    // Send a message to the stream producer
    ctx.send(tx, "Hello world!".to_string()).await.unwrap();

    // Get the next message from the stream consumer
    let msg = rx.next::<String>().await.unwrap();
    println!("Forwarded: `{}`", msg);
    ctx.stop().await
}
