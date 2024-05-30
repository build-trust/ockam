use ockam_core::Result;
use ockam_node::workers::Echoer;
use ockam_node::Context;
use ockam_transport_udp::UdpTransport;

#[ockam_macros::node]
async fn main(ctx: Context) -> Result<()> {
    let udp = UdpTransport::create(&ctx).await?;
    udp.listen("127.0.0.1:8000").await?;
    ctx.start_worker("echoer", Echoer).await?;
    Ok(())
}
