// This node creates a secure channel with a listener that is multiple hops away.

use ockam::{Context, Result, Route, SecureChannel, Vault};
use ockam_get_started::{Echoer, Hop};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    ctx.start_worker("echoer", Echoer).await?;

    // Start 3 hop workers at addresses "h1", "h2" and "h3".
    ctx.start_worker("h1", Hop).await?;
    ctx.start_worker("h2", Hop).await?;
    ctx.start_worker("h3", Hop).await?;

    let vault = Vault::create(&ctx)?;

    // Create a secure channel listener at address "secure_channel_listener"
    SecureChannel::create_listener(&mut ctx, "secure_channel_listener", &vault).await?;

    // Connect to a secure channel listener and perform a handshake.
    let channel = SecureChannel::create(
        &mut ctx,
        // route to the secure channel listener, via "h1", "h2" and "h3"
        Route::new()
            .append("h1")
            .append("h2")
            .append("h3")
            .append("secure_channel_listener"),
        &vault,
    )
    .await?;

    // Send a message to the echoer worker, via the secure channel.
    ctx.send(
        // route to the "echoer" worker via the secure channel.
        Route::new().append(channel.address()).append("echoer"),
        // the message you want echo-ed back
        "Hello Ockam!".to_string(),
    )
    .await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
