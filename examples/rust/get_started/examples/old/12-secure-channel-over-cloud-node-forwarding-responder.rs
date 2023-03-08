use ockam::{
    Context, Entity, RemoteForwarder, Result, TcpTransport, TrustEveryonePolicy,
    Vault, TCP,
};
use hello_ockam::Echoer;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a cloud node by going to https://hub.ockam.network
    let cloud_node_tcp_address = "Paste the tcp address of your cloud node here.";

    // Initialize the TCP Transport.
    let node = node(ctx);
    let _tcp = node.create_tcp_transport().await?;

    // Create an echoer worker
    node.start_worker("echoer", Echoer).await?;
    let bob = node.create_identity().await?;

    // Create a secure channel listener at address "bob_secure_channel_listener"
    node.create_secure_channel_listener(bob, "bob_secure_channel_listener", TrustEveryonePolicy).await?;

    let forwarder = node.create_forwarder(
        (TCP, cloud_node_tcp_address),
        "bob_secure_channel_listener",
    )
        .await?;

    println!("Forwarding address: {}", forwarder.remote_address());

    Ok(())
}
