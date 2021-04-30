// This node starts a tcp listener, a secure channel listener, and an echoer worker.
// It then runs forever waiting for messages.

use ockam::{Context, Profile, Result, TcpTransport, Vault};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:4000").await?;

    let vault = Vault::create(&ctx)?;

    let mut bob = Profile::create(&ctx, &vault)?;

    // Create a secure channel listener at address "secure_channel_listener"
    bob.create_secure_channel_listener(&ctx, "secure_channel_listener")
        .await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
