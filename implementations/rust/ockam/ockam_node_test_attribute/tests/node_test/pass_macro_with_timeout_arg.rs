#[ockam_node_test_attribute::node_test(timeout = 1000)]
async fn my_test(ctx: &mut ockam::Context) -> ockam::Result<()> {
    ctx.stop().await
}
