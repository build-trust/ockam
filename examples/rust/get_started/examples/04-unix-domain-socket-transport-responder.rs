// This node starts a uds listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::{node, Context, Result};
use ockam_transport_uds::UdsTransportExtension;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let node = node(ctx).await?;

    // Initialize the UDS Transport
    let uds = node.create_uds_transport().await?;

    // Create a Uds listener and wait for incoming connections.
    uds.listen("/tmp/ockam-example-echoer").await?;

    // Create an echoer worker
    node.start_worker("echoer", Echoer)?;

    // Don't call node.shutdown() here so this node runs forever.
    Ok(())
}
