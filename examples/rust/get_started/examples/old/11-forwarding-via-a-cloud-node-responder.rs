use ockam::{Context, RemoteForwarder, Result, TcpTransport, TCP};
use hello_ockam::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a cloud node by going to https://hub.ockam.network
    let cloud_node_tcp_address = "Paste the tcp address of your cloud node here.";

    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    let forwarder = RemoteForwarder::create(&ctx, (TCP, cloud_node_tcp_address), "echoer").await?;
    println!(
        "Forwarding address of echoer: {}",
        forwarder.remote_address()
    );

    Ok(())
}
