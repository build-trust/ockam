// This node creates an end-to-end encrypted secure channel over two tcp transport hops.
// It then routes a message, to a worker on a different node, through this encrypted channel.

use ockam::authenticated_storage::InMemoryStorage;
use ockam::identity::{Identity, SecureChannelRegistry, TrustEveryonePolicy};
use ockam::{route, vault::Vault, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Alice.
    let vault = Vault::create();

    // Create a SecureChannel registry.
    let registry = SecureChannelRegistry::new();

    // Create an Identity to represent Alice.
    let alice = Identity::create(&ctx, &vault).await?;

    // Create an AuthenticatedStorage to store info about Alice's known Identities.
    let storage = InMemoryStorage::new();

    // Connect to a secure channel listener and perform a handshake.
    let r = route![(TCP, "localhost:3000"), "hop", (TCP, "localhost:4000"), "bob_listener"];
    let channel = alice
        .create_secure_channel(r, TrustEveryonePolicy, &storage, &registry)
        .await?;

    // Send a message to the echoer worker via the channel.
    ctx.send(route![channel, "echoer"], "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
