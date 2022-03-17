use ockam::{route, Context, Identity, Result, TrustEveryonePolicy, Vault};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Create a Vault to safely store secret keys for Alice and Bob.
    let vault = Vault::create();

    // Create an Identity to represent Bob.
    let bob = Identity::create(&ctx, &vault).await?;

    // Create a secure channel listener for Bob that will wait for requests to
    // initiate an Authenticated Key Exchange.
    bob.create_secure_channel_listener("bob", TrustEveryonePolicy).await?;

    // Create an entity to represent Alice.
    let alice = Identity::create(&ctx, &vault).await?;

    // As Alice, connect to Bob's secure channel listener and perform an
    // Authenticated Key Exchange to establish an encrypted secure channel with Bob.
    let channel = alice.create_secure_channel("bob", TrustEveryonePolicy).await?;

    // Send a message, ** THROUGH ** the secure channel,
    // to the "app" worker on the other side.
    //
    // This message will automatically get encrypted when it enters the channel
    // and decrypted just before it exits the channel.
    ctx.send(route![channel, "app"], "Hello Ockam!".to_string()).await?;

    // Wait to receive a message for the "app" worker and print it.
    let message = ctx.receive::<String>().await?;
    println!("App Received: {}", message); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
