// This node creates a tcp connection to a node at 127.0.0.1:4000
// Starts a forwarder worker to forward messages to 127.0.0.1:4000
// Starts a tcp listener at 127.0.0.1:3000
// It then runs forever waiting to route messages.

use hello_ockam::Forwarder;
use ockam::access_control::AllowAll;
use ockam::{node, Context, Result, TcpConnectionOptions, TcpListenerOptions};
use ockam_core::flow_control::{FlowControlPolicy, FlowControls};
use ockam_transport_tcp::TcpTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx);

    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport().await?;

    // Create a TCP connection to the responder node.
    let connection_to_responder = tcp.connect("127.0.0.1:4000", TcpConnectionOptions::new()).await?;

    // Create a Forwarder worker
    node.start_worker(
        "forward_to_responder",
        Forwarder(connection_to_responder),
        AllowAll,
        AllowAll,
    )
    .await?;

    // Allow access to the Forwarder via TCP connections from the TCP listener
    let tcp_flow_control_id = FlowControls::generate_id();
    node.flow_controls().add_consumer(
        "forward_to_responder",
        &tcp_flow_control_id,
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:3000", TcpListenerOptions::new(&tcp_flow_control_id))
        .await?;

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
