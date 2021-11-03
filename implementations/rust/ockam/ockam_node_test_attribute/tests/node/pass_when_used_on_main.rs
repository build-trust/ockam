// This test checks that an attribute macro #[ockam_node_test_attribute::node] exists
// and can be used with an async main function

use ockam::{Context, Result};

#[ockam_node_test_attribute::node]
async fn main(mut context: Context) -> Result<()> {
    context.stop().await
}
