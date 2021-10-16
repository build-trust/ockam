#[ockam_node_test_attribute::node_test]
async fn my_test(ctx: &Context) -> ockam::Result<()> {
    ctx.address();
}
