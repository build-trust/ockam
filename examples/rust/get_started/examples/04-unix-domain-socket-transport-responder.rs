// This node starts a uds listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::{access_control::AllowAll, node, Context, Result};
use ockam_transport_uds::UdsTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx);

    // Initialize the UDS Transport
    let uds = node.create_uds_transport().await?;

    // Create a Uds listener and wait for incoming connections.
    uds.listen("/tmp/ockam-example-echoer").await?;

    // Create an echoer worker
    node.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Don't call node.stop() here so this node runs forever.
    Ok(())
}
