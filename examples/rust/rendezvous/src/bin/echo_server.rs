use ockam_core::Result;
use ockam_node::workers::Echoer;
use ockam_node::Context;
use ockam_transport_udp::{UdpBindArguments, UdpBindOptions, UdpTransport};

#[ockam_macros::node]
async fn main(ctx: Context) -> Result<()> {
    let udp = UdpTransport::create(&ctx).await?;
    let bind = udp
        .bind(
            UdpBindArguments::new().with_bind_address("127.0.0.1:8000")?,
            UdpBindOptions::new(),
        )
        .await?;

    ctx.start_worker("echoer", Echoer).await?;

    ctx.flow_controls()
        .add_consumer("echoer", bind.flow_control_id());

    Ok(())
}
