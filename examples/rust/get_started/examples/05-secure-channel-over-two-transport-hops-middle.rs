// This node creates a tcp connection to a node at 127.0.0.1:4000
// Starts a forwarder worker to forward messages to 127.0.0.1:4000
// Starts a tcp listener at 127.0.0.1:3000
// It then runs forever waiting to route messages.

use hello_ockam::Forwarder;
use ockam::access_control::AllowAll;
use ockam::{node, Context, Result, TcpConnectionOptions, TcpListenerOptions};
use ockam_core::flow_control::FlowControlPolicy;
use ockam_transport_tcp::TcpTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx);

    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport().await?;

    // Create a TCP connection to Bob.
    let connection_to_bob = tcp.connect("127.0.0.1:4000", TcpConnectionOptions::new()).await?;

    // Start a Forwarder to forward messages to Bob using the TCP connection.
    node.start_worker("forward_to_bob", Forwarder(connection_to_bob), AllowAll, AllowAll)
        .await?;

    let tcp_listener_options = TcpListenerOptions::new();
    node.flow_controls().add_consumer(
        "forward_to_bob",
        &tcp_listener_options.spawner_flow_control_id(),
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:3000", tcp_listener_options).await?;

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
