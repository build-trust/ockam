// This node creates a secure channel and routes a message through it.

use ockam::{
    route, Address, Context, Entity, IdentifierTrustPolicy, ProfileIdentity, Result, Route,
};
use ockam_get_started::Echoer;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    ctx.start_worker("echoer", Echoer).await?;

    let mut bob = Entity::create(&ctx).await?;

    // Connect to a secure channel listener and perform a handshake.
    let mut alice = Entity::create(&ctx).await?;

    // Bob defines a trust policy that only trusts Alice
    let bob_trust_policy = IdentifierTrustPolicy::new(alice.identifier()?);

    // Alice defines a trust policy that only trusts Bob
    let alice_trust_policy = IdentifierTrustPolicy::new(bob.identifier()?);

    // Create a secure channel listener.
    bob.create_secure_channel_listener("bob_secure_channel_listener", bob_trust_policy)
        .await?;

    let channel_to_bob = alice
        .create_secure_channel("bob_secure_channel_listener", alice_trust_policy)
        .await?;

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
