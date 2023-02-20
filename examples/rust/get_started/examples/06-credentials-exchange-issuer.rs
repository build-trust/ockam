use ockam::identity::credential_issuer::CredentialIssuer;
use ockam::{Context, TcpTransport};
use ockam_core::{AllowAll, Result};

/// This node starts a temporary credential issuer accessible via TCP on localhost:5000
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP listener and wait for incoming connections.
    tcp.listen("127.0.0.1:5000").await?;

    let issuer = CredentialIssuer::create(&ctx).await?;
    ctx.start_worker("issuer", issuer, AllowAll, AllowAll).await?;
    Ok(())
}
