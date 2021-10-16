#[ockam_node_test_attribute::node_test]
async fn my_test(ctx: &mut ockam::Context) {
    ctx.address();
}
