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

    // Create a stream client
    let (sender, _receiver) = node
        .create_stream()
        .await?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id("stream-over-cloud-node-initiator")
        .connect(
            route![node_in_hub],      // route to hub
            "initiator-to-responder", // outgoing stream
            "responder-to-initiator", // incoming stream
        )
        .await?;

    // Send a message
    node.send(
        route![
            sender.clone(), // via the "initiator-to-responder" stream
            "echoer"        // to the "echoer" worker
        ],
        "Hello World!".to_string(),
    )
    .await?;

    // Receive a message from the "responder-to-initiator" stream
    let reply = node
        .receive_extended::<String>(MessageReceiveOptions::new().without_timeout())
        .await?;
    println!("Reply via stream: {}", reply);

    node.stop().await
}
