/// This example uses the stream service to send messages between two
/// clients.  A stream is a buffered message sending channel, which
/// means that you can run `initiator` and `responder` in any order
/// you like.
use ockam::{route, stream::Stream, Context, Result, Route, TcpTransport, TCP};
use std::time::Duration;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect("127.0.0.1:4000").await?;

    // Create the stream client
    let (tx, _rx) = Stream::new(&ctx)?
        .stream_service("stream")
        .index_service("stream_index")
        .client_id("stream-over-cloud-node-initiator")
        .with_interval(Duration::from_millis(100))
        .connect(
            route![(TCP, "127.0.0.1:4000")],
            // Stream name from THIS node to the OTHER node
            "test-a-b",
            // Stream name from the OTHER node to THIS node
            "test-b-a",
        )
        .await?;

    ctx.send(tx.to_route().append("echoer"), "Hello World!".to_string())
        .await?;

    let reply = ctx.receive_block::<String>().await?;
    println!("Reply via stream: {}", reply);

    ctx.stop().await
}
