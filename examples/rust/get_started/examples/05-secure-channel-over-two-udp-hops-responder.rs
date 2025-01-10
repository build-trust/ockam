// This node starts a udp bind, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::identity::SecureChannelListenerOptions;
use ockam::{node, Context, Result};
use ockam_transport_udp::{UdpBindArguments, UdpBindOptions, UdpTransportExtension};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx).await?;

    // Initialize the UDP Transport.
    let udp = node.create_udp_transport().await?;

    node.start_worker("echoer", Echoer)?;

    let bob = node.create_identity().await?;

    let udp_bind = udp
        .bind(
            UdpBindArguments::new().with_bind_address("127.0.0.1:4000")?,
            UdpBindOptions::new(),
        )
        .await?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    let secure_channel_listener = node.create_secure_channel_listener(
        &bob,
        "bob_listener",
        SecureChannelListenerOptions::new().as_consumer(udp_bind.flow_control_id()),
    )?;

    // Allow access to the Echoer via Secure Channels
    node.flow_controls()
        .add_consumer(&"echoer".into(), secure_channel_listener.flow_control_id());

    // Don't call node.shutdown() here so this node runs forever.
    Ok(())
}
