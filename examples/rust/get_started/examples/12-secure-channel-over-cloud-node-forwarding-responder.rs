use ockam::{Context, Entity, Identity, RemoteForwarder, Result, SecureChannels, TcpTransport};
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
    let mut bob = Entity::create(&ctx)?;

    // Create a secure channel listener at address "bob_secure_channel_listener"
    bob.create_secure_channel_listener("bob_secure_channel_listener")?;

    let forwarder =
        RemoteForwarder::create(&ctx, cloud_node_tcp_address, "bob_secure_channel_listener")
            .await?;

    println!("Forwarding address: {}", forwarder.remote_address());

    Ok(())
}
