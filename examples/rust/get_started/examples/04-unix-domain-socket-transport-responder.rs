// This node starts a uds listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::{Context, Result};
use ockam_transport_uds::UdsTransport;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the Uds Transport.
    let uds = UdsTransport::create(&ctx).await?;

    // Create a Uds listener and wait for incoming connections.
    uds.listen("/tmp/ockam-example-echoer").await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
