// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::identity::SecureChannelListenerOptions;
use ockam::{node, Context, Result, TcpListenerOptions};
use ockam_core::flow_control::{FlowControlPolicy, FlowControls};
use ockam_transport_tcp::TcpTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx);

    // Initialize the TCP Transport.
    let tcp = node.create_tcp_transport().await?;

    let tcp_flow_control_id = FlowControls::generate_id();
    let sc_flow_control_id = FlowControls::generate_id();

    node.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;
    node.flow_controls().add_consumer(
        "echoer",
        &sc_flow_control_id,
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );

    let bob = node.create_identity().await?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.

    node.create_secure_channel_listener(
        &bob,
        "bob_listener",
        SecureChannelListenerOptions::new(&sc_flow_control_id)
            .as_consumer(&tcp_flow_control_id, FlowControlPolicy::SpawnerAllowMultipleMessages),
    )
    .await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000", TcpListenerOptions::new(&tcp_flow_control_id))
        .await?;

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
