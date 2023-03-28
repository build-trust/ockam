use ockam_core::Result;
use ockam_node::Context;
use ockam_transport_udp::{UdpRendezvousService, UdpTransport};
use tracing::debug;

#[ockam_macros::node]
async fn main(ctx: Context) -> Result<()> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or(String::from("0.0.0.0:4000"));

    debug!("Starting UDP Rendezvous service listening on {}", addr);

    UdpRendezvousService::start(&ctx, "rendezvous").await?;

    let udp = UdpTransport::create(&ctx).await?;
    udp.listen(addr).await?;

    // Don't stop context/node. Run forever.
    Ok(())
}
