// This node creates an end-to-end encrypted secure channel over two tcp transport hops.
// It then routes a message, to a worker on a different node, through this encrypted channel.

use ockam::identity::{Identity, TrustEveryonePolicy};
use ockam::{route, vault::Vault, Context, Result, TcpConnectionTrustOptions, TcpTransport};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Initialize the TCP Transport.
    let tcp = TcpTransport::create(&ctx).await?;

    // Create a Vault to safely store secret keys for Alice.
    let vault = Vault::create();

    // Create an Identity to represent Alice.
    let alice = Identity::create(&ctx, vault).await?;

    // Create a TCP connection to the middle node.
    let connection_to_middle_node = tcp.connect("localhost:3000", TcpConnectionTrustOptions::new()).await?;

    // Connect to a secure channel listener and perform a handshake.
    let r = route![connection_to_middle_node, "forward_to_bob", "bob_listener"];
    let channel = alice.create_secure_channel(r, TrustEveryonePolicy).await?;

    // Send a message to the echoer worker via the channel.
    ctx.send(route![channel, "echoer"], "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = ctx.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
