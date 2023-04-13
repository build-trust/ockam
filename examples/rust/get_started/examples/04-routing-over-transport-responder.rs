// This node starts a tcp listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::{node, Context, Result, TcpListenerOptions};
use ockam_core::flow_control::{FlowControlPolicy, FlowControls};
use ockam_transport_tcp::TcpTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx);

    // Initialize the TCP Transport
    let tcp = node.create_tcp_transport().await?;

    // Create an echoer worker
    node.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Allow access to the Echoer via TCP connections from the TCP listener
    let tcp_flow_control_id = FlowControls::generate_id();
    node.flow_controls().add_consumer(
        "echoer",
        &tcp_flow_control_id,
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000", TcpListenerOptions::new(&tcp_flow_control_id))
        .await?;

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
