use ockam::{route, stream::Stream, Context, Result, SecureChannel, TcpTransport, Vault, TCP};
use ockam_get_started::Echoer;
use std::time::Duration;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    let _tcp = TcpTransport::create(&ctx).await?;

    // Create a vault
    let vault = Vault::create(&ctx)?;

    // Create a secure channel listener at address "secure_channel_listener"
    SecureChannel::create_listener(&ctx, "secure_channel_listener", &vault).await?;

    // Create a bi-directional stream
    Stream::new(&ctx)?
        .client_id("secure-channel-over-stream-over-cloud-node-responder")
        .with_interval(Duration::from_millis(100))
        .connect(
            route![(TCP, "localhost:4000")],
            // Stream name from THIS node to the OTHER node
            "test-b-a",
            // Stream name from OTHER to THIS
            "test-a-b",
        )
        .await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
