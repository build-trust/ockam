#[ockam_node_test_attribute::node_test]
async fn my_test(ctx: &mut ockam::Context) -> ockam::Result<()> {
    ctx.stop().await
}
