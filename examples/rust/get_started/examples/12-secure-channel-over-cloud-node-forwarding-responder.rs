use ockam::{Context, LocalEntity, RemoteForwarder, Result, TcpTransport};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a cloud node by going to https://hub.ockam.network
    let cloud_node_tcp_address = "40.78.99.34:4000"; //"Paste the tcp address of your cloud node here.";

    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a TCP connection to your cloud node.
    tcp.connect(cloud_node_tcp_address).await?;

    // Create an echoer worker
    let mut local = LocalEntity::create_with_worker(&ctx, "echoer", Echoer).await?;

    // Create a secure channel listener at address "secure_channel_listener"
    local
        .create_secure_channel_listener("secure_channel_listener")
        .await?;

    let forwarder =
        RemoteForwarder::create(&ctx, cloud_node_tcp_address, "secure_channel_listener").await?;
    println!("Forwarding address: {}", forwarder.remote_address());

    Ok(())
}
