/// This example uses the stream service to send messages between two
/// clients.  A stream is a buffered message sending channel, which
/// means that you can run `initiator` and `responder` in any order
/// you like.
use ockam::{route, stream::Stream, Context, Result, TcpTransport, TCP};
use std::time::Duration;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>"; // e.g. "127.0.0.1:4000"

    // Create the stream client
    let (sender, _receiver) = Stream::new(&ctx)?
        .stream_service("stream")
        .index_service("stream_index")
        .client_id("stream-over-cloud-node-initiator")
        .with_interval(Duration::from_millis(100))
        .connect(
            route![(TCP, hub_node_tcp_address)],
            "initiator-to-responder",
            "responder-to-initiator",
        )
        .await?;

    ctx.send(sender.to_route().append("echoer"), "Hello World!".to_string())
        .await?;

    let reply = ctx.receive_block::<String>().await?;
    println!("Reply via stream: {}", reply);

    ctx.stop().await
}
