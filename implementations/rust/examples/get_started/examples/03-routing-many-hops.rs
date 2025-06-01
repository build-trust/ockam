// This node routes a message through many hops.

use hello_ockam::{Echoer, Hop};
use ockam::{node, route, Context, Result};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let mut node = node(ctx).await?;

    // Start an Echoer worker at address "echoer"
    node.start_worker("echoer", Echoer)?;

    // Start 3 hop workers at addresses "h1", "h2" and "h3".
    node.start_worker("h1", Hop)?;
    node.start_worker("h2", Hop)?;
    node.start_worker("h3", Hop)?;

    // Send a message to the echoer worker via the "h1", "h2", and "h3" workers
    let r = route!["h1", "h2", "h3", "echoer"];
    node.send(r, "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = node.receive::<String>().await?;
    println!("App Received: {}", reply.into_body()?); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    node.shutdown().await
}
