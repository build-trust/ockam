// This node routes a message.

use hello_ockam::{Echoer, Hop};
use ockam::{node, route, Context, Result};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let mut node = node(ctx).await?;

    // Start a worker, of type Echoer, at address "echoer"
    node.start_worker("echoer", Echoer)?;

    // Start a worker, of type Hop, at address "h1"
    node.start_worker("h1", Hop)?;

    // Send a message to the worker at address "echoer",
    // via the worker at address "h1"
    node.send(route!["h1", "echoer"], "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = node.receive::<String>().await?;
    println!("App Received: {}", reply.into_body()?); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    node.shutdown().await
}
