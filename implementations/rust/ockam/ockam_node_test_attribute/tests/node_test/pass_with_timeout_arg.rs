#[ockam_node_test_attribute::node_test(timeout = 1000)]
async fn my_test(mut ctx: &Context) -> ockam::Result<()> {
    ctx.stop().await.unwrap();
}
