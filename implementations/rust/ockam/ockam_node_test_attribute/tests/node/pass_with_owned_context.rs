use ockam::Result;

#[ockam_node_test_attribute::node]
async fn main(ctx: ockam::Context) -> Result<()> {
    ctx.stop().await
}
