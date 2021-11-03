use ockam::{Context, Result};

#[ockam_node_test_attribute::node]
async fn main(mut ctx: Context) -> Result<()> {
    ctx.stop().await
}
