use ockam_core::Result;
use ockam_node::Context;
use ockam_transport_udp::{RendezvousService, UdpBindArguments, UdpBindOptions, UdpTransport};
use tracing::info;

#[ockam_macros::node]
async fn main(ctx: Context) -> Result<()> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or(String::from("0.0.0.0:4000"));

    info!("Starting UDP Rendezvous service listening on {}", addr);

    RendezvousService::start(&ctx, "rendezvous").await?;

    let udp = UdpTransport::create(&ctx).await?;
    let bind = udp
        .bind(
            UdpBindArguments::new().with_bind_address(addr)?,
            UdpBindOptions::new(),
        )
        .await?;

    ctx.flow_controls()
        .add_consumer("rendezvous", bind.flow_control_id());

    // Don't stop context/node. Run forever.
    Ok(())
}
