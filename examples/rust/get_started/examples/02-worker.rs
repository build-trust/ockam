// This node creates a worker, sends it a message, and receives a reply.

use hello_ockam::Echoer;
use ockam::access_control::AllowAll;
use ockam::{Context, Result};
use std::sync::Arc;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start a worker, of type Echoer, at address "echoer"
    ctx.start_worker("echoer", Echoer, Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;

    // Send a message to the worker at address "echoer". Wait to receive a reply and print it.
    let reply: String = ctx.send_and_receive("echoer", "Hello Ockam!".to_string()).await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
