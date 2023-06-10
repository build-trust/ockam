// This node creates a tcp connection to a node at 127.0.0.1:4000
// Starts a forwarder worker to forward messages to 127.0.0.1:4000
// Starts a tcp listener at 127.0.0.1:3000
// It then runs forever waiting to route messages.

use hello_ockam::Forwarder;
use ockam::flow_control::FlowControlPolicy;
use ockam::{node, Context, Result, TcpConnectionOptions, TcpListenerOptions, TcpTransportExtension};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx);

    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport().await?;

    // Create a TCP connection to Bob.
    let connection_to_bob = tcp.connect("127.0.0.1:4000", TcpConnectionOptions::new()).await?;

    // Start a Forwarder to forward messages to Bob using the TCP connection.
    node.start_worker("forward_to_bob", Forwarder(connection_to_bob.into()))
        .await?;

    // Create a TCP listener and wait for incoming connections.
    let listener = tcp.listen("127.0.0.1:3000", TcpListenerOptions::new()).await?;

    node.flow_controls().add_consumer(
        "forward_to_bob",
        listener.flow_control_id(),
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
