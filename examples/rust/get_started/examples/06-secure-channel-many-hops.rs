// This node creates a secure channel with a listener that is multiple hops away.

use ockam::{Context, LocalEntity, Result, Route};
use ockam_get_started::{Echoer, Hop};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    let mut local = LocalEntity::create_with_worker(&ctx, "echoer", Echoer).await?;

    // Start 3 hop workers at addresses "h1", "h2" and "h3".
    ctx.start_worker("h1", Hop).await?;
    ctx.start_worker("h2", Hop).await?;
    ctx.start_worker("h3", Hop).await?;

    // Create a secure channel listener at address "secure_channel_listener"
    local
        .create_secure_channel_listener("secure_channel_listener")
        .await?;

    // Route to the secure channel listener, via "h1", "h2" and "h3"
    let route: Route = vec!["h1", "h2", "h3", "secure_channel_listener"].into();

    // Connect to the secure channel listener and perform a handshake.
    let mut initiator = LocalEntity::create(&ctx, "initiator").await?;

    let channel = initiator.create_secure_channel(route).await?;

    // Route to the "echoer" worker via the secure channel.
    let echoer_route: Route = vec![channel, "echoer".into()].into();

    // Send a message to the echoer worker, via the secure channel.
    ctx.send(
        echoer_route,
        // The message you want echo-ed back
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
