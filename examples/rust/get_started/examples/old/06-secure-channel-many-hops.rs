// This node creates a secure channel with a listener that is multiple hops away.

use ockam::{route, Context, Entity, Result, TrustEveryonePolicy, Vault};
use hello_ockam::{Echoer, Hop};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    ctx.start_worker("echoer", Echoer).await?;

    let bob_vault = Vault::create(&ctx).expect("failed to create vault");
    let mut bob = Entity::create(&ctx, &bob_vault).await?;

    // Start 3 hop workers at addresses "h1", "h2" and "h3".
    ctx.start_worker("h1", Hop).await?;
    ctx.start_worker("h2", Hop).await?;
    ctx.start_worker("h3", Hop).await?;

    // Create a secure channel listener at address "secure_channel_listener"
    bob.create_secure_channel_listener("secure_channel_listener", TrustEveryonePolicy).await?;

    // Route to the secure channel listener, via "h1", "h2" and "h3"
    let route = route!["h1", "h2", "h3", "secure_channel_listener"];

    // Connect to the secure channel listener and perform a handshake.
    let alice_vault = Vault::create(&ctx).expect("failed to create vault");
    let mut alice = Entity::create(&ctx, &alice_vault).await?;

    let channel_to_bob = alice.create_secure_channel(route, TrustEveryonePolicy).await?;

    // Route to the "echoer" worker via the secure channel.
    let echoer_route = route![channel_to_bob, "echoer"];

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
