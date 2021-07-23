// This node creates an end-to-end encrypted secure channel over two tcp transport hops.
// It then routes a message, to a worker on a different node, through this encrypted channel.

use ockam::{
    route, Context, Entity, NoOpTrustPolicy, Result, SecureChannels, TcpTransport, Vault, TCP,
};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let _tcp = TcpTransport::create(&ctx).await?;

    let alice_vault = Vault::create(&ctx).expect("failed to create vault");
    let mut alice = Entity::create(&ctx, &alice_vault)?;
    let route = route![
        (TCP, "localhost:3000"),
        (TCP, "localhost:4000"),
        "bob_secure_channel_listener"
    ];

    // Connect to a secure channel listener and perform a handshake.
    let channel = alice.create_secure_channel(route, NoOpTrustPolicy)?;

    // Send a message to the echoer worker via the channel.
    let echoer_route = route![channel, "echoer"];

    ctx.send(echoer_route, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"
                                         // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
