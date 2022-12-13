// This node routes a message.

use hello_ockam::{Echoer, Hop};
use ockam::access_control::AllowAll;
use ockam::{route, Context, Result};
use std::sync::Arc;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start a worker, of type Echoer, at address "echoer"
    ctx.start_worker("echoer", Echoer, Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;

    // Start a worker, of type Hop, at address "h1"
    ctx.start_worker("h1", Hop, Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;

    // Send a message to the worker at address "echoer",
    // via the worker at address "h1"
    // Wait to receive a reply and print it.
    let reply: String = ctx
        .send_and_receive(route!["h1", "echoer"], "Hello Ockam!".to_string())
        .await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
