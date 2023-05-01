// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::identity::SecureChannelListenerOptions;
use ockam::{node, Context, Result, TcpListenerOptions};
use ockam_core::flow_control::FlowControlPolicy;
use ockam_transport_tcp::TcpTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx);

    // Initialize the TCP Transport.
    let tcp = node.create_tcp_transport().await?;

    let tcp_listener_options = TcpListenerOptions::new();

    let sc_listener_options = SecureChannelListenerOptions::new().as_consumer(
        &tcp_listener_options.spawner_flow_control_id(),
        FlowControlPolicy::SpawnerAllowOnlyOneMessage,
    );

    node.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;
    node.flow_controls().add_consumer(
        "echoer",
        &sc_listener_options.spawner_flow_control_id(),
        FlowControlPolicy::SpawnerAllowMultipleMessages,
    );

    let bob = node.create_identity().await?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    node.create_secure_channel_listener(&bob, "bob_listener", sc_listener_options)
        .await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000", tcp_listener_options).await?;

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
