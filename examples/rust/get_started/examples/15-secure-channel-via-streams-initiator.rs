/// This example uses the stream service to send messages between two
/// clients.  A stream is a buffered message sending channel, which
/// means that you can run `initiator` and `responder` in any order
/// you like.
use ockam::{route, stream::Stream, Context, Result, SecureChannel, TcpTransport, Vault, TCP};
use std::time::Duration;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>"; // e.g. "127.0.0.1:4000"

    // Create a vault
    let vault = Vault::create(&ctx)?;

    // Create a bi-directional stream
    let (sender, _receiver) = Stream::new(&ctx)?
        .stream_service("stream")
        .index_service("stream_index")
        .client_id("secure-channel-over-stream-over-cloud-node-initiator")
        .connect(
            route![(TCP, hub_node_tcp_address)],
            // Stream name from THIS node to the OTHER node
            "sc-initiator-to-responder",
            // Stream name from the OTHER node to THIS node
            "sc-responder-to-initiator",
        )
        .await?;

    // Create a secure channel via the stream
    let channel = SecureChannel::create(
        &ctx,
        route![
            // Send via the stream
            sender.clone(),
            // And then to the secure_channel_listener
            "secure_channel_listener"
        ],
        &vault,
    )
    .await?;

    // Send a message via the channel to the echoer worker
    ctx.send(
        route![channel.address(), "echoer"],
        "Hello World!".to_string(),
    )
    .await?;

    // Wait for the reply
    let reply = ctx.receive_block::<String>().await?;
    println!("Reply through secure channel via stream: {}", reply);

    ctx.stop().await
}
