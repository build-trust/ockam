use ockam::{Context, RemoteForwarder, Result};
use ockam_get_started::Echoer;
use ockam_transport_tcp::TcpTransport;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    ctx.start_worker("echo_service", Echoer {}).await?;

    let hub = "Paste the address of the node you created on Ockam Hub here.";

    let tcp = TcpTransport::create(&ctx).await?;
    tcp.connect(hub).await?;

    let mailbox = RemoteForwarder::create(&mut ctx, hub, "echo_service").await?;
    println!(
        "Forwarding address for echo_service: {}",
        mailbox.remote_address()
    );
    Ok(())
}
