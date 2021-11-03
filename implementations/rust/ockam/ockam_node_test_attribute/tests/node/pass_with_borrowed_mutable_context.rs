#[ockam_node_test_attribute::node]
async fn main(ctx: &mut ockam::Context) -> ockam::Result<()> {
    ctx.stop().await
}
