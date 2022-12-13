// This node starts an udp listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::{Context, Result};
use ockam_transport_udp::UdpTransport;
use std::sync::Arc;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the UDP Transport.
    let udp = UdpTransport::create(&ctx).await?;

    // Create a UDP listener and wait for incoming datagrams.
    // Use port 4000, unless otherwise specified by command line argument.
    let port = std::env::args().nth(1).unwrap_or_else(|| "4000".to_string());
    udp.listen(format!("127.0.0.1:{port}")).await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer, Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
