use ockam::{Context, Result};
use ockam_transport_tcp::TcpTransport;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    TcpTransport::create_listener(&ctx, "127.0.0.1:4000").await?;
    TcpTransport::create(&ctx, "127.0.0.1:6000").await?;

    // This node never shuts down.
    Ok(())
}
