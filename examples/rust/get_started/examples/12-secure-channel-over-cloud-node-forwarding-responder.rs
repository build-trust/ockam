use ockam::{
    Context, Entity, NoOpTrustPolicy, RemoteForwarder, Result, SecureChannels, TcpTransport, Vault,
};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a cloud node by going to https://hub.ockam.network
    let cloud_node_tcp_address = "Paste the tcp address of your cloud node here.";

    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    // Create an echoer worker
    ctx.start_worker("echoer", Echoer).await?;
    let vault = Vault::create(&ctx).expect("failed to create vault");
    let mut bob = Entity::create(&ctx, &vault)?;

    // Create a secure channel listener at address "bob_secure_channel_listener"
    bob.create_secure_channel_listener("bob_secure_channel_listener", NoOpTrustPolicy)?;

    let forwarder =
        RemoteForwarder::create(&ctx, cloud_node_tcp_address, "bob_secure_channel_listener")
            .await?;

    println!("Forwarding address: {}", forwarder.remote_address());

    Ok(())
}
