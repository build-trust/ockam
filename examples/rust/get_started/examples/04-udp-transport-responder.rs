// This node starts an udp listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::udp::{UdpBindArguments, UdpBindOptions, UdpTransportExtension};
use ockam::{node, Context, Result};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx).await?;

    // Initialize the UDP Transport
    let udp = node.create_udp_transport().await?;

    // Create a UDP listener and wait for incoming datagrams.
    let bind = udp
        .bind(
            UdpBindArguments::new().with_bind_address("127.0.0.1:4000")?,
            UdpBindOptions::new(),
        )
        .await?;

    // Create an echoer worker
    node.start_worker("echoer", Echoer)?;

    node.flow_controls()
        .add_consumer(&"echoer".into(), bind.flow_control_id());

    // Don't call node.shutdown() here so this node runs forever.
    Ok(())
}
