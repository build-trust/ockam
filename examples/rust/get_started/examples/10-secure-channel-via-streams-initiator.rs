use ockam::{
    channel::SecureChannel, route, stream::Stream, try_route, vault::Vault, Context, Result, TcpTransport, TCP,
};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // Set the address of the Kafka node you created here. (e.g. "192.0.2.1:4000")
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>";

    // Create a vault
    let vault = Vault::create();

    // Create a stream client
    let (sender, _receiver) = Stream::new(&ctx)
        .await?
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
        try_route![
            sender.clone(),            // via the "sc-initiator-to-responder" stream
            "secure_channel_listener"  // to the "secure_channel_listener" listener
        ]?,
        &vault,
    )
    .await?;

    // Send a message
    ctx.send(
        try_route![
            secure_channel.address(), // via the secure channel
            "echoer"                  // to the "echoer" worker
        ]?,
        "Hello World!".to_string(),
    )
    .await?;

    // Receive a message from the "sc-responder-to-initiator" stream
    let reply = ctx.receive_block::<String>().await?;
    println!("Reply through secure channel via stream: {}", reply);

    ctx.stop().await
}
