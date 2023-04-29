// This node starts an udp listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::{node, Context, Result};
use ockam_transport_udp::UdpTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx);

    // Initialize the UDP Transport
    let udp = node.create_udp_transport().await?;

    // Create a UDP listener and wait for incoming datagrams.
    udp.listen("127.0.0.1:4000").await?;

    // Create an echoer worker
    node.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
