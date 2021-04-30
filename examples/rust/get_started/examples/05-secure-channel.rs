// This node creates a secure channel and routes a message through it.

use ockam::{Context, Profile, Result, Route, Vault};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    ctx.start_worker("echoer", Echoer).await?;

    let vault = Vault::create(&ctx)?;

    let mut alice = Profile::create(&ctx, &vault)?;
    let mut bob = Profile::create(&ctx, &vault)?;

    // Create a secure channel listener.
    alice
        .create_secure_channel_listener(&ctx, "secure_channel_listener")
        .await?;

    // Connect to a secure channel listener and perform a handshake.
    let channel = bob
        .create_secure_channel(&ctx, "secure_channel_listener")
        .await?;

    // Send a message to the echoer worker, via the secure channel.
    ctx.send(
        // route to the "echoer" worker via the secure channel.
        Route::new().append(channel).append("echoer"),
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
