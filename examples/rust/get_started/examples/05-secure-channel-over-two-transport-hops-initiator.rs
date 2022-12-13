// This node creates an end-to-end encrypted secure channel over two tcp transport hops.
// It then routes a message, to a worker on a different node, through this encrypted channel.

use ockam::authenticated_storage::InMemoryStorage;
use ockam::identity::{Identity, TrustEveryonePolicy};
use ockam::{route, vault::Vault, Context, Result, TcpTransport, TCP};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Alice.
    let vault = Vault::create();

    // Create an Identity to represent Alice.
    let alice = Identity::create(&ctx, &vault).await?;

    // Create an AuthenticatedStorage to store info about Alice's known Identities.
    let storage = InMemoryStorage::new();

    // Connect to a secure channel listener and perform a handshake.
    // Use ports 3000 & 4000, unless otherwise specified by command line arguments.
    let port_middle = std::env::args().nth(1).unwrap_or_else(|| "3000".to_string());
    let port_responder = std::env::args().nth(2).unwrap_or_else(|| "4000".to_string());
    let r = route![
        (TCP, format!("localhost:{port_middle}")),
        "hop",
        (TCP, format!("localhost:{port_responder}")),
        "bob_listener"
    ];
    let channel = alice.create_secure_channel(r, TrustEveryonePolicy, &storage).await?;

    // Send a message to the echoer worker via the channel.
    // Wait to receive a reply and print it.
    let reply: String = ctx
        .send_and_receive(route![channel, "echoer"], "Hello Ockam!".to_string())
        .await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
