// This node starts an udp listener and an echoer worker.
// It then runs forever waiting for messages.

use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::{Context, Result};
use ockam_transport_udp::UdpTransport;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Initialize the UDP Transport.
    let udp = UdpTransport::create(&ctx).await?;

    // Create a UDP listener and wait for incoming datagrams.
    udp.listen("127.0.0.1:4000").await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Don't call ctx.stop() here so this node runs forever.
    Ok(())
}
