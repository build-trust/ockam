use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::identity::SecureChannelListenerOptions;
use ockam::{node, route, Context, Result, TcpConnectionOptions};
use ockam_core::flow_control::FlowControls;
use ockam_transport_tcp::TcpTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx);
    let tcp = node.create_tcp_transport().await?;

    // Start an echoer worker
    node.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Set the address of the Kafka node you created here. (e.g. "192.0.2.1:4000")
    let hub_node_tcp_address = "<Your node Address copied from hub.ockam.network>";
    let node_in_hub = tcp.connect(hub_node_tcp_address, TcpConnectionOptions::new()).await?;

    // Create an Identity
    let bob = node.create_identity().await?;

    // Create a secure channel listener at address "secure_channel_listener"
    let sc_flow_control_id = FlowControls::generate_id();
    node.create_secure_channel_listener(
        &bob,
        "secure_channel_listener",
        SecureChannelListenerOptions::new(&sc_flow_control_id),
    )
    .await?;

    // Create a stream client
    node.create_stream()
        .await?
        .stream_service("stream_kafka")
        .index_service("stream_kafka_index")
        .client_id("secure-channel-over-stream-over-cloud-node-responder")
        .connect(
            route![node_in_hub],         // route to hub
            "sc-responder-to-initiator", // outgoing stream
            "sc-initiator-to-responder", // incoming stream
        )
        .await?;

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
