// This program creates and then immediately stops a node.

use ockam::{node, Context, Result};
use tracing::info;

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node with default implementations
    let mut node = node(ctx);
    info!("hey");
    // Stop the node as soon as it starts.
    node.stop().await
}
