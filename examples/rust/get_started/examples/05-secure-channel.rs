// This node creates a secure channel and routes a message through it.

use ockam::{Address, Context, LocalEntity, Result, Route, SecureChannel, Vault};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    let mut local = LocalEntity::create_with_worker(&ctx, "echoer", Echoer).await?;

    // Create a secure channel listener.
    local
        .create_secure_channel_listener("secure_channel_listener")
        .await?;

    // Connect to a secure channel listener and perform a handshake.
    let mut initiator = LocalEntity::create(&ctx, "initiator").await?;

    let channel = initiator
        .create_secure_channel("secure_channel_listener")
        .await?;

    let echoer: Address = "echoer".into();

    let route: Route = vec![channel, echoer].into();

    // Send a message to the echoer worker, via the secure channel.
    ctx.send(route, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
