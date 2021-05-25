use ockam::{Context, RemoteForwarder, Result, TcpTransport};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a cloud node by going to https://hub.ockam.network
    let cloud_node_tcp_address = "Paste the tcp address of your cloud node here.";

    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection to your cloud node.
    tcp.connect(cloud_node_tcp_address).await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;

    let forwarder = RemoteForwarder::create(&ctx, cloud_node_tcp_address, "echoer").await?;
    println!(
        "Forwarding address of echoer: {}",
        forwarder.remote_address()
    );

    Ok(())
}
