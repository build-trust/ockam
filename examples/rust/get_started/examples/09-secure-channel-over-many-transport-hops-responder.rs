use ockam::{Context, Result, SecureChannel};
use ockam_get_started::Echoer;
use ockam_transport_tcp::TcpTransport;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    TcpTransport::create_listener(&ctx, "127.0.0.1:6000").await?;

    SecureChannel::create_listener(&mut ctx, "secure_channel_listener").await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    // This node never shuts down.
    Ok(())
}
