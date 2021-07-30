/// This example uses the stream service to send messages between two
/// clients.  A stream is a buffered message sending channel, which
/// means that you can run `initiator` and `responder` in any order
/// you like.
use ockam::{route, stream::Stream, Context, Result, SecureChannel, TcpTransport, Vault, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // Set the address of the Kafka node you created here. (e.g. "192.0.2.1:4000")
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>";

    // Create a vault
    let vault = Vault::create(&ctx)?;

    // Create a stream client
    let (sender, _receiver) = Stream::new(&ctx)?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id("secure-channel-over-stream-over-cloud-node-initiator")
        .connect(
            route![(TCP, hub_node_tcp_address)], // route to hub
            "sc-initiator-to-responder",         // outgoing stream
            "sc-responder-to-initiator",         // incoming stream
        )
        .await?;

    // Create a secure channel
    let secure_channel = SecureChannel::create(
        &ctx,
        route![
            sender.clone(),            // via the "sc-initiator-to-responder" stream
            "secure_channel_listener"  // to the "secure_channel_listener" listener
        ],
        &vault,
    )
    .await?;

    // Send a message
    ctx.send(
        route![
            secure_channel.address(), // via the secure channel
            "echoer"                  // to the "echoer" worker
        ],
        "Hello World!".to_string(),
    )
    .await?;

    // Receive a message from the "sc-responder-to-initiator" stream
    let reply = ctx.receive_block::<String>().await?;
    println!("Reply through secure channel via stream: {}", reply);

    ctx.stop().await
}
