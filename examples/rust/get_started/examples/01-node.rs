// This program creates and then immediately stops a node.

use ockam::{node, Context, Result};

/// Create and then immediately stop a node.
#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create a node.
    let mut node = node(ctx).await?;

    // Stop the node as soon as it starts.
    node.shutdown().await
}
