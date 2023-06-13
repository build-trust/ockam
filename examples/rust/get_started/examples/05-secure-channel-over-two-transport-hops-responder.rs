// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::flow_control::SpawnerFlowControlPolicy;
use ockam::identity::SecureChannelListenerOptions;
use ockam::{node, Context, Result, TcpListenerOptions, TcpTransportExtension};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx);

    // Initialize the TCP Transport.
    let tcp = node.create_tcp_transport().await?;

    node.start_worker("echoer", Echoer).await?;

    let bob = node.create_identity().await?;

    // Create a TCP listener and wait for incoming connections.
    let listener = tcp.listen("127.0.0.1:4000", TcpListenerOptions::new()).await?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    let secure_channel_listener = node
        .create_secure_channel_listener(
            &bob,
            "bob_listener",
            SecureChannelListenerOptions::new().as_consumer_for_spawner(
                listener.flow_control_id(),
                SpawnerFlowControlPolicy::AllowOnlyOneMessage,
            ),
        )
        .await?;

    // Allow access to the Echoer via Secure Channels
    node.flow_controls().add_consumer_for_spawner(
        "echoer",
        secure_channel_listener.flow_control_id(),
        SpawnerFlowControlPolicy::AllowMultipleMessages,
    );

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
