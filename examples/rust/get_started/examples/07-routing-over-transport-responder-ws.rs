// This node starts a ws listener and an echoer worker.
// It then runs forever waiting for messages.

use ockam::{Context, Result, WebSocketTransport};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the WS Transport.
    let ws = WebSocketTransport::create(&ctx).await?;

    // Create a WS listener and wait for incoming connections.
    ws.listen("127.0.0.1:4000").await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
