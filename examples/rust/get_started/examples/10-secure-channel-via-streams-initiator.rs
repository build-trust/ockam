use ockam::identity::{Identity, SecureChannelTrustOptions};
use ockam::{
    route, stream::Stream, vault::Vault, Context, MessageReceiveOptions, Result, TcpConnectionTrustOptions,
    TcpTransport,
};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let tcp = TcpTransport::create(&ctx).await?;

    // Set the address of the Kafka node you created here. (e.g. "192.0.2.1:4000")
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>";
    let node_in_hub = tcp
        .connect(hub_node_tcp_address, TcpConnectionTrustOptions::new())
        .await?;

    // Create a vault
    let vault = Vault::create();

    // Create an Identity
    let alice = Identity::create(&ctx, vault).await?;

    // Create a stream client
    let (sender, _receiver) = Stream::new(&ctx)
        .await?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id("secure-channel-over-stream-over-cloud-node-initiator")
        .connect(
            route![node_in_hub],         // route to hub
            "sc-initiator-to-responder", // outgoing stream
            "sc-responder-to-initiator", // incoming stream
        )
        .await?;

    // Create a secure channel
    let secure_channel = alice
        .create_secure_channel(
            route![
                sender.clone(),            // via the "sc-initiator-to-responder" stream
                "secure_channel_listener"  // to the "secure_channel_listener" listener
            ],
            SecureChannelTrustOptions::new(),
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
    let reply = ctx
        .receive_extended::<String>(MessageReceiveOptions::new().without_timeout())
        .await?;
    println!("Reply through secure channel via stream: {}", reply);

    ctx.stop().await
}
