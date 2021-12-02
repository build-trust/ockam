#[ockam::test]
async fn my_test(ctx: &mut ockam_node::Context) -> String {
    ctx.address();
}
