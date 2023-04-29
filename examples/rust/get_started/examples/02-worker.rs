// This node creates a worker, sends it a message, and receives a reply.

use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::{node, Context, Result};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let mut node = node(ctx);

    // Start a worker, of type Echoer, at address "echoer"
    node.start_worker("echoer", Echoer, AllowAll, AllowAll).await?;

    // Send a message to the worker at address "echoer".
    node.send("echoer", "Hello Ockam!".to_string()).await?;

    // Wait to receive a reply and print it.
    let reply = node.receive::<String>().await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    node.stop().await
}
