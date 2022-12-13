// This node routes a message through many hops.

use hello_ockam::{Echoer, Hop};
use ockam::access_control::AllowAll;
use ockam::{route, Context, Result};
use std::sync::Arc;

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    // Start an Echoer worker at address "echoer"
    ctx.start_worker("echoer", Echoer, Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;

    // Start 3 hop workers at addresses "h1", "h2" and "h3".
    ctx.start_worker("h1", Hop, Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    ctx.start_worker("h2", Hop, Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;
    ctx.start_worker("h3", Hop, Arc::new(AllowAll), Arc::new(AllowAll))
        .await?;

    // Send a message to the echoer worker via the "h1", "h2", and "h3" workers
    // Wait to receive a reply and print it.
    let r = route!["h1", "h2", "h3", "echoer"];
    let reply: String = ctx.send_and_receive(r, "Hello Ockam!".to_string()).await?;
    println!("App Received: {}", reply); // should print "Hello Ockam!"

    // Stop all workers, stop the node, cleanup and return.
    ctx.stop().await
}
