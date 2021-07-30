use ockam::{
    route, Context, Entity, RemoteForwarder, Result, SecureChannels, TcpTransport,
    TrustEveryonePolicy, Vault, TCP,
};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Using a shared Hub Node.
    // You can create a personal node by going to https://hub.ockam.network
    let hub_node_tcp_address = "1.node.ockam.network:4000";

    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    let vault = Vault::create(&ctx).expect("failed to create vault");
    let mut bob = Entity::create(&ctx, &vault)?;

    // Create a secure channel listener at address "bob_secure_channel_listener"
    bob.create_secure_channel_listener("bob_secure_channel_listener", TrustEveryonePolicy)?;

    let forwarder = RemoteForwarder::create(
        &ctx,
        route![(TCP, hub_node_tcp_address)],
        "bob_secure_channel_listener",
    )
    .await?;

    println!("Forwarding address: {}", forwarder.remote_address());

    let message = ctx.receive_timeout::<String>(10000).await?;
    println!("Bob Received: {} from Alice via secure channel", message); // should print "Hello Ockam!"

    ctx.stop().await
}
