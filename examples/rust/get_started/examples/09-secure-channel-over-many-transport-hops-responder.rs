// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.

use ockam::{Context, LocalEntity, Result, TcpTransport};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    let mut local = LocalEntity::create_with_worker(&ctx, "echoer", Echoer).await?;

    // Create a secure channel listener at address "secure_channel_listener"
    local
        .create_secure_channel_listener("secure_channel_listener")
        .await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
