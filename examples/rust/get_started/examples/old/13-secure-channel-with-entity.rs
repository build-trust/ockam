// This node creates a secure channel and routes a message through it.

use ockam::{
    route, Address, Context, Entity, Identity, Result, TrustIdentifierPolicy, Vault,
};
use hello_ockam::Echoer;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    ctx.start_worker("echoer", Echoer).await?;

    let bob_vault = Vault::create(&ctx).expect("failed to create vault");
    let mut bob = Entity::create(&ctx, &bob_vault).await?;

    // Connect to a secure channel listener and perform a handshake.
    let alice_vault = Vault::create(&ctx).expect("failed to create vault");
    let mut alice = Entity::create(&ctx, &alice_vault).await?;

    // Create a secure channel listener.
    let bob_trust_policy = TrustIdentifierPolicy::new(alice.identifier()?);
    bob.create_secure_channel_listener("bob_secure_channel_listener", bob_trust_policy).await?;

    let alice_trust_policy = TrustIdentifierPolicy::new(bob.identifier()?);
    let channel_to_bob =
        alice.create_secure_channel("bob_secure_channel_listener", alice_trust_policy).await?;

    let echoer: Address = "echoer".into();
    let route = route![channel_to_bob, echoer];

    // Send a message to the echoer worker, via the secure channel.
    ctx.send(route, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
