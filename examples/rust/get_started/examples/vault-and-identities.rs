use ockam::node;
use ockam::{Context, Result};

#[ockam::node]
async fn main(ctx: Context) -> Result<()> {
    // Create default node to safely store secret keys for Alice
    let mut node = node(ctx);

    // Create an Identity to represent Alice.
    let _alice = node.create_identity().await?;

    // Stop the node.
    node.stop().await
}
