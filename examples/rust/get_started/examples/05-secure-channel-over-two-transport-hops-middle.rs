// This node creates a tcp connection to a node at 127.0.0.1:4000
// Starts a tcp listener at 127.0.0.1:3000
// It then runs forever waiting to route messages.

use hello_ockam::Hop;
use ockam::access_control::AllowAll;
use ockam::{Context, Result, TcpTransport};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    ctx.start_worker("hop", Hop, AllowAll, AllowAll).await?;

    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:3000").await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
