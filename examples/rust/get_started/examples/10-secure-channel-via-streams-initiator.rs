use ockam::identity::SecureChannelOptions;
use ockam::{node, route, Context, MessageReceiveOptions, Result, TcpConnectionOptions};
use ockam_transport_tcp::TcpTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let mut node = node(ctx);
    let tcp = node.create_tcp_transport().await?;

    // Set the address of the Kafka node you created here. (e.g. "192.0.2.1:4000")
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>";
    let node_in_hub = tcp.connect(hub_node_tcp_address, TcpConnectionOptions::new()).await?;

    // Create an Identity
    let alice = node.create_identity().await?;

    // Create a stream client
    let (sender, _receiver) = node
        .create_stream()
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
    let secure_channel = node
        .create_secure_channel(
            &alice,
            route![
                sender.clone(),            // via the "sc-initiator-to-responder" stream
                "secure_channel_listener"  // to the "secure_channel_listener" listener
            ],
            SecureChannelOptions::new(),
        )
        .await?
        .encryptor_address()
        .clone();

    // Send a message
    node.send(
        route![
            secure_channel.address(), // via the secure channel
            "echoer"                  // to the "echoer" worker
        ],
        "Hello World!".to_string(),
    )
    .await?;

    // Receive a message from the "sc-responder-to-initiator" stream
    let reply = node
        .receive_extended::<String>(MessageReceiveOptions::new().without_timeout())
        .await?;
    println!("Reply through secure channel via stream: {}", reply);

    node.stop().await
}
