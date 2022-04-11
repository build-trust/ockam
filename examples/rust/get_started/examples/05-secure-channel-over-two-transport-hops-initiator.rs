// This node creates an end-to-end encrypted secure channel over two tcp transport hops.
// It then routes a message, to a worker on a different node, through this encrypted channel.

use ockam::access_control::AllowedTransport;
use ockam::identity::{Identity, TrustEveryonePolicy};
use ockam::{route, vault::Vault, Address, Context, Mailboxes, Result, TcpTransport, TCP};
use std::sync::Arc;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Alice.
    let vault = Vault::create();

    // FIXME: Child context is needed because "app" Context has LocalOriginOnly AccessControl,
    // that can be fixed by improving #[ockam::node] macro
    let mut child_ctx = ctx
        .new_context_impl(Mailboxes::main(
            Address::random_local(),
            Arc::new(AllowedTransport::single(TCP)),
        ))
        .await?;

    // Create an Identity to represent Alice.
    let alice = Identity::create(&child_ctx, &vault).await?;

    // Connect to a secure channel listener and perform a handshake.
    let r = route![(TCP, "localhost:3000"), (TCP, "localhost:4000"), "bob_listener"];
    let channel = alice.create_secure_channel(r, TrustEveryonePolicy).await?;

    // Send a message to the echoer worker via the channel.
    child_ctx
        .send(route![channel, "echoer"], "Hello Ockam!".to_string())
        .await?;

    // Wait to receive a reply and print it.
    let reply = child_ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
