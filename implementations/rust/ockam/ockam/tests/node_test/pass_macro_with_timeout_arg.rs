#[ockam_test_macros::node_test(timeout = 1000)]
async fn my_test(ctx: &mut ockam_node::Context) -> ockam_core::Result<()> {
    ctx.stop().await
}
